# Installs to ~/.local by default. Override with e.g. `make install PREFIX=/usr/local`
# (as root), and set DESTDIR for staged/package builds.
PREFIX ?= $(HOME)/.local
DESTDIR ?=

APP_ID := io.github.linkion.CharSheet
BIN := target/release/charsheet
MCP_BIN := target/release/charsheet-mcp

BINDIR := $(DESTDIR)$(PREFIX)/bin
APPDIR := $(DESTDIR)$(PREFIX)/share/applications
ICONDIR := $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps
METADIR := $(DESTDIR)$(PREFIX)/share/metainfo

SRD_PDF ?= SRD_CC_v5.2.1.pdf

.PHONY: all build install uninstall refresh-caches run test clean srd

all: build

build:
	cargo build --release -p charsheet -p charsheet-mcp

$(BIN): build

install: $(BIN)
	install -Dm755 $(BIN) $(BINDIR)/charsheet
	install -Dm755 $(MCP_BIN) $(BINDIR)/charsheet-mcp
	install -d $(APPDIR)
	sed 's|^Exec=charsheet|Exec=$(PREFIX)/bin/charsheet|' packaging/$(APP_ID).desktop > $(APPDIR)/$(APP_ID).desktop
	chmod 644 $(APPDIR)/$(APP_ID).desktop
	install -Dm644 packaging/$(APP_ID).svg $(ICONDIR)/$(APP_ID).svg
	install -Dm644 packaging/$(APP_ID).metainfo.xml $(METADIR)/$(APP_ID).metainfo.xml
	@$(MAKE) --no-print-directory refresh-caches
	@echo "Installed charsheet to $(BINDIR)"

uninstall:
	rm -f $(BINDIR)/charsheet $(BINDIR)/charsheet-mcp
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

# Regenerates data/srd from the SRD PDF (not in the repo; download it from
# https://www.dndbeyond.com/srd) and checks the result. Needs python3 and mutool.
srd:
	python3 -I tools/srd/build.py $(SRD_PDF) data/srd
	python3 -I tools/srd/check.py data/srd $(SRD_PDF)
