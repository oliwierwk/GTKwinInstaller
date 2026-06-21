#![cfg_attr(windows, windows_subsystem = "windows")]

#[path = "../config.rs"]
mod config;

fn main() {
    #[cfg(windows)]
    windows::run();
}

#[cfg(windows)]
mod windows {
    use super::config::APP_NAME;
    use std::path::PathBuf;

    #[cfg(bundled)]
    static BUNDLE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/bundle.zip"));

    fn tmp_dir() -> PathBuf {
        std::env::temp_dir().join(APP_NAME)
    }

    fn show_error(msg: &str) {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        use windows::core::PCWSTR;
        let title: Vec<u16> = OsStr::new(APP_NAME).encode_wide().chain(std::iter::once(0)).collect();
        let text: Vec<u16> = OsStr::new(msg).encode_wide().chain(std::iter::once(0)).collect();
        unsafe { let _ = MessageBoxW(None, PCWSTR(text.as_ptr()), PCWSTR(title.as_ptr()), MB_OK | MB_ICONERROR); }
    }

    fn is_locked(dir: &std::path::Path) -> bool {
        let exe = dir.join("installer.exe");
        exe.exists() && std::fs::OpenOptions::new().write(true).open(&exe).is_err()
    }

    pub fn run() {
        #[cfg(not(bundled))]
        {
            show_error(&format!("Not a bundled build — run 'make dist/gtkwininstaller-setup.exe' to create the installer."));
            return;
        }
        #[cfg(bundled)]
        {
            use std::io::Cursor;
            let tmp = tmp_dir();

            if tmp.exists() {
                if is_locked(&tmp) {
                    show_error(&format!("{APP_NAME} Installer is already running."));
                    return;
                }
                let _ = std::fs::remove_dir_all(&tmp);
            }

            std::fs::create_dir_all(&tmp).unwrap();
            let mut archive = zip::ZipArchive::new(Cursor::new(BUNDLE)).unwrap();
            archive.extract(&tmp).unwrap();

            let installer = tmp.join("installer.exe");
            let _ = std::process::Command::new(&installer)
                .current_dir(&tmp)
                .env("SETUP_EXE_PATH", std::env::current_exe().unwrap_or_default())
                .status();

            let _ = std::fs::remove_dir_all(&tmp);
        }
    }
}
