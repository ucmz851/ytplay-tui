use rand::seq::SliceRandom;
use std::time::Duration;

fn get_healthy_invidious_instance() -> Option<String> {
    let api_url = "https://api.invidious.io/instances.json";
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;

    let response = client
        .get(api_url)
        .send()
        .ok()?
        .json::<serde_json::Value>()
        .ok()?;
    let mut valid_instances = Vec::new();

    if let Some(instances) = response.as_array() {
        for instance_data in instances {
            if let Some(details) = instance_data.get(1) {
                let is_api = details
                    .get("api")
                    .and_then(|a| a.as_bool())
                    .unwrap_or(false);
                let is_https = details.get("type").and_then(|t| t.as_str()) == Some("https");

                if is_api && is_https {
                    #[allow(clippy::collapsible_if)]
                    if let Some(uri) = details.get("uri").and_then(|u| u.as_str()) {
                        valid_instances.push(uri.to_string());
                    }
                }
            }
        }
    }

    let mut rng = rand::thread_rng();
    valid_instances.choose(&mut rng).cloned()
}

pub(crate) fn get_direct_stream_url(video_id: &str) -> Option<String> {
    let base_url = get_healthy_invidious_instance()?;
    let api_url = format!("{}/api/v1/videos/{}", base_url, video_id);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .ok()?;

    let response = client
        .get(&api_url)
        .send()
        .ok()?
        .json::<serde_json::Value>()
        .ok()?;

    if let Some(first) = response
        .get("formatStreams")
        .and_then(|f| f.as_array())
        .and_then(|arr| arr.first())
    {
        return first.get("url").and_then(|u| u.as_str()).map(String::from);
    }
    None
}
