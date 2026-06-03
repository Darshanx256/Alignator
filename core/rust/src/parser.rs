#[derive(Debug, PartialEq)]
pub struct TagMatch {
    pub start_idx: usize,
    pub end_idx: usize,
    pub timestamp_ms: i32,
}

pub fn parse_timestamp_ms(s: &str) -> Option<i32> {
    let s = s.trim();
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let minutes: i64 = parts[0].parse().ok()?;
    if minutes < 0 {
        return None;
    }

    let sec_parts: Vec<&str> = parts[1].split('.').collect();
    if sec_parts.len() > 2 {
        return None;
    }
    let seconds: i64 = sec_parts[0].parse().ok()?;
    if !(0..60).contains(&seconds) {
        return None;
    }

    let mut fraction_ms: i64 = 0;
    if sec_parts.len() > 1 {
        let frac_str = sec_parts[1];
        if !frac_str.is_empty() {
            if !frac_str.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            let truncated: String = frac_str.chars().take(3).collect();
            let parsed_val: i64 = truncated.parse().ok()?;
            fraction_ms = match frac_str.len() {
                1 => parsed_val * 100,
                2 => parsed_val * 10,
                _ => parsed_val,
            };
        }
    }
    let total_seconds = minutes.checked_mul(60)?.checked_add(seconds)?;
    let total_ms = total_seconds.checked_mul(1000)?.checked_add(fraction_ms)?;
    i32::try_from(total_ms).ok()
}

pub fn find_word_tags(line_content: &str) -> Vec<TagMatch> {
    let mut matches = Vec::new();
    let chars: Vec<char> = line_content.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '>' {
                j += 1;
            }
            if j < chars.len() {
                let tag_content: String = chars[i + 1..j].iter().collect();
                if let Some(ms) = parse_timestamp_ms(&tag_content) {
                    let start_byte = line_content
                        .char_indices()
                        .nth(i)
                        .map(|(idx, _)| idx)
                        .unwrap_or(0);
                    let end_byte = line_content
                        .char_indices()
                        .nth(j)
                        .map(|(idx, _)| idx + 1)
                        .unwrap_or(0);
                    matches.push(TagMatch {
                        start_idx: start_byte,
                        end_idx: end_byte,
                        timestamp_ms: ms,
                    });
                }
                i = j;
            }
        }
        i += 1;
    }
    matches
}

pub fn clean_whitespace(s: &str) -> String {
    let mut result = String::new();
    let mut last_was_space = false;
    for c in s.trim().chars() {
        if c.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(c);
            last_was_space = false;
        }
    }
    result
}

pub fn is_moracore_kana(c: char) -> bool {
    matches!(c, '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}' | '\u{31F0}'..='\u{31FF}' | '\u{FF65}'..='\u{FF9F}')
}

pub fn is_moracore_non_starter(c: char) -> bool {
    "ぁぃぅぇぉゃゅょゎァィゥェォャュョヮ".contains(c)
}

pub fn is_moracore_mora_extender(c: char) -> bool {
    c == 'ー'
}

pub fn is_batchimbound_syllable(c: char) -> bool {
    matches!(c, '\u{AC00}'..='\u{D7A3}')
}

pub fn split_into_syllables(word: &str, vowels: &str) -> Vec<String> {
    let chars: Vec<char> = word.chars().collect();
    if chars.is_empty() {
        return Vec::new();
    }

    // Check if word contains Hangul (BatchimBound)
    let has_batchimbound = chars.iter().any(|&c| is_batchimbound_syllable(c));
    if has_batchimbound {
        let mut syllables: Vec<String> = Vec::new();
        for &c in &chars {
            if is_batchimbound_syllable(c) {
                syllables.push(c.to_string());
            } else if c.is_alphanumeric() {
                syllables.push(c.to_string());
            }
        }
        return syllables;
    }

    // Check if word contains Japanese Kana (MoraCore)
    let has_moracore = chars.iter().any(|&c| is_moracore_kana(c));
    if has_moracore {
        let mut syllables: Vec<String> = Vec::new();
        for &c in &chars {
            if is_moracore_non_starter(c) {
                if let Some(last) = syllables.last_mut() {
                    last.push(c);
                } else {
                    syllables.push(c.to_string());
                }
            } else if is_moracore_kana(c) || is_moracore_mora_extender(c) || matches!(c, '\u{4E00}'..='\u{9FFF}') {
                syllables.push(c.to_string());
            } else if c.is_alphanumeric() {
                syllables.push(c.to_string());
            }
        }
        return syllables;
    }

    // Standard English/Latin vowel-based syllable splitting
    if chars.len() <= 3 {
        return vec![word.to_string()];
    }

    let mut syllables = Vec::new();
    let mut current = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        current.push(c);
        let is_vowel = vowels.contains(c);

        if is_vowel && i + 1 < chars.len() {
            let next_char = chars[i + 1];
            if !vowels.contains(next_char) {
                if i + 2 < chars.len() && vowels.contains(chars[i + 2]) {
                    syllables.push(current.iter().collect::<String>());
                    current.clear();
                } else if i + 3 < chars.len()
                    && !vowels.contains(chars[i + 2])
                    && vowels.contains(chars[i + 3])
                {
                    current.push(next_char);
                    i += 1;
                    syllables.push(current.iter().collect::<String>());
                    current.clear();
                }
            }
        }
        i += 1;
    }

    if !current.is_empty() {
        syllables.push(current.iter().collect::<String>());
    }

    let rejoined: String = syllables.concat();
    if rejoined != word {
        return vec![word.to_string()];
    }
    syllables
}

pub fn count_line_syllables(text: &str, vowels: &str) -> i32 {
    let mut clean_text = String::new();
    let mut in_tag = false;
    for c in text.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            clean_text.push(c);
        }
    }

    let mut total = 0;
    let mut current_word = String::new();
    for c in clean_text.chars() {
        if c.is_alphanumeric() || c == '_' || c == 'ー' {
            current_word.push(c);
        } else {
            if !current_word.is_empty() {
                total += split_into_syllables(&current_word, vowels).len();
                current_word.clear();
            }
        }
    }
    if !current_word.is_empty() {
        total += split_into_syllables(&current_word, vowels).len();
    }

    std::cmp::max(1, total as i32)
}
