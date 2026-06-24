use crate::models::Video;
use std::collections::HashSet;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

pub(crate) struct SearchOutcome {
    pub(crate) seq: u64,
    pub(crate) result: Result<Vec<Video>, String>,
}

pub(crate) fn spawn_search(query: String, seq: u64, tx: Sender<SearchOutcome>) {
    thread::spawn(move || {
        let result = scrape_youtube_html(&query);
        let _ = tx.send(SearchOutcome { seq, result });
    });
}

fn scrape_youtube_html(query: &str) -> Result<Vec<Video>, String> {
    let safe_query = query.replace(' ', "+");
    let url = format!(
        "https://www.youtube.com/results?search_query={safe_query}&sp=EgIQAQ%253D%253D&hl=en&gl=US"
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Network error: {}", e))?;

    let body = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .map_err(|e| format!("Failed to connect: {}", e))?
        .text()
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let re = regex::Regex::new(r"(?s)var ytInitialData = (\{.*?\});").unwrap();
    let cap = re
        .captures(&body)
        .ok_or("Failed to extract ytInitialData (YouTube structure changed)")?;
    let json_str = cap.get(1).unwrap().as_str();

    let root: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut videos = Vec::new();
    let mut seen_ids = HashSet::new();

    fn extract_videos(
        node: &serde_json::Value,
        videos: &mut Vec<Video>,
        seen: &mut HashSet<String>,
    ) {
        if videos.len() >= 25 {
            return;
        }

        match node {
            serde_json::Value::Object(map) => {
                if let Some(vr) = map
                    .get("videoRenderer")
                    .or_else(|| map.get("compactVideoRenderer"))
                {
                    let id = vr.get("videoId").and_then(|v| v.as_str()).unwrap_or("");

                    if !id.is_empty() && !seen.contains(id) {
                        seen.insert(id.to_string());

                        let title = vr
                            .get("title")
                            .and_then(|t| t.get("runs"))
                            .and_then(|r| r.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|first| first.get("text"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("Unknown Title")
                            .to_string();

                        let uploader = vr
                            .get("longBylineText")
                            .or_else(|| vr.get("shortBylineText"))
                            .and_then(|t| t.get("runs"))
                            .and_then(|r| r.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|first| first.get("text"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("Unknown Channel")
                            .to_string();

                        let channel_id = vr
                            .get("longBylineText")
                            .or_else(|| vr.get("shortBylineText"))
                            .and_then(|t| t.get("runs"))
                            .and_then(|r| r.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|first| first.get("navigationEndpoint"))
                            .and_then(|n| n.get("browseEndpoint"))
                            .and_then(|b| b.get("browseId"))
                            .and_then(|id| id.as_str())
                            .map(|s| s.to_string());

                        let duration_str = vr
                            .get("lengthText")
                            .and_then(|l| l.get("simpleText"))
                            .and_then(|t| t.as_str());

                        let mut duration_secs = 0.0;
                        if let Some(d) = duration_str {
                            let parts: Vec<&str> = d.split(':').collect();
                            let mut mult = 1.0;
                            for p in parts.iter().rev() {
                                if let Ok(n) = p.parse::<f64>() {
                                    duration_secs += n * mult;
                                    mult *= 60.0;
                                }
                            }
                        }

                        let view_str = vr
                            .get("viewCountText")
                            .and_then(|v| v.get("simpleText"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("");

                        let views_digits: String =
                            view_str.chars().filter(|c| c.is_ascii_digit()).collect();
                        let view_count = views_digits.parse::<u64>().unwrap_or(0);

                        videos.push(Video {
                            id: id.to_string(),
                            title,
                            uploader,
                            duration_secs: Some(duration_secs),
                            view_count: Some(view_count),
                            channel_id,
                        });
                    }
                } else {
                    for (_, v) in map {
                        extract_videos(v, videos, seen);
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                for v in arr {
                    extract_videos(v, videos, seen);
                }
            }
            _ => {}
        }
    }

    extract_videos(&root, &mut videos, &mut seen_ids);

    if videos.is_empty() {
        Err("No videos found matching the query".to_string())
    } else {
        Ok(videos)
    }
}

pub(crate) struct SubVideosOutcome {
    pub(crate) channel_id: String,
    pub(crate) result: Result<Vec<Video>, String>,
}

fn decode_xml_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

pub(crate) fn fetch_channel_videos(channel_id: &str) -> Result<Vec<Video>, String> {
    let url = format!(
        "https://www.youtube.com/feeds/videos.xml?channel_id={}",
        channel_id
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| format!("Network error: {}", e))?;

    let body = client
        .get(&url)
        .send()
        .map_err(|e| format!("Failed to connect: {}", e))?
        .text()
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let entry_re = regex::Regex::new(r"(?s)<entry>(.*?)</entry>").unwrap();
    let id_re = regex::Regex::new(r"<yt:videoId>(.*?)</yt:videoId>").unwrap();
    let title_re = regex::Regex::new(r"<title>(.*?)</title>").unwrap();
    let author_re = regex::Regex::new(r"<name>(.*?)</name>").unwrap();

    let mut videos = Vec::new();
    for cap in entry_re.captures_iter(&body) {
        let entry_content = &cap[1];

        let id = id_re
            .captures(entry_content)
            .map(|c| c[1].to_string())
            .unwrap_or_default();

        let title = title_re
            .captures(entry_content)
            .map(|c| decode_xml_entities(&c[1]))
            .unwrap_or_default();

        let uploader = author_re
            .captures(entry_content)
            .map(|c| decode_xml_entities(&c[1]))
            .unwrap_or_default();

        if !id.is_empty() {
            videos.push(Video {
                id,
                title,
                uploader,
                duration_secs: None,
                view_count: None,
                channel_id: Some(channel_id.to_string()),
            });
        }
    }

    Ok(videos)
}
