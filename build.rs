use sha2::{Digest, Sha256};
use std::{env, error::Error, fmt::Write, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    let output = PathBuf::from(env::var("OUT_DIR")?);
    let mut assets = vec![
        ("site.css", "text/css; charset=utf-8"),
        ("mark.svg", "image/svg+xml"),
        ("favicon.svg", "image/svg+xml"),
        ("images.js", "text/javascript; charset=utf-8"),
        ("navigation.js", "text/javascript; charset=utf-8"),
        ("vendor/datastar-1.0.4.js", "text/javascript; charset=utf-8"),
        ("images/crowns-adrift-island-480.webp", "image/webp"),
        ("images/crowns-adrift-island-800.webp", "image/webp"),
        ("images/crowns-adrift-island-1200.webp", "image/webp"),
        ("images/crowns-adrift-detail-480.webp", "image/webp"),
        ("images/crowns-adrift-detail-800.webp", "image/webp"),
        ("images/crowns-adrift-detail-1200.webp", "image/webp"),
        ("images/cold-orbit-flight-480.webp", "image/webp"),
        ("images/cold-orbit-flight-800.webp", "image/webp"),
        ("images/cold-orbit-flight-1200.webp", "image/webp"),
        ("images/spice-cad-editor-480.webp", "image/webp"),
        ("images/spice-cad-editor-800.webp", "image/webp"),
        ("images/spice-cad-editor-1200.webp", "image/webp"),
        ("images/studio-social.png", "image/png"),
        ("images/crowns-adrift-island.webp", "image/webp"),
        ("images/crowns-adrift-detail.webp", "image/webp"),
        ("images/crowns-adrift-detail-preview.webp", "image/webp"),
        ("images/crowns-adrift-fields.webp", "image/webp"),
        ("images/crowns-adrift-fields-preview.webp", "image/webp"),
        ("images/crowns-adrift-granary.webp", "image/webp"),
        ("images/crowns-adrift-granary-preview.webp", "image/webp"),
        ("images/crowns-adrift-foundry.webp", "image/webp"),
        ("images/crowns-adrift-foundry-preview.webp", "image/webp"),
        ("images/crowns-adrift-levy.webp", "image/webp"),
        ("images/crowns-adrift-levy-preview.webp", "image/webp"),
        ("images/crowns-adrift-commander.webp", "image/webp"),
        ("images/crowns-adrift-commander-preview.webp", "image/webp"),
        ("images/cold-orbit-flight.webp", "image/webp"),
        ("images/spice-cad-editor.webp", "image/webp"),
        ("images/crowns-adrift-island-preview.webp", "image/webp"),
        ("images/cold-orbit-flight-preview.webp", "image/webp"),
        ("images/spice-cad-editor-preview.webp", "image/webp"),
    ];
    assets.sort_by_key(|(path, _)| path.ends_with(".css"));
    assert!(assets.len() < 128);
    let mut urls: Vec<(&str, String)> = Vec::with_capacity(assets.len());
    let mut manifest = String::from("pub const ASSETS: &[Asset] = &[\n");
    for (path, content_type) in assets {
        let source = format!("static/{path}");
        println!("cargo:rerun-if-changed={source}");
        let mut bytes = fs::read(&source)?;
        assert!(bytes.len() < 4 * 1024 * 1024);
        if path.ends_with(".css") {
            let mut css = String::from_utf8(bytes)?;
            for (dependency, url) in &urls {
                css = css.replace(&format!("/static/{dependency}"), url);
            }
            bytes = css.into_bytes();
        }
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let url = format!("/static/{hash}/{path}");
        let target = output.join(path);
        fs::create_dir_all(target.parent().ok_or("asset has no parent directory")?)?;
        fs::write(&target, &bytes)?;
        writeln!(
            manifest,
            "Asset {{ path: {path:?}, url: {url:?}, content_type: {content_type:?}, bytes: include_bytes!({target:?}) }},"
        )?;
        urls.push((path, url));
    }
    manifest.push_str("];\n");
    fs::write(output.join("assets.rs"), manifest)?;
    Ok(())
}
