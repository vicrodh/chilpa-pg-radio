//! The plugin against replies recorded from the directory on 2026-09-30.
//!
//! Nothing here goes out: the SDK's test host answers each URL with the
//! file recorded for it, and a URL nobody recorded fails like no network.

use chilpa_pg_radio::{browse, resolve, search};
use chilpa_plugin_sdk::api::{Browse, EntryKind, Query, Resolve};
use chilpa_plugin_sdk::testing;

const SERVER: &str = "https://all.api.radio-browser.info";
const MIRROR: &str = "https://de2.api.radio-browser.info";
const UUID: &str = "d28420a4-eccf-47a2-ace1-088c7e7cb7e0";

fn recorded(name: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/recorded/{name}.json", env!("CARGO_MANIFEST_DIR")))
        .expect("the recorded reply is in the repository")
}

fn stations_url(server: &str, filter: &str, offset: usize) -> String {
    let and = if filter.is_empty() { "" } else { "&" };
    format!(
        "{server}/json/stations/search?{filter}{and}order=clickcount&reverse=true&hidebroken=true&limit=20&offset={offset}"
    )
}

fn answer_the_columns() {
    testing::answer(&format!("{SERVER}/json/countries?order=name&hidebroken=true"), 200, &recorded("countries"));
    testing::answer(
        &format!("{SERVER}/json/languages?order=stationcount&reverse=true&hidebroken=true&limit=200"),
        200,
        &recorded("languages"),
    );
    testing::answer(
        &format!("{SERVER}/json/tags?order=stationcount&reverse=true&hidebroken=true&limit=200"),
        200,
        &recorded("tags"),
    );
}

#[test]
fn the_root_is_the_popular_stations_and_three_columns_to_narrow_them() {
    testing::reset();
    answer_the_columns();
    testing::answer(&stations_url(SERVER, "", 0), 200, &recorded("topclick"));
    let root = browse(Browse { language: "es".into(), ..Browse::root() }).unwrap();
    assert_eq!(root.entries[0].kind, EntryKind::Search, "the place to type comes first");
    assert_eq!(root.entries.len(), 6, "and then the stations");
    let titles: Vec<&str> = root.columns.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(&titles[..3], ["País", "Idioma", "Géneros"]);
    assert!(root.columns.iter().any(|column| column.id == "order"));
    let countries = &root.columns[0];
    assert_eq!(countries.id, "country");
    assert!(countries.entries.iter().any(|c| c.id == "US" && c.subtitle == "7,095 estaciones"));
    let names: Vec<String> = countries.entries.iter().map(|c| c.title.to_lowercase()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "by name");
    assert_eq!(root.columns[2].entries[0].id, "pop");
}

#[test]
fn picks_narrow_the_stations_by_all_of_them_and_ask_only_for_the_stations() {
    testing::reset();
    let filter = "countrycode=MX&language=spanish&languageExact=true&tag=rock%20en%20espa%C3%B1ol&tagExact=true";
    testing::answer(&stations_url(SERVER, filter, 0), 200, &recorded("topclick"));
    let picks = [("country", "MX"), ("language", "spanish"), ("genre", "rock en español")]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let narrowed = browse(Browse { picks, ..Browse::root() }).unwrap();
    assert_eq!(narrowed.entries.len(), 6);
    assert!(narrowed.columns.is_empty(), "the columns came with the first page");
    assert_eq!(testing::asked().len(), 1);

    // A country that is not a code is not sent.
    testing::reset();
    testing::answer(&stations_url(SERVER, "", 0), 200, "[]");
    let odd = [("country".to_string(), "../x".to_string())].into_iter().collect();
    assert_eq!(browse(Browse { picks: odd, ..Browse::root() }).unwrap().entries.len(), 1);
}

