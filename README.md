# GTKwinInstaller

A Windows installer skeleton built with GTK4 / libadwaita and Rust.

## What's included

| Binary | Description |
|--------|-------------|
| `GTKwinInstaller.exe` | GTK4/Adwaita installer GUI (welcome → options → install/uninstall) |
| `setup.exe` | Self-extracting launcher — bundles `GTKwinInstaller.exe` + runtime into a single `.exe` |

## Building

Requires MSYS2 with the UCRT64 toolchain.

```bash
# Verify dependencies
make check-ucrt64

# Build portable zip (dist/gtkwininstaller-windows.zip)
make package-windows

# Build self-extracting setup.exe (dist/gtkwininstaller-setup.exe)
make dist/gtkwininstaller-setup.exe
```

## Translations

PO files live in `po/`. To update the POT template after adding translatable strings to the installer source:

```bash
make update-pot
```
