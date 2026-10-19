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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub package: String,
    pub version: String,
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
}

pub async fn fetch_release(
    client: &reqwest::Client,
    package: &str,
) -> Result<Release, Box<dyn Error + Send + Sync>> {
    assert!(MODS.iter().any(|item| item.package == package));
    let url = format!("https://thunderstore.io/api/experimental/package/holyfurries/{package}/");
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
    let result: Package = serde_json::from_slice(&body)?;
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
    };
    if !release.valid() {
        return Err("Invalid release metadata".into());
    }
    assert!(release.version.len() <= 32);
    Ok(release)
}
