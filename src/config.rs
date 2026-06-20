#![allow(dead_code)] // items are only used inside #[cfg(windows)] blocks

// ─── Customize your installer here ──────────────────────────────────────────
pub const APP_NAME:  &str = "GTKwinInstaller"; // display/brand name (free-form)
pub const PUBLISHER: &str = "GTKwinInstaller"; // Add/Remove Programs publisher
pub const APP_ID:    &str = "com.gtkwininstaller.GTKwinInstaller";

/// License file shown to the user before installation. Path is relative to the
/// installer executable. Set to "" to disable the license page entirely.
pub const LICENSE_FILE: &str = "LICENSE";

/// When true, setup.exe is copied into the install directory as uninstall.exe.
/// It bundles its own GTK runtime, so the install directory doesn't need to
/// ship the GTK runtime separately. Use when the app you are installing is not
/// a GTK app and would not otherwise have the GTK runtime available.
pub const BUNDLED_UNINSTALLER: bool = false;

/// Filename of the installed binary — derives from the Cargo package name
/// so it always matches what cargo actually produces.
pub fn exe_name() -> String { format!("{}.exe", env!("CARGO_PKG_NAME")) }

/// Registry uninstall key — derived from APP_NAME so renames propagate.
pub fn reg_key() -> String {
    format!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{APP_NAME}")
}
// ────────────────────────────────────────────────────────────────────────────