#[test]
fn the_popular_folder_lists_live_stations() {
    testing::reset();
    testing::answer(&stations_url(SERVER, "", 0), 200, &recorded("topclick"));
    let top = browse(Browse::at("top")).unwrap();
    assert_eq!(top.entries.len(), 5);
    let first = &top.entries[0];
    assert_eq!(first.title, "Sports Radio Brila FM");
    assert!(first.id.starts_with("station:"));
    assert!(first.live && first.playable);
    assert_eq!(first.kind, EntryKind::Item);
    assert_eq!(first.subtitle, "Nigeria · MP3 128 kbps");
    assert!(first.image_url.as_deref().unwrap().starts_with("https://"));
    assert_eq!(top.next, None, "five stations is not a full page");

    // What its page says: only the lines the directory has something for.
    let labels: Vec<&str> = first.facts.iter().map(|f| f.label.as_str()).collect();
    assert_eq!(labels, ["Country", "Broadcast", "Clicks in 24 hours", "Last checked", "Votes in the directory", "Website", "Listed in"]);
    assert_eq!(first.facts[1].value, "MP3 · 128 kbps");
    assert_eq!(first.facts.iter().find(|fact| fact.label == "Website").unwrap().url.as_deref(), Some("https://brila.net/"));
}

#[test]
fn countries_are_folders_keyed_by_their_code() {
    testing::reset();
    testing::answer(&format!("{SERVER}/json/countries?order=name&hidebroken=true"), 200, &recorded("countries"));
    let list = browse(Browse { language: "es".into(), ..Browse::at("countries") }).unwrap();
    let us = list.entries.iter().find(|e| e.id == "country:US").expect("it is listed by its code");
    assert_eq!(us.subtitle, "7,095 estaciones");
    let names: Vec<String> = list.entries.iter().map(|e| e.title.to_lowercase()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "by name, not by the code the directory orders them by");

    testing::answer(&stations_url(SERVER, "countrycode=US", 0), 200, &recorded("topclick"));
    assert_eq!(browse(Browse::at("country:US")).unwrap().entries.len(), 5);
}

#[test]
fn genres_and_languages_are_capitalized_and_filter_exactly() {
    testing::reset();
    testing::answer(
        &format!("{SERVER}/json/tags?order=stationcount&reverse=true&hidebroken=true&limit=200"),
        200,
        &recorded("tags"),
    );
    let tags = browse(Browse::at("tags")).unwrap();
    assert_eq!(tags.entries[0].title, "Pop");
    assert_eq!(tags.entries[0].id, "tag:pop");

    testing::answer(&stations_url(SERVER, "tag=smooth%20jazz&tagExact=true", 0), 200, "[]");
    let empty = browse(Browse::at("tag:smooth jazz")).unwrap();
    assert!(empty.entries.is_empty());
    assert_eq!(empty.title, "Smooth Jazz");
}

#[test]
fn searching_asks_by_name_and_an_empty_search_asks_nothing() {
    testing::reset();
    let nothing = browse(Browse { text: Some("  ".into()), ..Browse::at("search") }).unwrap();
    assert!(nothing.entries.is_empty() && testing::asked().is_empty());

    testing::answer(&stations_url(SERVER, "name=jazz", 0), 200, &recorded("search"));
    let found = browse(Browse { text: Some("jazz".into()), ..Browse::at("search") }).unwrap();
    assert_eq!(found.entries.len(), 4);
    assert_eq!(found.entries[0].subtitle, "The United States Of America · easy listening, jazz, smooth jazz · MP3 192 kbps");
}

#[test]
fn a_full_page_gives_a_cursor_and_the_cursor_is_the_offset() {
    testing::reset();
    let one: serde_json::Value = serde_json::from_str::<Vec<serde_json::Value>>(&recorded("topclick")).unwrap().remove(0);
    let full = serde_json::to_string(&vec![one; 20]).unwrap();
    testing::answer(&stations_url(SERVER, "", 0), 200, &full);
    let first = browse(Browse::at("top")).unwrap();
    assert_eq!(first.next.as_deref(), Some("20"));

    testing::answer(&stations_url(SERVER, "", 20), 200, &recorded("topclick"));
    let second = browse(Browse { cursor: first.next, ..Browse::at("top") }).unwrap();
    assert_eq!(second.entries.len(), 5);
    assert_eq!(second.next, None);
}

