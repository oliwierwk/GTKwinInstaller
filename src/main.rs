#![cfg_attr(windows, windows_subsystem = "windows")]

#[path = "../config.rs"]
mod config;

fn main() {
    #[cfg(windows)]
    windows::run();
}

#[cfg(windows)]
mod windows {
    use super::config::{APP_DESCRIPTION, APP_EXE, APP_ID, APP_NAME, BUNDLED_UNINSTALLER, LICENSE_FILE, PUBLISHER, reg_key};
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use winreg::enums::*;
    use winreg::RegKey;
    use gtk4::prelude::*;
    use gtk4::{
        ApplicationWindow, Box as GtkBox, Button, CheckButton, Entry, FileDialog,
        HeaderBar, Image, Label, Orientation, ScrolledWindow, Separator, Stack,
        StackTransitionType, AlertDialog, TextView, WrapMode, gio, glib, gdk,
    };
    use libadwaita as adw;
    use gettextrs::gettext;


    const CREATE_NO_WINDOW: u32 = 0x08000000;

    fn init_gettext(exe_dir: &Path) {
        use gettextrs::{bind_textdomain_codeset, bindtextdomain, setlocale, textdomain, LocaleCategory};
        setlocale(LocaleCategory::LcAll, "");
        let _ = bindtextdomain("gtkwininstaller", exe_dir.join("share/locale"));
        let _ = bind_textdomain_codeset("gtkwininstaller", "UTF-8");
        let _ = textdomain("gtkwininstaller");
    }

    fn is_uninstaller() -> bool {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.file_name().map(|n| n == std::ffi::OsStr::new("uninstaller.exe")))
            .unwrap_or(false)
    }

    fn setup_env(exe_dir: &Path) {
        unsafe {
            std::env::set_var(
                "GDK_PIXBUF_MODULEDIR",
                exe_dir.join("lib/gdk-pixbuf-2.0/2.10.0/loaders"),
            );
        }
    }

    fn default_install_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_NAME)
    }

    fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let dst_path = dst.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_dir(&entry.path(), &dst_path)?;
            } else {
                std::fs::copy(entry.path(), dst_path)?;
            }
        }
        Ok(())
    }

