//! Internet radio for Chilpa.
//!
//! It reads the Radio Browser directory, which is open, kept by its
//! community and asks for no account. The plugin only lists and points:
//! Chilpa opens the stream and plays it.
//!
//! The root is the most listened to stations, narrowed by three columns
//! the listener picks in: country, language and genre. The directory
//! filters by all three at once, so any combination narrows the list; it
//! cannot narrow one column by another, so each is offered whole.
//!
//! ```text
//! (root)                stations, narrowed by the picks
//!   search              stations by name, narrowed by the picks too
//! station:<uuid>        a station, which is what sounds
//! ```
//!
//! The folders an earlier version offered (`top`, `countries`,
//! `country:<code>` and the like) still open, for whoever saved one.

mod directory;
mod words;

use chilpa_plugin_sdk::api::{
    Browse, Column, Entry, EntryKind, Fact, Listing, Query, RemoteTrack, Resolve, Results, Stream,
};
use std::collections::BTreeMap;
use chilpa_plugin_sdk::plugin;
use directory::{Group, Station};
use words::Words;

/// How many stations one page carries.
const PAGE: usize = 20;

/// How many genres and languages are offered: the directory has thousands
/// of each, most of them with one station and a typo.
const GROUPS: usize = 200;

const STATION: &str = "station:";

fn folder(id: &str, title: &str, subtitle: String) -> Entry {
    Entry { id: id.to_string(), title: title.to_string(), subtitle, playable: true, ..Entry::default() }
}

fn station_entry(station: &Station, words: &Words) -> Option<Entry> {
    let title = station.name.trim();
    if title.is_empty() || station.stationuuid.trim().is_empty() {
        return None;
    }
    // A segmented stream is listed and says why it does not play: a station
    // that vanishes from its own country looks like a broken directory.
    let segmented = station.hls != 0;
    Some(Entry {
        id: format!("{STATION}{}", station.stationuuid),
        title: title.to_string(),
        subtitle: station.summary(),
        image_url: station.image(),
        kind: EntryKind::Item,
        live: true,
        playable: !segmented,
        why_not: if segmented { words.segmented.to_string() } else { String::new() },
        facts: facts(station, words),
        ..Entry::default()
    })
}