#[test]
fn a_station_that_only_has_hls_is_listed_and_says_why_it_does_not_play() {
    testing::reset();
    let mut list: Vec<serde_json::Value> = serde_json::from_str(&recorded("topclick")).unwrap();
    list[0]["hls"] = 1.into();
    testing::answer(&stations_url(SERVER, "", 0), 200, &serde_json::to_string(&list).unwrap());
    let top = browse(Browse { language: "es".into(), ..Browse::at("top") }).unwrap();
    assert_eq!(top.entries.len(), 5, "it is still listed");
    assert!(!top.entries[0].playable);
    assert!(top.entries[0].why_not.contains("HLS"));
    assert!(top.entries[1].playable);
}

#[test]
fn when_the_first_server_is_down_the_next_one_answers() {
    testing::reset();
    testing::answer(&stations_url(SERVER, "", 0), 502, "bad gateway");
    testing::answer(&stations_url(MIRROR, "", 0), 200, &recorded("topclick"));
    assert_eq!(browse(Browse::at("top")).unwrap().entries.len(), 5);
    assert_eq!(testing::asked().len(), 2);
}

#[test]
fn when_no_server_answers_the_reason_is_said() {
    testing::reset();
    let error = browse(Browse::at("top")).unwrap_err();
    assert!(!error.is_empty());
    assert_eq!(testing::asked().len(), 3, "every server was tried");
}

#[test]
fn resolving_gives_the_address_the_directory_already_followed() {
    testing::reset();
    testing::answer(&format!("{SERVER}/json/stations/byuuid/{UUID}"), 200, &recorded("byuuid"));
    let stream = resolve(Resolve::new(&format!("station:{UUID}"))).unwrap();
    // The station publishes an .m3u; the directory followed it to the stream.
    assert_eq!(stream.url, "http://jking.cdnstream1.com/b22139_128mp3");
    assert_eq!(stream.codec.as_deref(), Some("mp3"));
    assert!(stream.usable() && stream.delivery.playable());
    assert_eq!(stream.expires_at, None);
    // The directory is told the station was chosen, and that failing changes nothing.
    assert_eq!(testing::asked().last().unwrap(), &format!("{SERVER}/json/url/{UUID}"));

    assert!(resolve(Resolve::new("track:1")).is_err(), "only what it gave out");
    testing::answer(&format!("{SERVER}/json/stations/byuuid/gone"), 200, "[]");
    testing::answer(&format!("{MIRROR}/json/stations/byuuid/gone"), 200, "[]");
    assert!(resolve(Resolve::new("station:gone")).is_err());
}

#[test]
fn chilpa_s_own_search_gets_stations_as_tracks_without_the_segmented_ones() {
    testing::reset();
    let filter = "name=jazz";
    let url = format!(
        "{SERVER}/json/stations/search?{filter}&order=clickcount&reverse=true&hidebroken=true&limit=10&offset=0"
    );
    let mut list: Vec<serde_json::Value> = serde_json::from_str(&recorded("search")).unwrap();
    list[1]["hls"] = 1.into();
    testing::answer(&url, 200, &serde_json::to_string(&list).unwrap());
    let found = search(Query::new("jazz", 10)).unwrap();
    assert_eq!(found.tracks.len(), 3);
    assert_eq!(found.tracks[0].title, "101 SMOOTH JAZZ");
    assert!(found.tracks[0].external_id.starts_with("station:"));
}

#[test]
fn medium_document_keeps_extra_facets_outside_the_legacy_columns() {
    testing::reset();
    answer_the_columns();
    testing::answer(&stations_url(SERVER, "", 0), 200, &recorded("topclick"));
    let document = chilpa_pg_radio::browse_document(Browse::root()).unwrap();
    assert_eq!(document["medium"], "radio");
    assert_eq!(document["columns"].as_array().unwrap().len(), 3);
    assert!(document["facets"].as_array().unwrap().iter().any(|facet| facet["id"] == "codec"));
}
