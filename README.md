# GTKwinInstaller

A Windows installer skeleton built with GTK4 / libadwaita and Rust.

## What's included

| Binary | Description |
|--------|-------------|
| `GTKwinInstaller.exe` | GTK4/Adwaita installer GUI (welcome → options → install/uninstall) |
| `setup.exe` | Self-extracting launcher — bundles `GTKwinInstaller.exe` + runtime into a single `.exe` |

## Standalone usage

Customize `config.rs` (app name, publisher, ID, flags), drop your app's files
into `app/` along with your `app/LICENSE`, then build:

```bash
# Verify dependencies
make check-ucrt64

# Build portable zip + self-extracting setup.exe
make package-windows
```

Requires MSYS2 with the UCRT64 toolchain.

## Use as a git submodule

Embed this repo as a submodule so your project gets installer updates without
forking. Configuration is injected from a parent-side file — no tracked files
inside the submodule need to be edited.

```bash
# In your project root
git submodule add https://github.com/youruser/GTKwinInstaller installer
cp installer/installer.env.example installer.env
```

Edit `installer.env` (lives in your repo, not in the submodule):

```ini
APP_NAME=My App
PUBLISHER=My Company
APP_ID=com.mycompany.myapp
APP_DIR=../dist          # your built app output
ASSETS_DIR=../branding   # your SVG/PNG assets
LICENSE=../dist/LICENSE  # your app's license shown during installation
```

Then build:

```bash
make -C installer package-windows INSTALLER_ENV=../installer.env
```

## Translations

PO files live in `po/`. To update the POT template after adding translatable
strings to the installer source:

```bash
make update-pot
```
