use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out = env::var("OUT_DIR").unwrap();
    // OUT_DIR is target/<profile>/build/.../out — go up 3 levels to get target/<profile>/
    let target_dir = Path::new(&out).ancestors().nth(3).unwrap().to_path_buf();
    let dest = target_dir.join("assets");

    // ASSETS_DIR: overridable for submodule use (default: assets/)
    println!("cargo:rerun-if-env-changed=ASSETS_DIR");
    let assets_dir = env::var("ASSETS_DIR").unwrap_or_else(|_| "assets".into());

    if dest.exists() { fs::remove_dir_all(&dest).unwrap(); }
    fs::create_dir_all(&dest).unwrap();

    println!("cargo:rustc-check-cfg=cfg(has_svg_assets)");
    let mut has_svg = false;
    for entry in fs::read_dir(&assets_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "png" | "svg") { continue; }
        if ext == "svg" { has_svg = true; }
        println!("cargo:rerun-if-changed={}", path.display());
        fs::copy(&path, dest.join(entry.file_name())).unwrap();
    }
    if has_svg { println!("cargo:rustc-cfg=has_svg_assets"); }

    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-env-changed=BUNDLE_ZIP");
        println!("cargo:rustc-check-cfg=cfg(bundled)");
        if let Ok(bundle_zip) = std::env::var("BUNDLE_ZIP") {
            println!("cargo:rustc-cfg=bundled");
            let bundle_dest = Path::new(&out).join("bundle.zip");
            let src_data = fs::read(&bundle_zip).unwrap();
            let mut src = zip::ZipArchive::new(std::io::Cursor::new(src_data)).unwrap();
            let out_file = fs::File::create(&bundle_dest).unwrap();
            let mut writer = zip::ZipWriter::new(out_file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .compression_level(Some(9));
            for i in 0..src.len() {
                let mut entry = src.by_index(i).unwrap();
                let name = entry.name().to_string();
                let name = name.strip_prefix("windows/").unwrap_or(&name);
                if name.is_empty() { continue; }
                if !entry.is_dir() {
                    writer.start_file(name, options).unwrap();
                    std::io::copy(&mut entry, &mut writer).unwrap();
                }
            }
            writer.finish().unwrap();
        }

        // Emit rerun-if-env-changed for every GTKWIN_* var (rustc tracks option_env!
        // automatically, but being explicit ensures build.rs itself also reruns).
        for var in &[
            "GTKWIN_APP_NAME", "GTKWIN_APP_DESCRIPTION", "GTKWIN_PUBLISHER", "GTKWIN_APP_ID",
            "GTKWIN_LICENSE_FILE", "GTKWIN_BUNDLED_UNINSTALLER", "GTKWIN_APP_ICON_DARK",
            "GTKWIN_APP_EXE",
        ] {
            println!("cargo:rerun-if-env-changed={var}");
        }

        println!("cargo:rerun-if-changed=config.rs");
        let config_src = fs::read_to_string("config.rs").unwrap_or_default();

        // GTKWIN_APP_ICON_DARK env wins; fall back to config.rs constant
        let icon_dark = match env::var("GTKWIN_APP_ICON_DARK") {
            Ok(v) => v == "1" || v == "true",
            Err(_) => config_src.contains("APP_ICON_DARK: bool = true"),
        };
        let icon_stem = if icon_dark { "app-icon-dark" } else { "app-icon" };

        let ico_path = Path::new(&out).join("app.ico");
        let svg_path = format!("{assets_dir}/{icon_stem}.svg");
        let png_path = format!("{assets_dir}/{icon_stem}.png");
        if Path::new(&svg_path).exists() {
            println!("cargo:rerun-if-changed={svg_path}");
            let data = fs::read(&svg_path).expect("failed to read app icon svg");
            let tree = resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default())
                .expect("failed to parse app icon svg");
            let mut pixmap = resvg::tiny_skia::Pixmap::new(256, 256).unwrap();
            let scale_x = 256.0 / tree.size().width();
            let scale_y = 256.0 / tree.size().height();
            resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale_x, scale_y), &mut pixmap.as_mut());
            let rgba = image::RgbaImage::from_raw(256, 256, pixmap.take())
                .expect("failed to create image from pixmap");
            image::DynamicImage::ImageRgba8(rgba)
                .save_with_format(&ico_path, image::ImageFormat::Ico)
                .expect("failed to write ico");
        } else {
            println!("cargo:rerun-if-changed={png_path}");
            let img = image::open(&png_path).expect("assets/app-icon.svg or assets/app-icon.png required");
            img.resize(256, 256, image::imageops::FilterType::Lanczos3)
                .save_with_format(&ico_path, image::ImageFormat::Ico)
                .expect("failed to write ico");
        }

        let mut res = winresource::WindowsResource::new();
        res.set_icon(ico_path.to_str().unwrap());
        res.compile().expect("failed to compile windows resources");
    }
}
