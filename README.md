# GTKwinInstaller

A Windows installer skeleton built with GTK4 / libadwaita and Rust. Produces a
self-extracting `setup.exe` that presents a native-looking install wizard, writes
registry uninstall entries, and places an `uninstaller.exe` in the install directory.

## Install modes

Both modes ship GTK in the archive (the installer wizard needs it). The difference
is what ends up in the user's install directory.

| Mode | When to use | What gets installed | Uninstaller |
|------|-------------|---------------------|-------------|
| **Non-bundled** (default) | Target app is a GTK app that ships its own runtime | App payload merged with GTK runtime — one set of DLLs | `installer.exe` → `uninstaller.exe` |
| **Bundled** (`BUNDLED_UNINSTALLER=1`) | Target app has no GTK runtime | App payload only — no GTK DLLs installed | `setup.exe` → `uninstaller.exe` (carries GTK internally for the uninstall wizard) |

## Requirements

MSYS2 with the UCRT64 toolchain. Verify and install missing packages with:

```bash
make check-ucrt64
```

## Standalone usage

Edit `config.rs` (name, publisher, app ID, flags) and drop your app's files into
`app/`, then:

```bash
make package-windows
```

This produces `dist/gtkwininstaller-windows.zip` and `dist/gtkwininstaller-setup.exe`.

## Use as a git submodule

Embed this repo as a submodule so your project gets installer updates without
forking. All configuration is injected from a parent-side file — no tracked files
inside the submodule need to be edited.

```bash
git submodule add https://github.com/oliwierwk/GTKwinInstaller installer
cp installer/installer.env.example installer.env
# edit installer.env, then:
make -C installer package-windows INSTALLER_ENV=../installer.env
```

## Configuration reference

All keys go in `installer.env` (submodule) or are set directly in `config.rs`
(standalone). Unset keys fall back to the `config.rs` defaults.

### Branding

| Key | Description |
|-----|-------------|
| `APP_NAME` | Display name shown in the wizard and registry |
| `APP_DESCRIPTION` | Subtitle on the welcome page; translatable via `PO_DIR` |
| `PUBLISHER` | Publisher name written to the registry uninstall entry |
| `APP_ID` | Reverse-DNS application ID (`com.example.myapp`) |

### Behaviour

| Key | Description |
|-----|-------------|
| `APP_EXE` | Filename of the user's app executable in the install directory. Used for shortcuts and the post-install launch button. Leave empty to skip both. |
| `BUNDLED_UNINSTALLER` | Set to `1` for bundled mode (see above). Default: `0`. |
| `LICENSE_FILE` | Path to the license file shown during installation, relative to the install root. Leave empty to hide the license page. |
| `APP_ICON_DARK` | Set to `1` to embed the dark-variant icon (`app-icon-dark.svg/png`). |

### Paths

| Key | Default | Description |
|-----|---------|-------------|
| `APP_DIR` | `app/` | Directory whose contents are staged as the app payload |
| `ASSETS_DIR` | `assets/` | Branding images (`app-icon`, `install-success`, `install-error` as SVG or PNG) |
| `LICENSE` | `app/LICENSE` | License file copied into the package |
| `PO_DIR` | _(empty)_ | Extra `.po` files; overrides built-in translations per-language and can add new ones |
| `APP_BUILD` | _(empty)_ | Shell command run before staging (e.g. `cargo build --release …`) |

## Translations

Built-in translations live in `po/`. To add or customise strings:

1. Run `make update-pot` (or `make -C installer update-pot`) to regenerate the
   `.pot` template from the installer source.
2. Create or update `.po` files in your `PO_DIR`.
3. `APP_DESCRIPTION` (if set) is passed through `gettext`, so add it as a `msgid`
   in your `.po` files to translate it.

To add a language not in the built-in set, create `<lang>.po` in `PO_DIR` and
set `PO_DIR` in `installer.env`.
