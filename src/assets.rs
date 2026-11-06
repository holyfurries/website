use axum::http::{HeaderMap, StatusCode, header};

pub struct Asset {
    pub path: &'static str,
    pub url: &'static str,
    pub content_type: &'static str,
    pub bytes: &'static [u8],
    pub gzip: Option<&'static [u8]>,
    pub brotli: Option<&'static [u8]>,
}

impl Asset {
    pub fn representation(
        &self,
        headers: &HeaderMap,
    ) -> Result<(&'static [u8], Option<&'static str>), StatusCode> {
        let mut weights: [Option<u16>; 4] = [None; 4];
        let mut length_bytes = 0;
        for value in headers.get_all(header::ACCEPT_ENCODING) {
            length_bytes += value.as_bytes().len();
            if length_bytes > 4096 {
                return Err(StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE);
            }
            for encoding in value
                .to_str()
                .map_err(|_| StatusCode::BAD_REQUEST)?
                .split(',')
            {
                let mut parameters = encoding.split(';');
                let name = parameters.next().unwrap_or_default().trim();
                let index = if name.eq_ignore_ascii_case("br") {
                    0
                } else if name.eq_ignore_ascii_case("gzip") || name.eq_ignore_ascii_case("x-gzip") {
                    1
                } else if name.eq_ignore_ascii_case("identity") {
                    2
                } else if name == "*" {
                    3
                } else {
                    continue;
                };
                let mut weight = 1000;
                if let Some(parameter) = parameters.next() {
                    weight = parameter
                        .trim()
                        .split_once('=')
                        .filter(|(name, _)| name.trim().eq_ignore_ascii_case("q"))
                        .map_or(0, |(_, value)| encoding_weight(value.trim()));
                }
                if parameters.next().is_some() {
                    weight = 0;
                }
                weights[index] =
                    Some(weights[index].map_or(weight, |previous| previous.min(weight)));
            }
        }
        let wildcard = weights[3].unwrap_or(0);
        let identity = weights[2].unwrap_or(if weights[3] == Some(0) { 0 } else { 1 });
        let candidates = [
            (self.brotli, Some("br"), weights[0].unwrap_or(wildcard)),
            (self.gzip, Some("gzip"), weights[1].unwrap_or(wildcard)),
            (Some(self.bytes), None, identity),
        ];
        let mut selected = None;
        let mut weight_max = 0;
        for (bytes, encoding, weight) in candidates {
            if weight > weight_max {
                if let Some(bytes) = bytes {
                    selected = Some((bytes, encoding));
                    weight_max = weight;
                }
            }
        }
        selected.ok_or(StatusCode::NOT_ACCEPTABLE)
    }
}

fn encoding_weight(value: &str) -> u16 {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if fraction.len() > 3 || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return 0;
    }
    match whole {
        "0" => fraction
            .bytes()
            .zip([100, 10, 1])
            .map(|(digit, scale)| u16::from(digit - b'0') * scale)
            .sum(),
        "1" if fraction.bytes().all(|digit| digit == b'0') => 1000,
        _ => 0,
    }
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
