use serde::{Deserialize, Serialize};
use std::{error::Error, time::Duration};

pub const MODS: [Mod; 5] = [
    Mod {
        package: "ClearMinimap",
        name: "Clear Minimap",
        symbol: "↗",
        description: "A square minimap showing teammates, properties, active deals, and objectives.",
        repository: "clear-minimap",
    },
    Mod {
        package: "CoordinatedPolice",
        name: "Coordinated Police",
        symbol: "✳",
        description: "Coordinated pursuits, searches, and police responses that scale with the severity of your offenses.",
        repository: "coordinated-police",
    },
    Mod {
        package: "CasinoLedger",
        name: "Casino Ledger",
        symbol: "♧",
        description: "Tracks each player’s casino profit and loss, with live standings and day-end summaries.",
        repository: "casino-ledger",
    },
    Mod {
        package: "DeathNotices",
        name: "Death Notices",
        symbol: "✝",
        description: "Death announcements based on how you died, with extra commentary for repeat deaths.",
        repository: "death-notices",
    },
    Mod {
        package: "ModSettings",
        name: "Mod Settings",
        symbol: "☷",
        description: "Adds a Mods tab to the game’s settings screen with switches, sliders, and dropdowns.",
        repository: "mod-settings",
    },
];

pub struct Mod {
    pub package: &'static str,
    pub name: &'static str,
    pub symbol: &'static str,
    pub description: &'static str,
    pub repository: &'static str,
}

impl Mod {
    pub fn release<'a>(&self, releases: &'a [Release]) -> Option<&'a Release> {
        releases
            .iter()
            .find(|release| release.package == self.package)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub package: String,
    pub version: String,
    pub description: String,
    pub downloads_total: u64,
}

impl Release {
    pub fn valid(&self) -> bool {
        if !MODS.iter().any(|item| item.package == self.package) {
            return false;
        }
        if self.version.is_empty() {
            return false;
        }
        if self.version.len() > 32 {
            return false;
        }
        if self.description.is_empty() {
            return false;
        }
        if self.description.len() > 512 {
            return false;
        }
        if self.description.chars().any(char::is_control) {
            return false;
        }
        if self.description.contains('\u{2014}') {
            return false;
        }
        self.version
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
    }
}

#[derive(Deserialize)]
struct Package {
    namespace: String,
    name: String,
    latest: Version,
}

#[derive(Deserialize)]
struct Version {
    version_number: String,
    is_active: bool,
    description: String,
}

#[derive(Deserialize)]
struct Metrics {
    #[serde(rename = "downloads")]
    downloads_total: u64,
}

pub async fn fetch_release(
    client: &reqwest::Client,
    package: &str,
) -> Result<Release, Box<dyn Error + Send + Sync>> {
    assert!(MODS.iter().any(|item| item.package == package));
    let url = format!("https://thunderstore.io/api/experimental/package/holyfurries/{package}/");
    let metrics_url =
        format!("https://thunderstore.io/api/v1/package-metrics/holyfurries/{package}/");
    let (body, metrics) = tokio::try_join!(
        fetch_metadata(client, &url),
        fetch_metadata(client, &metrics_url)
    )?;
    parse_release(package, &body, &metrics)
}

async fn fetch_metadata(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let mut response = client
        .get(url)
        .timeout(Duration::from_secs(15))
        .send()
        .await?
        .error_for_status()?;
    let mut body = Vec::with_capacity(4096);
    while let Some(chunk) = response.chunk().await? {
        if body.len() + chunk.len() > 65_536 {
            return Err("Thunderstore response exceeds 64 KiB".into());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn parse_release(
    package: &str,
    body: &[u8],
    metrics: &[u8],
) -> Result<Release, Box<dyn Error + Send + Sync>> {
    assert!(body.len() <= 65_536);
    assert!(metrics.len() <= 65_536);
    assert!(MODS.iter().any(|item| item.package == package));
    let result: Package = serde_json::from_slice(body)?;
    let metrics: Metrics = serde_json::from_slice(metrics)?;
    if result.namespace != "holyfurries" {
        return Err("Unexpected package owner".into());
    }
    if result.name != package {
        return Err("Unexpected package name".into());
    }
    if !result.latest.is_active {
        return Err("Package release is inactive".into());
    }
    let release = Release {
        package: result.name,
        version: result.latest.version_number,
        description: result.latest.description.trim().replace('\u{2014}', " - "),
        downloads_total: metrics.downloads_total,
    };
    if !release.valid() {
        return Err("Invalid release metadata".into());
    }
    assert!(release.version.len() <= 32);
    Ok(release)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn thunderstore_metadata_is_validated_before_it_replaces_the_catalog() {
        let metrics = br#"{"downloads":420}"#;
        let mut package = json!({"namespace":"holyfurries", "name":"ClearMinimap", "total_downloads":-1, "latest":{
            "version_number":"0.2.1", "is_active":true,
            "description":" A minimap\u{2014}with markers. ", "downloads":42
        }});
        let release = parse_release(
            "ClearMinimap",
            &serde_json::to_vec(&package).unwrap(),
            metrics,
        )
        .unwrap();
        assert_eq!(release.description, "A minimap - with markers.");
        assert_eq!(release.downloads_total, 420);
        for invalid in [
            br#"{"downloads":-1}"#.as_slice(),
            br#"{"downloads":null}"#,
            b"{}",
        ] {
            assert!(
                parse_release(
                    "ClearMinimap",
                    &serde_json::to_vec(&package).unwrap(),
                    invalid
                )
                .is_err()
            );
        }
        package["latest"]["is_active"] = json!(false);
        assert!(
            parse_release(
                "ClearMinimap",
                &serde_json::to_vec(&package).unwrap(),
                metrics
            )
            .is_err()
        );
        package["latest"]["is_active"] = json!(true);
        package["namespace"] = json!("someone-else");
        assert!(
            parse_release(
                "ClearMinimap",
                &serde_json::to_vec(&package).unwrap(),
                metrics
            )
            .is_err()
        );
        package["namespace"] = json!("holyfurries");
        for description in [String::new(), "x".repeat(513), "line\nbreak".into()] {
            package["latest"]["description"] = json!(description);
            assert!(
                parse_release(
                    "ClearMinimap",
                    &serde_json::to_vec(&package).unwrap(),
                    metrics
                )
                .is_err()
            );
        }
    }
}
