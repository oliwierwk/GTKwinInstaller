#![allow(dead_code)] // items are only used inside #[cfg(windows)] blocks

// ─── Customize your installer here ──────────────────────────────────────────
// Standalone: edit the None => "..." defaults below.
// Submodule:  set GTKWIN_* env vars in your installer.env — these files stay
//             pristine and git-clean inside the submodule.

pub const APP_NAME: &str = match option_env!("GTKWIN_APP_NAME") {
    Some(v) => v,
    None => "MyApp", // display/brand name (free-form)
};

/// One-line description shown as subtitle on the installer welcome page.
/// Not translated — provide it in whatever language you want displayed.
/// Leave empty ("") to show the default translatable description instead.
pub const APP_DESCRIPTION: &str = match option_env!("GTKWIN_APP_DESCRIPTION") {
    Some(v) => v,
    None => "",
};

pub const PUBLISHER: &str = match option_env!("GTKWIN_PUBLISHER") {
    Some(v) => v,
    None => "GTKwinInstaller", // Add/Remove Programs publisher
};

pub const APP_ID: &str = match option_env!("GTKWIN_APP_ID") {
    Some(v) => v,
    None => "com.gtkwininstaller.GTKwinInstaller",
};

/// License file shown to the user before installation. Path is relative to the
/// installer executable. Set to "" to disable the license page entirely.
pub const LICENSE_FILE: &str = match option_env!("GTKWIN_LICENSE_FILE") {
    Some(v) => v,
    None => "LICENSE",
};

/// Filename of the user's app executable in the install directory.
/// Used to create desktop/start-menu shortcuts and to launch the app after "Finish".
/// Leave empty ("") to skip shortcuts and auto-launch.
pub const APP_EXE: &str = match option_env!("GTKWIN_APP_EXE") {
    Some(v) => v,
    None => "MyApp.cmd",
};

/// When true, the dark variant of app-icon (app-icon-dark.svg/png) is used as
/// the embedded Windows application icon (.ico). Has no effect at runtime —
/// only changes the icon baked into the binary at build time.
/// In submodule mode, set GTKWIN_APP_ICON_DARK=1 in installer.env instead.
pub const APP_ICON_DARK: bool = true;

/// When true, setup.exe is copied into the install directory as uninstall.exe.
/// It bundles its own GTK runtime, so the install directory doesn't need to
/// ship the GTK runtime separately. Use when the app you are installing is not
/// a GTK app and would not otherwise have the GTK runtime available.
pub const BUNDLED_UNINSTALLER: bool = {
    match option_env!("GTKWIN_BUNDLED_UNINSTALLER") {
        None => false,
        Some(s) => {
            let b = s.as_bytes();
            (b.len() == 1 && b[0] == b'1')
                || (b.len() == 4
                    && b[0] == b't' && b[1] == b'r'
                    && b[2] == b'u' && b[3] == b'e')
        }
    }
};

/// Filename of the installed binary — derives from the Cargo package name
/// so it always matches what cargo actually produces.
pub fn exe_name() -> String { format!("{}.exe", env!("CARGO_PKG_NAME")) }

/// Registry uninstall key — derived from APP_NAME so renames propagate.
pub fn reg_key() -> String {
    format!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{APP_NAME}")
}
// ────────────────────────────────────────────────────────────────────────────