fn create_shortcut(target: &Path, lnk: &Path) {
        let t = target.to_string_lossy().replace('\'', "''");
        let l = lnk.to_string_lossy().replace('\'', "''");
        let script = format!(
            "$s=(New-Object -ComObject WScript.Shell).CreateShortcut('{l}');$s.TargetPath='{t}';$s.Save()"
        );
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }

    fn existing_install() -> Option<PathBuf> {
        let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey(reg_key()).ok()?;
        let loc: String = key.get_value("InstallLocation").ok()?;
        Some(PathBuf::from(loc))
    }

    fn write_registry(dest: &Path) {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok((key, _)) = hkcu.create_subkey(reg_key()) {
            let _ = key.set_value("DisplayName", &APP_NAME);
            let _ = key.set_value("Publisher", &PUBLISHER);
            let _ = key.set_value("InstallLocation", &dest.to_string_lossy().as_ref());
            let uninstall_exe = dest.join("uninstaller.exe");
            let _ = key.set_value("DisplayIcon",     &uninstall_exe.to_string_lossy().as_ref());
            let _ = key.set_value("UninstallString", &uninstall_exe.to_string_lossy().as_ref());
            let _ = key.set_value("NoModify", &1u32);
            let _ = key.set_value("NoRepair", &1u32);
        }
    }

    fn tracked_paths(install_dir: &Path) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        fn scan_dir(dir: &Path, out: &mut Vec<PathBuf>) {
            let Ok(entries) = std::fs::read_dir(dir) else { return };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() { scan_dir(&p, out); }
                else { out.push(p); }
            }
        }
        if install_dir.exists() { scan_dir(install_dir, &mut paths); }

        if let Some(d) = dirs::desktop_dir() {
            let p = d.join(format!("{APP_NAME}.lnk"));
            if p.exists() { paths.push(p); }
        }
        if let Some(d) = dirs::data_dir() {
            let p = d.join(format!("Microsoft\\Windows\\Start Menu\\Programs\\{APP_NAME}.lnk"));
            if p.exists() { paths.push(p); }
        }
        paths
    }

    fn locked_by_other_process(install_dir: &Path) -> bool {
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::System::RestartManager::*;
        use windows::core::{PCWSTR, PWSTR};

        if !install_dir.exists() { return false; }

        let all_files: Vec<Vec<u16>> = tracked_paths(install_dir)
            .iter()
            .map(|p| {
                let mut w: Vec<u16> = p.as_os_str().encode_wide().collect();
                w.push(0);
                w
            })
            .collect();
        if all_files.is_empty() { return false; }

        let own_pid = std::process::id();

        (|| -> windows::core::Result<bool> {
            let mut session = 0u32;
            let mut key = [0u16; 33];
            unsafe { RmStartSession(&mut session, Some(0), PWSTR(key.as_mut_ptr())).ok()? };

            struct RmSession(u32);
            impl Drop for RmSession {
                fn drop(&mut self) { unsafe { let _ = RmEndSession(self.0); } }
            }
            let _guard = RmSession(session);

            let ptrs: Vec<PCWSTR> = all_files.iter().map(|f| PCWSTR(f.as_ptr())).collect();
            unsafe { RmRegisterResources(session, Some(&ptrs), None, None).ok()? };

            let mut needed = 0u32;
            let mut actual = 0u32;
            let mut reboot = 0u32;
            let _ = unsafe { RmGetList(session, &mut needed, &mut actual, None, &mut reboot) };
            if needed == 0 { return Ok(false); }

            let mut infos = vec![RM_PROCESS_INFO::default(); needed as usize];
            actual = needed;
            unsafe { RmGetList(session, &mut needed, &mut actual, Some(infos.as_mut_ptr()), &mut reboot).ok()? };

            Ok(infos[..actual as usize].iter().any(|p| p.Process.dwProcessId != own_pid))
        })().unwrap_or(false)
    }

    fn temp_installer_dir() -> PathBuf {
        std::env::temp_dir().join(APP_NAME)
    }

    fn show_native_error(msg: &str) {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        use windows::core::PCWSTR;
        let title: Vec<u16> = OsStr::new(APP_NAME).encode_wide().chain(std::iter::once(0)).collect();
        let text: Vec<u16> = OsStr::new(msg).encode_wide().chain(std::iter::once(0)).collect();
        unsafe { let _ = MessageBoxW(None, PCWSTR(text.as_ptr()), PCWSTR(title.as_ptr()), MB_OK | MB_ICONERROR); }
    }

    fn do_uninstall(install_dir: &Path) -> std::io::Result<()> {
        if let Some(d) = dirs::desktop_dir() {
            let _ = std::fs::remove_file(d.join(format!("{APP_NAME}.lnk")));
        }
        if let Some(data) = dirs::data_dir() {
            let _ = std::fs::remove_file(
                data.join(format!("Microsoft\\Windows\\Start Menu\\Programs\\{APP_NAME}.lnk")),
            );
        }
        let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(reg_key());
        let _ = std::env::set_current_dir(std::env::temp_dir());
        std::fs::remove_dir_all(install_dir)
    }

    fn do_install(src: &Path, dest: &Path, desktop: bool, startmenu: bool) -> std::io::Result<()> {
        if BUNDLED_UNINSTALLER {
            copy_dir(&src.join("app"), dest)?;
            if let Ok(setup_path) = std::env::var("SETUP_EXE_PATH") {
                let _ = std::fs::copy(setup_path, dest.join("uninstaller.exe"));
            }
        } else {
            // Non-bundled: the Makefile merges the app payload into the archive root
            // alongside the installer's GTK runtime (cp -rT), so there is only one set
            // of DLLs. Copy everything flat; installer.exe becomes uninstaller.exe.
            for entry in std::fs::read_dir(src)? {
                let entry = entry?;
                let fname = entry.file_name();
                let dst = dest.join(&fname);
                if entry.file_type()?.is_dir() {
                    copy_dir(&entry.path(), &dst)?;
                } else {
                    if fname == std::ffi::OsStr::new("installer.exe") { continue; }
                    std::fs::copy(entry.path(), dst)?;
                }
            }
            std::fs::copy(src.join("installer.exe"), dest.join("uninstaller.exe"))?;
        }
        write_registry(dest);
        if !APP_EXE.is_empty() {
            let exe = dest.join(APP_EXE);
            if desktop {
                if let Some(d) = dirs::desktop_dir() {
                    create_shortcut(&exe, &d.join(format!("{APP_NAME}.lnk")));
                }
            }
            if startmenu {
                if let Some(data) = dirs::data_dir() {
                    let programs = data.join("Microsoft\\Windows\\Start Menu\\Programs");
                    let _ = std::fs::create_dir_all(&programs);
                    create_shortcut(&exe, &programs.join(format!("{APP_NAME}.lnk")));
                }
            }
        }
        Ok(())
    }

    fn asset_path(assets: &std::path::Path, stem: &str, dark: bool) -> std::path::PathBuf {
        let candidates: &[&str] = if dark { &["-dark.svg", "-dark.png", ".svg", ".png"] }
                                  else     { &[".svg", ".png"] };
        candidates.iter()
            .map(|s| assets.join(format!("{stem}{s}")))
            .find(|p| p.exists())
            .unwrap_or_else(|| assets.join(format!("{stem}.svg")))
    }

    #[cfg(all(feature = "svg", has_svg_assets))]
    fn svg_to_texture(path: &std::path::Path, size: i32) -> Option<gdk::Texture> {
        let data = std::fs::read(path).ok()?;
        let tree = resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default()).ok()?;
        let size = size.max(1) as u32;
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)?;
        let ts = tree.size();
        let sx = size as f32 / ts.width();
        let sy = size as f32 / ts.height();
        resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(sx, sy), &mut pixmap.as_mut());
        let bytes = glib::Bytes::from(pixmap.data());
        Some(gdk::MemoryTexture::new(size as i32, size as i32, gdk::MemoryFormat::R8g8b8a8Premultiplied, &bytes, (size * 4) as usize).upcast())
    }

    fn set_asset_image(image: &Image, assets: &std::path::Path, stem: &str, dark: bool, scale: i32) {
        let path = asset_path(assets, stem, dark);
        let size = 96 * scale.max(1);
        #[cfg(all(feature = "svg", has_svg_assets))]
        if path.extension().and_then(|e| e.to_str()) == Some("svg") {
            if let Some(tex) = svg_to_texture(&path, size) {
                image.set_paintable(Some(&tex));
                return;
            }
        }
        image.set_from_file(Some(&path));
    }

    fn load_asset_image(assets: &std::path::Path, stem: &str, dark: bool) -> Image {
        let image = Image::new();
        set_asset_image(&image, assets, stem, dark, 1);
        image
    }

    fn build_ui(app: &adw::Application) {
        let src = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
        let assets = src.join("assets");
        let style_manager = adw::StyleManager::default();

        let license_text: Option<String> = if !LICENSE_FILE.is_empty() {
            std::fs::read_to_string(src.join(LICENSE_FILE)).ok().filter(|s| !s.is_empty())
        } else {
            None
        };
        let has_license = license_text.is_some();

        let existing = existing_install();
        let uninstaller = is_uninstaller();

        let title = if uninstaller {
            format!("{APP_NAME} Uninstaller")
        } else {
            format!("{APP_NAME} Installer")
        };
        let window = ApplicationWindow::builder()
            .application(app)
            .title(&title)
            .default_width(520)
            .default_height(400)
            .resizable(false)
            .build();

        let header = HeaderBar::new();
        window.set_titlebar(Some(&header));

        let stack = Stack::new();
        stack.set_transition_duration(250);

        // ── Welcome ──────────────────────────────────────────────────────────
        let welcome = GtkBox::new(Orientation::Vertical, 0);
        welcome.set_margin_top(30);
        welcome.set_margin_bottom(20);
        welcome.set_margin_start(30);
        welcome.set_margin_end(30);

        let wc = GtkBox::new(Orientation::Vertical, 12);
        wc.set_vexpand(true);
        wc.set_valign(gtk4::Align::Center);
        wc.set_halign(gtk4::Align::Center);

        let app_icon = load_asset_image(&assets, "app-icon", style_manager.is_dark());
        app_icon.set_pixel_size(96);
        wc.append(&app_icon);

        let title = Label::new(Some(APP_NAME));
        title.add_css_class("title-1");
        wc.append(&title);

        let subtitle_str = if APP_DESCRIPTION.is_empty() {
            gettext("A GTK-based Windows installer")
        } else {
            gettext(APP_DESCRIPTION)
        };
        let subtitle = Label::new(Some(&subtitle_str));
        subtitle.add_css_class("dim-label");
        wc.append(&subtitle);

        welcome.append(&wc);

        let wb = GtkBox::new(Orientation::Horizontal, 0);
        let next_btn = Button::builder()
            .label(gettext("Next"))
            .css_classes(["suggested-action"])
            .halign(gtk4::Align::End)
            .hexpand(true)
            .build();
        wb.append(&next_btn);
        welcome.append(&wb);

        stack.add_named(&welcome, Some("welcome"));

        // ── License ──────────────────────────────────────────────────────────
        if let Some(ref text) = license_text {
            let license_page = GtkBox::new(Orientation::Vertical, 12);
            license_page.set_margin_top(20);
            license_page.set_margin_bottom(20);
            license_page.set_margin_start(30);
            license_page.set_margin_end(30);

            let license_title = Label::builder()
                .label(gettext("License Agreement"))
                .halign(gtk4::Align::Start)
                .build();
            license_title.add_css_class("title-4");
            license_page.append(&license_title);

            let tv = TextView::builder()
                .editable(false)
                .wrap_mode(WrapMode::Word)
                .vexpand(true)
                .left_margin(8)
                .right_margin(8)
                .top_margin(8)
                .bottom_margin(8)
                .build();
            tv.buffer().set_text(text);

            let sw = ScrolledWindow::builder()
                .child(&tv)
                .vexpand(true)
                .build();
            license_page.append(&sw);

            let accept_cb = CheckButton::builder()
                .label(gettext("I have read and accept the license agreement"))
                .build();
            license_page.append(&accept_cb);

            let lb = GtkBox::new(Orientation::Horizontal, 8);
            lb.set_margin_top(4);
            let lic_prev_btn = Button::with_label(&gettext("Previous"));
            let lic_spacer = Label::new(None);
            lic_spacer.set_hexpand(true);
            let lic_next_btn = Button::builder()
                .label(gettext("Next"))
                .css_classes(["suggested-action"])
                .sensitive(false)
                .build();
            lb.append(&lic_prev_btn);
            lb.append(&lic_spacer);
            lb.append(&lic_next_btn);
            license_page.append(&lb);

            stack.add_named(&license_page, Some("license"));

            // Accept checkbox gates the Next button
            {
                let lic_next_btn = lic_next_btn.clone();
                accept_cb.connect_toggled(move |cb| {
                    lic_next_btn.set_sensitive(cb.is_active());
                });
            }
            // License Previous → welcome
            {
                let stack = stack.clone();
                lic_prev_btn.connect_clicked(move |_| {
                    stack.set_transition_type(StackTransitionType::SlideRight);
                    stack.set_visible_child_name("welcome");
                });
            }
            // License Next → options
            {
                let stack = stack.clone();
                lic_next_btn.connect_clicked(move |_| {
                    stack.set_transition_type(StackTransitionType::SlideLeft);
                    stack.set_visible_child_name("options");
                });
            }
        }

        // ── Options ──────────────────────────────────────────────────────────
        let options = GtkBox::new(Orientation::Vertical, 0);
        options.set_margin_top(20);
        options.set_margin_bottom(20);
        options.set_margin_start(30);
        options.set_margin_end(30);

        let oc = GtkBox::new(Orientation::Vertical, 12);
        oc.set_vexpand(true);
        oc.set_valign(gtk4::Align::Center);

        let path_label = Label::builder()
            .label(gettext("Install location:"))
            .halign(gtk4::Align::Start)
            .build();
        oc.append(&path_label);

        let path_row = GtkBox::new(Orientation::Horizontal, 8);
        let entry = Entry::builder()
            .text(default_install_path().to_string_lossy().as_ref())
            .hexpand(true)
            .build();
        let browse_btn = Button::with_label(&gettext("Browse..."));
        path_row.append(&entry);
        path_row.append(&browse_btn);
        oc.append(&path_row);

        oc.append(&Separator::new(Orientation::Horizontal));

        let desktop_cb = CheckButton::builder()
            .label(gettext("Create desktop shortcut"))
            .active(true)
            .build();
        let startmenu_cb = CheckButton::builder()
            .label(gettext("Create Start Menu shortcut"))
            .active(true)
            .build();
        oc.append(&desktop_cb);
        oc.append(&startmenu_cb);

        options.append(&oc);

        let ob = GtkBox::new(Orientation::Horizontal, 8);
        ob.set_margin_top(8);
        let prev_btn = Button::with_label(&gettext("Previous"));
        let spacer = Label::new(None);
        spacer.set_hexpand(true);
        let install_btn = Button::builder()
            .label(gettext("Install"))
            .css_classes(["suggested-action"])
            .build();
        ob.append(&prev_btn);
        ob.append(&spacer);
        ob.append(&install_btn);
        options.append(&ob);

        stack.add_named(&options, Some("options"));

        // ── Complete ─────────────────────────────────────────────────────────
        let complete = GtkBox::new(Orientation::Vertical, 0);
        complete.set_margin_top(30);
        complete.set_margin_bottom(30);
        complete.set_margin_start(30);
        complete.set_margin_end(30);

        let cc = GtkBox::new(Orientation::Vertical, 16);
        cc.set_vexpand(true);
        cc.set_valign(gtk4::Align::Center);
        cc.set_halign(gtk4::Align::Center);

        let check_icon = load_asset_image(&assets, "install-success", style_manager.is_dark());
        check_icon.set_pixel_size(96);
        cc.append(&check_icon);

        let done_label = Label::new(Some(&gettext("Installation complete!")));
        done_label.add_css_class("title-1");
        cc.append(&done_label);

        complete.append(&cc);

        let cb = GtkBox::new(Orientation::Horizontal, 0);
        let cs1 = Label::new(None);
        cs1.set_hexpand(true);
        let finish_btn = Button::builder()
            .label(gettext("Finish"))
            .css_classes(["suggested-action", "pill"])
            .width_request(120)
            .build();
        let cs2 = Label::new(None);
        cs2.set_hexpand(true);
        cb.append(&cs1);
        cb.append(&finish_btn);
        cb.append(&cs2);
        complete.append(&cb);

        stack.add_named(&complete, Some("complete"));

        // ── Failed ───────────────────────────────────────────────────────────
        let failed_page = GtkBox::new(Orientation::Vertical, 0);
        failed_page.set_margin_top(30);
        failed_page.set_margin_bottom(30);
        failed_page.set_margin_start(30);
        failed_page.set_margin_end(30);

        let efc = GtkBox::new(Orientation::Vertical, 16);
        efc.set_vexpand(true);
        efc.set_valign(gtk4::Align::Center);
        efc.set_halign(gtk4::Align::Center);

        let error_icon = load_asset_image(&assets, "install-error", style_manager.is_dark());
        error_icon.set_pixel_size(96);
        efc.append(&error_icon);

        let failed_title = Label::new(Some(&gettext("Installation failed")));
        failed_title.add_css_class("title-1");
        efc.append(&failed_title);

        let failed_detail = Label::new(None);
        failed_detail.add_css_class("dim-label");
        failed_detail.set_wrap(true);
        efc.append(&failed_detail);

        failed_page.append(&efc);

        let efb = GtkBox::new(Orientation::Horizontal, 0);
        efb.set_margin_top(8);
        let retry_btn = Button::builder()
            .label(gettext("Try again"))
            .css_classes(["suggested-action"])
            .halign(gtk4::Align::End)
            .hexpand(true)
            .build();
        efb.append(&retry_btn);
        failed_page.append(&efb);

        stack.add_named(&failed_page, Some("failed"));

        // Update images when the system theme changes.
        // Reload images at the correct scale whenever theme or monitor scale changes.
        {
            let assets = assets.clone();
            let app_icon = app_icon.clone();
            let check_icon = check_icon.clone();
            let error_icon = error_icon.clone();
            let sm = style_manager.clone();
            let reload = move |scale: i32| {
                let dark = sm.is_dark();
                set_asset_image(&app_icon,    &assets, "app-icon",        dark, scale);
                set_asset_image(&check_icon,  &assets, "install-success", dark, scale);
                set_asset_image(&error_icon,  &assets, "install-error",   dark, scale);
            };
            let reload = std::rc::Rc::new(reload);
            let window_weak = window.downgrade();

            let r = reload.clone();
            let w = window_weak.clone();
            style_manager.connect_notify_local(Some("dark"), move |_, _| {
                r(w.upgrade().map(|w| w.scale_factor()).unwrap_or(1));
            });

            let r = reload.clone();
            window.connect_realize(move |win| r(win.scale_factor()));

            let r = reload.clone();
            window.connect_notify_local(Some("scale-factor"), move |win, _| r(win.scale_factor()));
        }

        // ── Uninstall ────────────────────────────────────────────────────────
        let uninstall_page = GtkBox::new(Orientation::Vertical, 0);
        uninstall_page.set_margin_top(30);
        uninstall_page.set_margin_bottom(30);
        uninstall_page.set_margin_start(30);
        uninstall_page.set_margin_end(30);

        let uc = GtkBox::new(Orientation::Vertical, 12);
        uc.set_vexpand(true);
        uc.set_valign(gtk4::Align::Center);
        uc.set_halign(gtk4::Align::Center);

        let already_label = Label::new(Some(&if uninstaller {
            gettext("Uninstall %s?").replacen("%s", APP_NAME, 1)
        } else {
            gettext("%s is already installed").replacen("%s", APP_NAME, 1)
        }));
        already_label.add_css_class("title-2");
        uc.append(&already_label);

        let install_path_label = Label::new(
            existing.as_deref().and_then(|p| p.to_str()),
        );
        install_path_label.add_css_class("dim-label");
        uc.append(&install_path_label);

        uninstall_page.append(&uc);

        let ub = GtkBox::new(Orientation::Horizontal, 0);
        let us1 = Label::new(None);
        us1.set_hexpand(true);
        let uninstall_btn = Button::builder()
            .label(gettext("Uninstall"))
            .css_classes(["destructive-action", "pill"])
            .width_request(120)
            .build();
        let us2 = Label::new(None);
        us2.set_hexpand(true);
        ub.append(&us1);
        ub.append(&uninstall_btn);
        ub.append(&us2);
        uninstall_page.append(&ub);

        stack.add_named(&uninstall_page, Some("uninstall"));

        window.set_child(Some(&stack));

        if existing.is_some() {
            stack.set_visible_child_name("uninstall");
        }

        // Next: welcome → license (if present) or options
        {
            let stack = stack.clone();
            next_btn.connect_clicked(move |_| {
                stack.set_transition_type(StackTransitionType::SlideLeft);
                stack.set_visible_child_name(if has_license { "license" } else { "options" });
            });
        }

        // Previous: options → license (if present) or welcome
        {
            let stack = stack.clone();
            prev_btn.connect_clicked(move |_| {
                stack.set_transition_type(StackTransitionType::SlideRight);
                stack.set_visible_child_name(if has_license { "license" } else { "welcome" });
            });
        }

        // Browse
        {
            let entry = entry.clone();
            let window_weak = window.downgrade();
            browse_btn.connect_clicked(move |_| {
                let Some(win) = window_weak.upgrade() else { return };
                let dialog = FileDialog::builder()
                    .title(gettext("Choose install folder"))
                    .build();
                let entry = entry.clone();
                dialog.select_folder(Some(&win), None::<&gio::Cancellable>, move |result| {
                    if let Ok(file) = result {
                        if let Some(path) = file.path() {
                            entry.set_text(&path.to_string_lossy());
                        }
                    }
                });
            });
        }

        // Install
        {
            let stack = stack.clone();
            let entry = entry.clone();
            let desktop_cb = desktop_cb.clone();
            let startmenu_cb = startmenu_cb.clone();
            let prev_btn = prev_btn.clone();
            let src = src.clone();

            install_btn.connect_clicked(move |btn| {
                let dest = PathBuf::from(entry.text().as_str());
                let desktop = desktop_cb.is_active();
                let startmenu = startmenu_cb.is_active();

                btn.set_label(&gettext("Installing..."));
                btn.set_sensitive(false);
                prev_btn.set_sensitive(false);

                let result: std::sync::Arc<std::sync::Mutex<Option<std::io::Result<()>>>> =
                    std::sync::Arc::new(std::sync::Mutex::new(None));
                let result_thread = std::sync::Arc::clone(&result);
                let src = src.clone();

                std::thread::spawn(move || {
                    *result_thread.lock().unwrap() = Some(do_install(&src, &dest, desktop, startmenu));
                });

                let stack = stack.clone();
                let btn = btn.clone();
                let prev_btn = prev_btn.clone();
                let failed_detail = failed_detail.clone();

                glib::idle_add_local(move || {
                    if let Some(res) = result.lock().unwrap().take() {
                        match res {
                            Ok(()) => {
                                stack.set_transition_type(StackTransitionType::SlideLeft);
                                stack.set_visible_child_name("complete");
                            }
                            Err(e) => {
                                failed_detail.set_text(&e.to_string());
                                stack.set_transition_type(StackTransitionType::SlideLeft);
                                stack.set_visible_child_name("failed");
                                btn.set_label(&gettext("Install"));
                                btn.set_sensitive(true);
                                prev_btn.set_sensitive(true);
                            }
                        }
                        return glib::ControlFlow::Break;
                    }
                    glib::ControlFlow::Continue
                });
            });
        }

        // Retry: failed → options
        {
            let stack = stack.clone();
            retry_btn.connect_clicked(move |_| {
                stack.set_transition_type(StackTransitionType::SlideRight);
                stack.set_visible_child_name("options");
            });
        }

        // Uninstall
        if let Some(install_path) = existing {
            let window_weak = window.downgrade();
            uninstall_btn.connect_clicked(move |btn| {
                if locked_by_other_process(&install_path) {
                    if let Some(win) = window_weak.upgrade() {
                        AlertDialog::builder()
                            .message(gettext("%s is still running").replacen("%s", APP_NAME, 1))
                            .detail(gettext("Please close %s before uninstalling.").replacen("%s", APP_NAME, 1))
                            .build()
                            .show(Some(&win));
                    }
                    return;
                }
                btn.set_label(&gettext("Uninstalling..."));
                btn.set_sensitive(false);

                let result: std::sync::Arc<std::sync::Mutex<Option<std::io::Result<()>>>> =
                    std::sync::Arc::new(std::sync::Mutex::new(None));
                let result_thread = std::sync::Arc::clone(&result);
                let path = install_path.clone();
                std::thread::spawn(move || {
                    *result_thread.lock().unwrap() = Some(do_uninstall(&path));
                });

                let btn = btn.clone();
                let window_weak = window_weak.clone();
                glib::idle_add_local(move || {
                    let mut guard = result.lock().unwrap();
                    if let Some(res) = guard.take() {
                        drop(guard);
                        match res {
                            Ok(()) => {
                                if let Ok(exe) = std::env::current_exe() {
                                    if exe.starts_with(std::env::temp_dir()) {
                                        if let Some(tmp) = exe.parent() {
                                            let pid = std::process::id();
                                            let t = tmp.to_string_lossy().replace('\'', "''");
                                            let script = format!(
                                                "Wait-Process -Id {pid} -ErrorAction SilentlyContinue; \
                                                 Remove-Item -Recurse -Force '{t}'"
                                            );
                                            let _ = std::process::Command::new("powershell")
                                                .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                                                .creation_flags(CREATE_NO_WINDOW)
                                                .spawn();
                                        }
                                    }
                                }
                                if let Some(win) = window_weak.upgrade() {
                                    let win_weak = win.downgrade();
                                    AlertDialog::builder()
                                        .message(gettext("%s has been uninstalled.").replacen("%s", APP_NAME, 1))
                                        .build()
                                        .choose(Some(&win), None::<&gio::Cancellable>, move |_| {
                                            if let Some(w) = win_weak.upgrade() { w.close(); }
                                        });
                                }
                            }
                            Err(e) => {
                                btn.set_label(&gettext("Uninstall"));
                                btn.set_sensitive(true);
                                if let Some(win) = window_weak.upgrade() {
                                    AlertDialog::builder()
                                        .message(gettext("Uninstallation failed"))
                                        .detail(e.to_string())
                                        .build()
                                        .show(Some(&win));
                                }
                            }
                        }
                        return glib::ControlFlow::Break;
                    }
                    glib::ControlFlow::Continue
                });
            });
        }

        // Finish
        {
            let window_weak = window.downgrade();
            finish_btn.connect_clicked(move |_| {
                let dest = PathBuf::from(entry.text().as_str());
                if !APP_EXE.is_empty() {
                    let _ = std::process::Command::new(dest.join(APP_EXE))
                        .current_dir(&dest)
                        .spawn();
                }
                if let Some(win) = window_weak.upgrade() {
                    win.close();
                }
            });
        }

        window.present();
    }

    pub fn run() {
        let self_exe = std::env::current_exe().unwrap_or_default();
        let exe_dir = self_exe.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        init_gettext(&exe_dir);

        let in_temp = self_exe.starts_with(std::env::temp_dir());

        if !in_temp {
            if let Some(install_dir) = existing_install() {
                if self_exe.starts_with(&install_dir) {
                    let tmp = temp_installer_dir();
                    let tmp_exe = tmp.join(self_exe.file_name().unwrap_or_default());
                    if tmp.exists() {
                        if locked_by_other_process(&tmp) {
                            let name = format!("{APP_NAME} Uninstaller");
                            show_native_error(
                                &gettext("%s is already running.").replacen("%s", &name, 1),
                            );
                            return;
                        }
                        let _ = std::fs::remove_dir_all(&tmp);
                    }
                    if copy_dir(&install_dir, &tmp).is_ok() {
                        let _ = std::process::Command::new(&tmp_exe)
                            .env("GTKWININSTALLER_INSTALL_DIR", &install_dir)
                            .creation_flags(CREATE_NO_WINDOW)
                            .spawn();
                    }
                    return;
                }
            }
        }

        setup_env(&exe_dir);
        glib::set_application_name(APP_NAME);
        let app = adw::Application::builder().application_id(APP_ID).build();
        app.connect_activate(build_ui);
        app.run();
    }
}
