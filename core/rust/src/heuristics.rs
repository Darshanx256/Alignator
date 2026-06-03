use crate::language::HeuristicsData;
use crate::parser::split_into_syllables;

pub fn calculate_comma_cadence_pause(ms_per_syllable: f64) -> i32 {
    let raw = ms_per_syllable * 0.5;
    if raw < 80.0 {
        80
    } else if raw > 250.0 {
        250
    } else {
        raw as i32
    }
}

pub fn calculate_punctuation_pause(word: &str, ms_per_syllable: f64) -> i32 {
    if word.ends_with("...") {
        350
    } else if word.ends_with('.') || word.ends_with('!') || word.ends_with('?') {
        300
    } else if !(word.ends_with(',') || word.ends_with(';') || word.ends_with(':')) {
        0
    } else {
        calculate_comma_cadence_pause(ms_per_syllable)
    }
}

pub fn decompose_batchimbound(c: char) -> Option<(char, char, Option<char>)> {
    let cp = c as u32;
    if !(0xAC00..=0xD7A3).contains(&cp) {
        return None;
    }
    let syllable_index = cp - 0xAC00;
    let choseong_index = syllable_index / 588;
    let jungseong_index = (syllable_index % 588) / 28;
    let jongseong_index = syllable_index % 28;

    let choseongs = [
        'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ',
        'ㅌ', 'ㅍ', 'ㅎ',
    ];
    let jungseongs = [
        'ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅘ', 'ㅙ', 'ㅚ', 'ㅛ', 'ㅜ', 'ㅝ', 'ㅞ',
        'ㅟ', 'ㅠ', 'ㅡ', 'ㅢ', 'ㅣ',
    ];
    let jongseongs = [
        '\0', 'ㄱ', 'ㄲ', 'ㄳ', 'ㄴ', 'ㄵ', 'ㄶ', 'ㄷ', 'ㄹ', 'ㄺ', 'ㄻ', 'ㄼ', 'ㄽ', 'ㄾ', 'ㄿ', 'ㅀ',
        'ㅁ', 'ㅂ', 'ㅄ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
    ];

    let cho = choseongs[choseong_index as usize];
    let jung = jungseongs[jungseong_index as usize];
    let jong = if jongseong_index > 0 {
        Some(jongseongs[jongseong_index as usize])
    } else {
        None
    };

    Some((cho, jung, jong))
}

pub fn apply_elastic_end_sustain(word: &str, is_last_word: bool, hd: &HeuristicsData) -> f64 {
    if !is_last_word {
        return 1.0;
    }

    let syllables = split_into_syllables(word, &hd.syllables.vowels);
    if syllables.is_empty() {
        return 1.0;
    }

    let last_syl = &syllables[syllables.len() - 1];

    // Check if it is a Hangul syllable (BatchimBound)
    let last_char = last_syl.chars().last().unwrap_or('\0');
    if let Some((_cho, jung, jong)) = decompose_batchimbound(last_char) {
        if let Some(j) = jong {
            let j_str = j.to_string();
            if hd.elastic_end.sonorants.contains(&j_str) || hd.elastic_end.special_sonorant_endings.contains(&j_str) {
                return 1.15;
            }
        } else {
            let jung_str = jung.to_string();
            if hd.elastic_end.diphthongs.contains(&jung_str) {
                return 1.4;
            }
            if hd.elastic_end.vowels.contains(&jung_str) {
                return 1.25;
            }
        }
        return 1.0;
    }

    let cleaned_last_syl: String = last_syl
        .chars()
        .flat_map(|c| c.to_lowercase())
        .filter(|c| c.is_alphanumeric())
        .collect::<String>();

    if cleaned_last_syl.is_empty() {
        return 1.0;
    }

    // Check diphthongs
    for d in &hd.elastic_end.diphthongs {
        if cleaned_last_syl.ends_with(&d.to_lowercase()) {
            return 1.4;
        }
    }

    // Check vowels
    for v in &hd.elastic_end.vowels {
        if cleaned_last_syl.ends_with(&v.to_lowercase()) {
            return 1.25;
        }
    }

    // Check sonorants and special sonorant endings
    for s in &hd.elastic_end.sonorants {
        if cleaned_last_syl.ends_with(&s.to_lowercase()) {
            return 1.15;
        }
    }
    for se in &hd.elastic_end.special_sonorant_endings {
        if cleaned_last_syl.ends_with(&se.to_lowercase()) {
            return 1.15;
        }
    }

    1.0
}

pub fn apply_step_strider_fallback(word_text: &str, current_ms: i32, hd: &HeuristicsData) -> i32 {
    let base_duration = word_text.chars().count() as i32 * 150;
    let multiplier = apply_elastic_end_sustain(word_text, true, hd);

    let stretched_duration = (base_duration as f64 * multiplier) as i32;
    let min_limit = (300.0 * multiplier) as i32;
    let max_limit = (1200.0 * multiplier) as i32;

    let duration = std::cmp::min(max_limit, std::cmp::max(min_limit, stretched_duration));
    duration + current_ms
}

pub fn calculate_span_sizer_duration(words_count: i32, safe_duration: i32) -> i32 {
    let val_to_coerce = (safe_duration as f64 * 0.82) as i32;
    let min_limit = words_count * 300;
    let max_limit = words_count * 900;
    let coerced = std::cmp::max(min_limit, std::cmp::min(val_to_coerce, max_limit));

    std::cmp::min(coerced, std::cmp::max(safe_duration, words_count * 350))
}

pub fn apply_echo_pause_check(word1: &str, word2: &str) -> bool {
    let clean = |w: &str| {
        w.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase()
    };
    let cleaned1 = clean(word1);
    let cleaned2 = clean(word2);

    if cleaned1.is_empty() || cleaned2.is_empty() {
        return false;
    }
    cleaned1 == cleaned2
}

pub fn apply_double_oomph_weight(cleaned_word: &str, current_weight: i32, prefix: &str) -> i32 {
    if cleaned_word
        .to_lowercase()
        .starts_with(&prefix.to_lowercase())
    {
        (current_weight as f64 * 1.35) as i32
    } else {
        current_weight
    }
}

pub fn apply_rapid_comma_emphasis(
    word: &str,
    words_list: &[&str],
    idx: usize,
    current_weight: i32,
) -> i32 {
    if word.ends_with(',') {
        let mut has_second_comma_soon = false;
        let end_idx = std::cmp::min(idx + 4, words_list.len());
        for next_word in words_list.iter().take(end_idx).skip(idx + 1) {
            if next_word.ends_with(',') {
                has_second_comma_soon = true;
                break;
            }
        }
        if has_second_comma_soon {
            return (current_weight as f64 * 1.3) as i32;
        }
    }
    current_weight
}

pub fn apply_word_warden_cap(sub_word: &str, sub_word_start: i32, original_end: i32) -> i32 {
    let max_dur = std::cmp::min(sub_word.chars().count() as i32 * 65 + 200, 850);
    if (original_end - sub_word_start) > max_dur {
        sub_word_start + max_dur
    } else {
        original_end
    }
}
