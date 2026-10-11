//! The few words the plugin says itself: its folders and its reasons.
//!
//! A plugin has no access to Chilpa's catalog, so it carries its own, and
//! Chilpa tells it which language the window speaks.

pub struct Words {
    pub search: &'static str,
    pub top: &'static str,
    pub countries: &'static str,
    pub tags: &'static str,
    pub languages: &'static str,
    pub segmented: &'static str,
    pub country: &'static str,
    pub language: &'static str,
    pub genres: &'static str,
    pub sound: &'static str,
    pub votes: &'static str,
    pub website: &'static str,
    pub directory: &'static str,
    one_station: &'static str,
    many_stations: &'static str,
}

const ENGLISH: Words = Words {
    search: "Search stations",
    top: "Popular",
    countries: "By country",
    tags: "By genre",
    languages: "By language",
    segmented: "This station only broadcasts in HLS, which Chilpa cannot play yet",
    country: "Country",
    language: "Language",
    genres: "Genres",
    sound: "Broadcast",
    votes: "Votes in the directory",
    website: "Website",
    directory: "Listed in",
    one_station: "station",
    many_stations: "stations",
};

const SPANISH: Words = Words {
    search: "Buscar estaciones",
    top: "Populares",
    countries: "Por país",
    tags: "Por género",
    languages: "Por idioma",
    segmented: "Esta estación solo transmite en HLS, que Chilpa todavía no reproduce",
    country: "País",
    language: "Idioma",
    genres: "Géneros",
    sound: "Transmisión",
    votes: "Votos en el directorio",
    website: "Sitio",
    directory: "Listada en",
    one_station: "estación",
    many_stations: "estaciones",
};

impl Words {
    /// The words for a language code. Anything unknown gets English.
    pub fn of(language: &str) -> &'static Words {
        match language.get(..2).map(str::to_ascii_lowercase).as_deref() {
            Some("es") => &SPANISH,
            _ => &ENGLISH,
        }
    }

    /// "1 station", "2,345 stations".
    pub fn stations(&self, count: u64) -> String {
        let word = if count == 1 { self.one_station } else { self.many_stations };
        format!("{} {word}", grouped(count))
    }
}

fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The directory writes genres and languages in lowercase; a list of them
/// reads better with each word capitalized.
pub fn titled(name: &str) -> String {
    name.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_count_is_said_in_the_window_s_language() {
        assert_eq!(Words::of("en").stations(1), "1 station");
        assert_eq!(Words::of("es").stations(7095), "7,095 estaciones");
        assert_eq!(Words::of("es_MX").stations(2), "2 estaciones");
        assert_eq!(Words::of("").stations(1_234_567), "1,234,567 stations");
        assert_eq!(Words::of("x").top, "Popular", "an unknown language gets English");
    }

    #[test]
    fn a_name_in_lowercase_is_capitalized_word_by_word() {
        assert_eq!(titled("smooth jazz"), "Smooth Jazz");
        assert_eq!(titled("  español "), "Español");
        assert_eq!(titled(""), "");
    }
}