/// What the directory knows of a station, for its page. Each line only if
/// the directory has something to say.
fn facts(station: &Station, words: &Words) -> Vec<Fact> {
    let place = [station.state.trim(), station.country.trim()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    let codec = station.codec.trim();
    let sound = match (codec.is_empty() || codec.eq_ignore_ascii_case("unknown"), station.bitrate) {
        (true, 0) => String::new(),
        (true, rate) => format!("{rate} kbps"),
        (false, 0) => codec.to_string(),
        (false, rate) => format!("{codec} · {rate} kbps"),
    };
    let tags = station.tags.split(',').map(str::trim).filter(|t| !t.is_empty()).collect::<Vec<_>>().join(", ");
    let homepage = station.homepage.trim();
    let mut facts = vec![
        Fact::new(words.country, &place),
        Fact::new(words.language, &words::titled(&station.language.replace(',', ", "))),
        Fact::new(words.genres, &tags),
        Fact::new(words.sound, &sound),
        Fact::new("Clicks in 24 hours", &station.clickcount.to_string()),
        Fact::new("Last checked", &station.lastchecktime),
    ];
    if station.votes > 0 {
        facts.push(Fact::new(words.votes, &station.votes.to_string()));
    }
    if homepage.starts_with("https://") || homepage.starts_with("http://") {
        facts.push(Fact { label: words.website.into(), value: homepage.into(), url: Some(homepage.into()) });
    }
    if station.lastcheckok == 0 { facts.push(Fact::new("Stream health", "Not responding lately")); }
    facts.push(Fact::new(words.directory, "Radio Browser"));
    facts.retain(|fact| !fact.value.trim().is_empty());
    facts
}

/// A page of stations, with the cursor to the next one if this one came full.
fn stations(title: String, filter: &str, cursor: Option<&str>, words: &Words) -> Result<Listing, String> {
    let offset: usize = cursor.and_then(|c| c.parse().ok()).unwrap_or(0);
    let found = directory::stations(filter, PAGE, offset)?;
    let full = found.len() >= PAGE;
    Ok(Listing {
        title,
        entries: found.iter().filter_map(|s| station_entry(s, words)).collect(),
        next: full.then(|| (offset + PAGE).to_string()),
        columns: Vec::new(),
    })
}

/// The directory's filter for what was picked in the columns.
fn filter_of(picks: &BTreeMap<String, String>) -> String {
    let mut parts = Vec::new();
    if let Some(code) = picks.get("country").filter(|c| c.len() == 2 && c.chars().all(|c| c.is_ascii_alphabetic())) {
        parts.push(format!("countrycode={}", directory::encode(code)));
    }
    if let Some(language) = picks.get("language").filter(|l| !l.trim().is_empty()) {
        parts.push(format!("language={}&languageExact=true", directory::encode(language)));
    }
    if let Some(tag) = picks.get("genre").filter(|t| !t.trim().is_empty()) {
        parts.push(format!("tag={}&tagExact=true", directory::encode(tag)));
    }
    for key in ["codec", "bitrateMin", "bitrateMax", "is_https", "order"] {
        if let Some(value) = picks.get(key).filter(|v| !v.is_empty()) {
            parts.push(format!("{key}={}", directory::encode(value)));
        }
    }
    if let Some(order) = picks.get("order") { parts.push(format!("reverse={}", order != "name")); }
    parts.join("&")
}

/// One column: what the directory groups its stations by, with how many
/// each has.
fn column(id: &str, title: &str, found: Vec<Group>, by_code: bool, words: &Words) -> Column {
    let entries = found
        .into_iter()
        .filter(|g| !g.name.trim().is_empty() && g.stationcount > 0)
        .map(|g| {
            let key = if by_code { g.iso_3166_1.clone() } else { g.name.clone() };
            folder(&key, &words::titled(&g.name), words.stations(g.stationcount))
        })
        .collect();
    Column { id: id.into(), title: title.into(), entries, preset: None }
}

fn groups(title: &str, found: Vec<Group>, prefix: &str, by_code: bool, words: &Words) -> Listing {
    let entries = found
        .into_iter()
        .filter(|g| !g.name.trim().is_empty())
        .map(|g| {
            let key = if by_code { g.iso_3166_1.clone() } else { g.name.clone() };
            folder(&format!("{prefix}{key}"), &words::titled(&g.name), words.stations(g.stationcount))
        })
        .collect();
    Listing { title: title.to_string(), entries, ..Listing::default() }
}

pub fn browse(ask: Browse) -> Result<Listing, String> {
    let words = Words::of(&ask.language);
    let cursor = ask.cursor.as_deref();
    let at = ask.at.as_str();

    if at.is_empty() {
        let mut picks = ask.picks.clone();
        let region = ask.language.split(['-', '_']).nth(1).filter(|part| part.len() == 2).unwrap_or("").to_uppercase();
        if !picks.contains_key("country") && !region.is_empty() { picks.insert("country".into(), region.clone()); }
        let mut listing = stations("Radio".into(), &filter_of(&picks), cursor, &words)?;
        // The first page of the root carries the columns and the place to
        // type; the window keeps them while the listener picks.
        if cursor.is_none() && ask.picks.is_empty() {
            let mut countries = directory::countries()?;
            countries.sort_by_key(|g| g.name.to_lowercase());
            listing.columns = vec![
                column("country", words.country, countries, true, &words),
                column("language", words.language, directory::groups("languages", GROUPS)?, false, &words),
                column("genre", words.genres, directory::groups("tags", GROUPS)?, false, &words),
            ];
            if !region.is_empty() { listing.columns[0].preset = Some(region); }
            let choices = |id: &str, title: &str, values: &[(&str, &str)]| Column {
                id: id.into(), title: title.into(), preset: None,
                entries: values.iter().map(|(id, title)| folder(id, title, String::new())).collect(),
            };
            listing.columns.extend([
                choices("order", "Sort", &[("clickcount", "Popularity (24 hours)"), ("clicktrend", "Trending"), ("votes", "Votes"), ("name", "Name"), ("bitrate", "Bitrate"), ("changetimestamp", "Recently changed"), ("clicktimestamp", "Recently played"), ("random", "Surprise me")]),
                choices("codec", "Codec", &[("MP3", "MP3"), ("AAC", "AAC"), ("AAC+", "AAC+"), ("OGG", "OGG"), ("FLAC", "FLAC")]),
                choices("bitrateMin", "Minimum bitrate", &[("64", "64 kbps"), ("128", "128 kbps"), ("192", "192 kbps"), ("256", "256 kbps"), ("320", "320 kbps")]),
                choices("is_https", "Connection", &[("true", "HTTPS only")]),
            ]);
        }
        if cursor.is_none() {
            listing.entries.insert(0, Entry { kind: EntryKind::Search, ..folder("search", words.search, String::new()) });
        }
        return Ok(listing);
    }
    match at {
        "search" => {
            let text = ask.text.unwrap_or_default();
            let text = text.trim();
            if text.is_empty() {
                return Ok(Listing { title: words.search.into(), ..Listing::default() });
            }
            // A search stays inside what was picked.
            let picked = filter_of(&ask.picks);
            let and = if picked.is_empty() { "" } else { "&" };
            let filter = format!("name={}{and}{picked}", directory::encode(text));
            stations(text.to_string(), &filter, cursor, &words)
        }
        "top" => stations(words.top.into(), "", cursor, &words),
        "countries" => {
            // The directory orders them by code, which reads as no order.
            let mut found = directory::countries()?;
            found.sort_by_key(|g| g.name.to_lowercase());
            Ok(groups(words.countries, found, "country:", true, &words))
        }
        "tags" => Ok(groups(words.tags, directory::groups("tags", GROUPS)?, "tag:", false, &words)),
        "languages" => {
            Ok(groups(words.languages, directory::groups("languages", GROUPS)?, "language:", false, &words))
        }
        _ => {
            if let Some(code) = at.strip_prefix("country:") {
                let filter = format!("countrycode={}", directory::encode(code));
                return stations(code.to_uppercase(), &filter, cursor, &words);
            }
            if let Some(tag) = at.strip_prefix("tag:") {
                let filter = format!("tag={}&tagExact=true", directory::encode(tag));
                return stations(words::titled(tag), &filter, cursor, &words);
            }
            if let Some(language) = at.strip_prefix("language:") {
                let filter = format!("language={}&languageExact=true", directory::encode(language));
                return stations(words::titled(language), &filter, cursor, &words);
            }
            Err(format!("there is no \"{at}\" here"))
        }
    }
}

/// Stations by name, for Chilpa's own search. A station is a track with no
/// artist and no length.
pub fn search(query: Query) -> Result<Results, String> {
    let text = query.text.trim();
    if text.is_empty() {
        return Ok(Results::default());
    }
    let filter = format!("name={}", directory::encode(text));
    let tracks = directory::stations(&filter, query.limit.clamp(1, PAGE), 0)?
        .into_iter()
        .filter(|s| s.hls == 0)
        .map(|s| RemoteTrack {
            external_id: format!("{STATION}{}", s.stationuuid),
            title: s.name.trim().to_string(),
            album: s.summary(),
            artwork_url: s.image(),
            ..RemoteTrack::default()
        })
        .collect();
    Ok(Results { tracks, ..Results::default() })
}

pub fn resolve(ask: Resolve) -> Result<Stream, String> {
    let uuid = ask
        .external_id
        .strip_prefix(STATION)
        .ok_or_else(|| format!("\"{}\" is not a station", ask.external_id))?;
    let station = directory::station(uuid)?;
    let url = if station.url_resolved.trim().is_empty() { &station.url } else { &station.url_resolved };
    let url = url.trim().to_string();
    if url.is_empty() {
        return Err("the directory has no address for that station".into());
    }
    let codec = Some(station.codec.to_lowercase()).filter(|c| !c.is_empty() && c != "unknown");
    Ok(Stream {
        url,
        codec: codec.clone(),
        quality: chilpa_plugin_sdk::api::Quality { codec, ..Default::default() },
        delivery: if station.hls != 0 {
            chilpa_plugin_sdk::api::Delivery::Hls
        } else {
            chilpa_plugin_sdk::api::Delivery::Progressive
        },
        ..Stream::default()
    })
}

plugin! {
    api_version: 1,
    chilpa_browse => browse_document,
    chilpa_search => search,
    chilpa_resolver => resolve,
}

/// I identify the medium explicitly so the host never recognizes a provider.
pub fn browse_document(ask: Browse) -> Result<chilpa_plugin_sdk::serde_json::Value, String> {
    let mut listing = browse(ask)?;
    let facets = if listing.columns.len() > 3 { listing.columns.split_off(3) } else { Vec::new() };
    let mut value = chilpa_plugin_sdk::serde_json::to_value(listing).map_err(|error| error.to_string())?;
    value["facets"] = chilpa_plugin_sdk::serde_json::to_value(facets).map_err(|error| error.to_string())?;
    value["medium"] = "radio".into();
    value["attribution"] = "Stations from Radio Browser".into();
    Ok(value)
}
