use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;
use crate::parser;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElasticEnd {
    pub diphthongs: Vec<String>,
    pub vowels: Vec<String>,
    pub sonorants: Vec<String>,
    pub special_sonorant_endings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Syllables {
    pub vowels: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeuristicsData {
    pub double_oomph_prefix: String,
    pub elastic_end: ElasticEnd,
    pub syllables: Syllables,
}

pub fn parse_heuristics_data(val: serde_json::Value) -> Result<HeuristicsData, serde_json::Error> {
    let mut data = val;
    if let Some(obj) = data.as_object() {
        if obj.len() == 1 && !obj.contains_key("double_oomph_prefix") {
            let key = obj.keys().next().unwrap().clone();
            if let Some(inner) = obj.get(&key) {
                if inner.is_object() {
                    data = inner.clone();
                }
            }
        }
    }
    serde_json::from_value(data)
}

pub fn get_default_english_heuristics() -> HeuristicsData {
    HeuristicsData {
        double_oomph_prefix: "oo".to_string(),
        elastic_end: ElasticEnd {
            diphthongs: vec![
                "ee", "oo", "aa", "ou", "ow", "ay", "ey", "oy", "ea", "ai", "ie", "ue", "ui", "uy",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            vowels: vec!["a", "e", "i", "o", "u", "y"]
                .into_iter()
                .map(String::from)
                .collect(),
            sonorants: vec!["r", "l", "m", "n"]
                .into_iter()
                .map(String::from)
                .collect(),
            special_sonorant_endings: vec!["ng"].into_iter().map(String::from).collect(),
        },
        syllables: Syllables {
            vowels: "aeiouyAEIOUY".to_string(),
        },
    }
}

static EMBEDDED_LANGS: OnceLock<HashMap<String, HeuristicsData>> = OnceLock::new();

const LANGUAGE_ORDER: &[&str] = &[
    "en",
    "arabic",
    "bengali",
    "french",
    "gujarati",
    "hindi",
    "japanese",
    "kannada",
    "korean",
    "malayalam",
    "marathi_devanagari",
    "odia",
    "portuguese",
    "punjabi_gurmukhi",
    "russian",
    "spanish",
    "tamil",
    "telugu",
];

fn normalize_language_mode(lang: &str) -> String {
    let normalized = lang.trim().to_lowercase().replace('-', "_");
    let primary = normalized.split('_').next().unwrap_or(&normalized);
    let mapped = match primary {
        "eng" | "english" => "en".to_string(),
        "ar" => "arabic".to_string(),
        "bn" => "bengali".to_string(),
        "fr" => "french".to_string(),
        "gu" => "gujarati".to_string(),
        "hi" => "hindi".to_string(),
        "ja" | "jp" | "japanese" => "japanese".to_string(),
        "kn" => "kannada".to_string(),
        "ko" | "kr" | "korean" => "korean".to_string(),
        "ml" => "malayalam".to_string(),
        "mr" => "marathi_devanagari".to_string(),
        "or" => "odia".to_string(),
        "pt" => "portuguese".to_string(),
        "pa" => "punjabi_gurmukhi".to_string(),
        "ru" => "russian".to_string(),
        "es" => "spanish".to_string(),
        "ta" => "tamil".to_string(),
        "te" => "telugu".to_string(),
        _ => normalized.clone(),
    };
    if mapped == normalized || !normalized.contains('_') {
        mapped
    } else {
        format!(
            "{}_{}",
            mapped,
            normalized
                .split_once('_')
                .map(|(_, rest)| rest)
                .unwrap_or("")
        )
    }
}

pub fn resolve_heuristics(lang: &str) -> HeuristicsData {
    let map = EMBEDDED_LANGS.get_or_init(|| {
        let mut m: HashMap<String, HeuristicsData> = HashMap::new();
        let lang_files = [
            ("en", include_str!("../../../heuristics_data/en.json")),
            (
                "arabic",
                include_str!("../../../heuristics_data/arabic.json"),
            ),
            (
                "bengali",
                include_str!("../../../heuristics_data/bengali.json"),
            ),
            (
                "french",
                include_str!("../../../heuristics_data/french.json"),
            ),
            (
                "gujarati",
                include_str!("../../../heuristics_data/gujarati.json"),
            ),
            ("hindi", include_str!("../../../heuristics_data/hindi.json")),
            (
                "japanese",
                include_str!("../../../heuristics_data/japanese.json"),
            ),
            (
                "kannada",
                include_str!("../../../heuristics_data/kannada.json"),
            ),
            (
                "korean",
                include_str!("../../../heuristics_data/korean.json"),
            ),
            (
                "malayalam",
                include_str!("../../../heuristics_data/malayalam.json"),
            ),
            (
                "marathi_devanagari",
                include_str!("../../../heuristics_data/marathi_devanagari.json"),
            ),
            ("odia", include_str!("../../../heuristics_data/odia.json")),
            (
                "portuguese",
                include_str!("../../../heuristics_data/portuguese.json"),
            ),
            (
                "punjabi_gurmukhi",
                include_str!("../../../heuristics_data/punjabi_gurmukhi.json"),
            ),
            (
                "russian",
                include_str!("../../../heuristics_data/russian.json"),
            ),
            (
                "spanish",
                include_str!("../../../heuristics_data/spanish.json"),
            ),
            ("tamil", include_str!("../../../heuristics_data/tamil.json")),
            (
                "telugu",
                include_str!("../../../heuristics_data/telugu.json"),
            ),
        ];
        for (name, content) in lang_files {
            let mut cleaned = String::new();
            for line in content.lines() {
                let parts: Vec<&str> = line.split("//").collect();
                cleaned.push_str(parts[0]);
                cleaned.push('\n');
            }
            if let Ok(val) = serde_json::from_str(&cleaned) {
                if let Ok(hd) = parse_heuristics_data(val) {
                    m.insert(name.to_string(), hd);
                }
            }
        }
        m
    });

    let lang = normalize_language_mode(lang);

    // Exact match
    if let Some(hd) = map.get(&lang) {
        return hd.clone();
    }

    // Locale/prefix match
    for &k in LANGUAGE_ORDER {
        if lang.starts_with(k) {
            if let Some(hd) = map.get(k) {
                return hd.clone();
            }
        }
    }

    // Fallback to English
    map.get("en")
        .cloned()
        .unwrap_or_else(get_default_english_heuristics)
}

pub fn detect_language(text: &str) -> String {
    if text.is_empty() {
        return "en".to_string();
    }

    // Direct script-based checks for Japanese and Korean
    if text.chars().any(parser::is_moracore_kana) {
        return "japanese".to_string();
    }
    if text.chars().any(parser::is_batchimbound_syllable) {
        return "korean".to_string();
    }

    // Trigger lazy init of EMBEDDED_LANGS
    let _ = resolve_heuristics("en");
    let map = EMBEDDED_LANGS.get().unwrap();

    let mut scores = HashMap::new();
    for &lang in LANGUAGE_ORDER {
        let Some(config) = map.get(lang) else {
            continue;
        };
        let mut count = 0;
        for c in text.chars() {
            if config.syllables.vowels.contains(c) {
                count += if (c as u32) > 127 { 10 } else { 1 };
            }
        }
        scores.insert(lang.to_string(), count);
    }

    let mut max_lang = "en".to_string();
    let mut max_score = 0;
    for &lang in LANGUAGE_ORDER {
        let score = *scores.get(lang).unwrap_or(&0);
        if score > max_score {
            max_score = score;
            max_lang = lang.to_string();
        }
    }

    if max_score == 0 {
        return "en".to_string();
    }

    // Tie-breaker logic for Devanagari
    if scores.contains_key("hindi") || scores.contains_key("marathi_devanagari") {
        let hi_score = *scores.get("hindi").unwrap_or(&0);
        let mr_score = *scores.get("marathi_devanagari").unwrap_or(&0);
        if hi_score > 0 || mr_score > 0 {
            let marathi_vowels = ['ॲ', 'ऑ'];
            if text.chars().any(|c| marathi_vowels.contains(&c)) {
                return "marathi_devanagari".to_string();
            }
            if hi_score > 0 && mr_score > 0 {
                return "hindi".to_string();
            }
        }
    }

    // Tie-breaker logic for Latin-based scripts
    let fr_chars = "èêîôùûœæëïÈÊÎÔÙÛŒÆËÏ";
    let pt_chars = "ãõàâÃÕÀÂ";
    let es_chars = "ñíóúüÑÍÓÚÜ";

    let has_fr = text.chars().any(|c| fr_chars.contains(c));
    let has_pt = text.chars().any(|c| pt_chars.contains(c));
    let has_es = text.chars().any(|c| es_chars.contains(c));

    if has_fr {
        return "french".to_string();
    } else if has_pt {
        return "portuguese".to_string();
    } else if has_es {
        return "spanish".to_string();
    }

    let latin_alphabetic = text.chars().any(|c| c.is_ascii_alphabetic());
    let non_latin_alphabetic = text
        .chars()
        .any(|c| c.is_alphabetic() && !c.is_ascii_alphabetic());
    if latin_alphabetic && !non_latin_alphabetic {
        return "en".to_string();
    }

    max_lang
}
