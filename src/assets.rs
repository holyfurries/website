pub struct Asset {
    pub path: &'static str,
    pub url: &'static str,
    pub content_type: &'static str,
    pub bytes: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

pub fn url(path: &str) -> &'static str {
    ASSETS
        .iter()
        .find(|asset| asset.path == path)
        .expect("template references a bundled asset")
        .url
}

pub fn image_url(stem: &str, suffix: &str) -> &'static str {
    url(&format!("images/{stem}{suffix}"))
}
