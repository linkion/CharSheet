# Installs to ~/.local by default. Override with e.g. `make install PREFIX=/usr/local`
# (as root), and set DESTDIR for staged/package builds.
PREFIX ?= $(HOME)/.local
DESTDIR ?=

APP_ID := io.github.porave.CharSheet
BIN := target/release/charsheet

BINDIR := $(DESTDIR)$(PREFIX)/bin
APPDIR := $(DESTDIR)$(PREFIX)/share/applications
ICONDIR := $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps
METADIR := $(DESTDIR)$(PREFIX)/share/metainfo

.PHONY: all build install uninstall refresh-caches run test clean

all: build

build:
	cargo build --release -p charsheet

$(BIN): build

install: $(BIN)
	install -Dm755 $(BIN) $(BINDIR)/charsheet
	install -d $(APPDIR)
	sed 's|^Exec=charsheet|Exec=$(PREFIX)/bin/charsheet|' packaging/$(APP_ID).desktop > $(APPDIR)/$(APP_ID).desktop
	chmod 644 $(APPDIR)/$(APP_ID).desktop
	install -Dm644 packaging/$(APP_ID).svg $(ICONDIR)/$(APP_ID).svg
	install -Dm644 packaging/$(APP_ID).metainfo.xml $(METADIR)/$(APP_ID).metainfo.xml
	@$(MAKE) --no-print-directory refresh-caches
	@echo "Installed charsheet to $(BINDIR)"

uninstall:
	rm -f $(BINDIR)/charsheet
	rm -f $(APPDIR)/$(APP_ID).desktop
	rm -f $(ICONDIR)/$(APP_ID).svg
	rm -f $(METADIR)/$(APP_ID).metainfo.xml
	@$(MAKE) --no-print-directory refresh-caches
	@echo "Removed charsheet (your characters in ~/.local/share/charsheet were kept)"

# Let the desktop pick up the new launcher and icon (skipped for staged installs).
refresh-caches:
	@if [ -z "$(DESTDIR)" ]; then \
		command -v update-desktop-database >/dev/null && update-desktop-database -q $(APPDIR) || true; \
		command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -qtf $(PREFIX)/share/icons/hicolor || true; \
		command -v kbuildsycoca6 >/dev/null && kbuildsycoca6 --noincremental >/dev/null 2>&1 || true; \
	fi

run:
	cargo run -p charsheet

test:
	cargo test --workspace

clean:
	cargo clean
