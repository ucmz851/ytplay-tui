# Makefile for ytplay-tui

PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share

.PHONY: all build install install-user uninstall uninstall-user clean

all: build

build:
	@echo "📦 Building ytplay-tui in release mode..."
	cargo build --release

install: build
	@echo "🚀 Installing binary to $(DESTDIR)$(BINDIR)..."
	install -Dm755 target/release/ytplay-tui "$(DESTDIR)$(BINDIR)/ytplay-tui"
	@echo "🖼️ Installing icon to $(DESTDIR)$(DATADIR)/icons/hicolor/scalable/apps..."
	install -Dm644 assets/icon.svg "$(DESTDIR)$(DATADIR)/icons/hicolor/scalable/apps/ytplay-tui.svg"
	@echo "🖥️ Installing desktop launcher to $(DESTDIR)$(DATADIR)/applications..."
	install -Dm644 assets/ytplay-tui.desktop "$(DESTDIR)$(DATADIR)/applications/ytplay-tui.desktop"
	@echo "🔄 Refreshing desktop database..."
	-which update-desktop-database >/dev/null 2>&1 && update-desktop-database "$(DESTDIR)$(DATADIR)/applications" || true
	@echo "✅ Installation complete!"

install-user: build
	@echo "🚀 Installing binary to $(HOME)/.local/bin..."
	install -Dm755 target/release/ytplay-tui "$(HOME)/.local/bin/ytplay-tui"
	@echo "🖼️ Installing icon to $(HOME)/.local/share/icons/hicolor/scalable/apps..."
	install -Dm644 assets/icon.svg "$(HOME)/.local/share/icons/hicolor/scalable/apps/ytplay-tui.svg"
	@echo "🖥️ Installing desktop launcher to $(HOME)/.local/share/applications..."
	install -Dm644 assets/ytplay-tui.desktop "$(HOME)/.local/share/applications/ytplay-tui.desktop"
	@echo "🔄 Refreshing desktop database..."
	-which update-desktop-database >/dev/null 2>&1 && update-desktop-database "$(HOME)/.local/share/applications" || true
	@echo "✅ User-space installation complete!"

uninstall:
	@echo "🧹 Removing system files..."
	rm -f "$(DESTDIR)$(BINDIR)/ytplay-tui"
	rm -f "$(DESTDIR)$(DATADIR)/icons/hicolor/scalable/apps/ytplay-tui.svg"
	rm -f "$(DESTDIR)$(DATADIR)/applications/ytplay-tui.desktop"
	@echo "🔄 Refreshing desktop database..."
	-which update-desktop-database >/dev/null 2>&1 && update-desktop-database "$(DESTDIR)$(DATADIR)/applications" || true
	@echo "✅ Uninstallation complete!"

uninstall-user:
	@echo "🧹 Removing user-space files..."
	rm -f "$(HOME)/.local/bin/ytplay-tui"
	rm -f "$(HOME)/.local/share/icons/hicolor/scalable/apps/ytplay-tui.svg"
	rm -f "$(HOME)/.local/share/applications/ytplay-tui.desktop"
	@echo "🔄 Refreshing desktop database..."
	-which update-desktop-database >/dev/null 2>&1 && update-desktop-database "$(HOME)/.local/share/applications" || true
	@echo "✅ User-space uninstallation complete!"

clean:
	@echo "🧹 Cleaning cargo build files..."
	cargo clean
