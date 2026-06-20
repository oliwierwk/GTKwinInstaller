BINARY  := GTKwinInstaller
RELEASE := target/release

DIST_WIN  := dist/windows
ZIP       := dist/gtkwininstaller-windows.zip
SETUP_EXE := dist/gtkwininstaller-setup.exe

.PHONY: all package package-windows build check-rust check-ucrt64 clean update-pot

all: package

package: package-windows

# ─── Windows ──────────────────────────────────────────────────────────────────

check-rust:
	@command -v rustc >/dev/null 2>&1 \
	  || { echo "error: rustc not found — install Rust from https://rustup.rs or: pacman -S mingw-w64-ucrt-x86_64-rust"; exit 1; }
	@RUSTC_VV=$$(rustc -vV 2>/dev/null); \
	  echo "$$RUSTC_VV" | grep -q 'host:.*-gnu' \
	  || { echo "error: Rust GNU toolchain required — current host: $$(echo "$$RUSTC_VV" | grep host)"; \
	       echo "install: pacman -S mingw-w64-ucrt-x86_64-rust"; exit 1; }

check-ucrt64: check-rust
	@test -d /ucrt64 \
	  || { echo "error: /ucrt64 not found — run inside MSYS2 UCRT64 shell"; exit 1; }
	@for pkg in \
	    mingw-w64-ucrt-x86_64-gtk4 \
	    mingw-w64-ucrt-x86_64-libadwaita \
	    mingw-w64-ucrt-x86_64-librsvg \
	    mingw-w64-ucrt-x86_64-pkgconf \
	    mingw-w64-ucrt-x86_64-gcc \
	    mingw-w64-ucrt-x86_64-vulkan-loader \
	    mingw-w64-ucrt-x86_64-gettext-tools \
	    zip; do \
	  pacman -Q "$$pkg" >/dev/null 2>&1 \
	    || { echo "error: missing $$pkg — run: pacman -S $$pkg"; exit 1; }; \
	done

package-windows: check-ucrt64
	cargo build --release --bin $(BINARY)
	rm -rf $(DIST_WIN) && mkdir -p $(DIST_WIN)/share/glib-2.0

	# Copy installer binary
	cp $(RELEASE)/$(BINARY).exe $(DIST_WIN)/

	# Gather DLL dependencies
	ldd $(RELEASE)/$(BINARY).exe \
	  | grep -i ucrt64 \
	  | awk '{print $$3}' \
	  | xargs -I{} cp {} $(DIST_WIN)/

	# Vulkan, glib schemas, Adwaita icons
	cp /ucrt64/bin/vulkan-1.dll $(DIST_WIN)/
	cp -r /ucrt64/share/glib-2.0/schemas $(DIST_WIN)/share/glib-2.0/
	mkdir -p $(DIST_WIN)/share/icons
	cp -r /ucrt64/share/icons/Adwaita $(DIST_WIN)/share/icons/
	glib-compile-schemas $(DIST_WIN)/share/glib-2.0/schemas/

	# Installer assets (PNGs)
	cp -r $(RELEASE)/assets $(DIST_WIN)/

	# gdk-pixbuf PNG + SVG loaders and their deps
	mkdir -p $(DIST_WIN)/lib/gdk-pixbuf-2.0/2.10.0/loaders
	for loader in libpixbufloader-png.dll libpixbufloader-svg.dll; do \
	  cp /ucrt64/lib/gdk-pixbuf-2.0/2.10.0/loaders/$$loader \
	     $(DIST_WIN)/lib/gdk-pixbuf-2.0/2.10.0/loaders/; \
	  ldd /ucrt64/lib/gdk-pixbuf-2.0/2.10.0/loaders/$$loader \
	    | grep -i ucrt64 | awk '{print $$3}' | xargs -I{} cp -n {} $(DIST_WIN)/ 2>/dev/null || true; \
	done

	# ── Target app payload ────────────────────────────────────────────────────
	# Drop your app's files into app/ — only this directory is copied to the
	# install destination. The GTK runtime above is installer-only.
	mkdir -p $(DIST_WIN)/app
	# cp -r my-app.exe ... $(DIST_WIN)/app/
	# ─────────────────────────────────────────────────────────────────────────

	# License file shown to the user during installation (optional)
	# cp LICENSE $(DIST_WIN)/LICENSE

	# Translations
	for po in po/*.po; do \
	  lang=$$(basename "$$po" .po); \
	  mkdir -p $(DIST_WIN)/share/locale/$$lang/LC_MESSAGES; \
	  msgfmt -o $(DIST_WIN)/share/locale/$$lang/LC_MESSAGES/gtkwininstaller.mo "$$po"; \
	done

	# Pack portable ZIP
	rm -f $(ZIP)
	cd $(DIST_WIN) && zip -r ../../$(ZIP) .

$(SETUP_EXE): $(ZIP)
	BUNDLE_ZIP=$(ZIP) cargo build --release --bin setup
	cp $(RELEASE)/setup.exe $(SETUP_EXE)

# ─── Shared ───────────────────────────────────────────────────────────────────

update-pot:
	xgettext --language=C --keyword=gettext --from-code=UTF-8 \
	  --package-name=gtkwininstaller \
	  --package-version=$(shell grep '^version' Cargo.toml | head -1 | cut -d'"' -f2) \
	  -o po/gtkwininstaller.pot \
	  $$(cat po/POTFILES.in | grep -v '^#' | grep -v '^$$')

build:
	cargo build --release

clean:
	rm -rf dist/
