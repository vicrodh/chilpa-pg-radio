//! The requests to the station directory, and what comes back.
//!
//! The directory runs on several servers that mirror each other. They are
//! tried in order, and the first one that answers serves the call.

use chilpa_plugin_sdk::host;
use serde::Deserialize;

pub use chilpa_plugin_sdk::host::encode;

/// The round robin name first, then two servers by name in case it points
/// at one that is down.
const SERVERS: &[&str] = &[
    "https://all.api.radio-browser.info",
    "https://de2.api.radio-browser.info",
    "https://de1.api.radio-browser.info",
];

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Station {
    #[serde(default)]
    pub stationuuid: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub url_resolved: String,
    #[serde(default)]
    pub favicon: String,
    #[serde(default)]
    pub tags: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub codec: String,
    #[serde(default)]
    pub bitrate: u32,
    #[serde(default)]
    pub hls: u8,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub votes: i64,
    #[serde(default)]
    pub clickcount: u64,
    #[serde(default)]
    pub lastcheckok: u8,
    #[serde(default)]
    pub lastchecktime: String,
}

impl Station {
    /// One line under the name: where it is from, what it plays, how it
    /// sounds. Each part only if the directory knows it.
    pub fn summary(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if !self.country.trim().is_empty() {
            parts.push(self.country.trim().to_string());
        }
        let tags: Vec<&str> =
            self.tags.split(',').map(str::trim).filter(|t| !t.is_empty()).take(3).collect();
        if !tags.is_empty() {
            parts.push(tags.join(", "));
        }
        let codec = self.codec.trim();
        if !codec.is_empty() && !codec.eq_ignore_ascii_case("unknown") {
            if self.bitrate > 0 {
                parts.push(format!("{codec} {} kbps", self.bitrate));
            } else {
                parts.push(codec.to_string());
            }
        }
        parts.join(" · ")
    }

    /// The station's picture, if it gave one that can be fetched.
    pub fn image(&self) -> Option<String> {
        let url = self.favicon.trim();
        (url.starts_with("https://") || url.starts_with("http://")).then(|| url.to_string())
    }
}

/// A country, a genre or a language, with how many stations it has.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Group {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub iso_3166_1: String,
    #[serde(default)]
    pub stationcount: u64,
}

fn ask<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    let mut last = String::from("the station directory did not answer");
    for server in SERVERS {
        match chilpa_plugin_sdk::cache::get(&format!("{server}{path}"), if path.contains("/stations/") { 300 } else { 3600 }).and_then(|reply| reply.json::<T>()) {
            Ok(value) => return Ok(value),
            Err(reason) => last = reason,
        }
    }
    Err(last)
}

/// Stations matching a filter, the most listened to first. Stations whose
/// last check failed are left out by the directory.
pub fn stations(filter: &str, limit: usize, offset: usize) -> Result<Vec<Station>, String> {
    let and = if filter.is_empty() { "" } else { "&" };
    let order = if filter.split('&').any(|part| part.starts_with("order=")) { "" } else { "order=clickcount&reverse=true&" };
    ask(&format!("/json/stations/search?{filter}{and}{order}hidebroken=true&limit={limit}&offset={offset}"))
}

pub fn countries() -> Result<Vec<Group>, String> {
    ask("/json/countries?order=name&hidebroken=true")
}

/// The genres or the languages with the most stations.
pub fn groups(what: &str, limit: usize) -> Result<Vec<Group>, String> {
    ask(&format!("/json/{what}?order=stationcount&reverse=true&hidebroken=true&limit={limit}"))
}

/// One station by its id. The directory is also told it was chosen, which
/// is what it orders its popular list by; if that fails the station still
/// plays.
pub fn station(uuid: &str) -> Result<Station, String> {
    let uuid = encode(uuid);
    let found: Vec<Station> = ask(&format!("/json/stations/byuuid/{uuid}"))?;
    let station = found.into_iter().next().ok_or("the directory no longer has that station")?;
    let _ = host::get(&format!("{}/json/url/{uuid}", SERVERS[0]));
    Ok(station)
}
