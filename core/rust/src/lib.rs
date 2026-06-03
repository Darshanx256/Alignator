pub mod heuristics;
pub mod language;
pub mod parser;
pub mod tempo;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WordInfo {
    pub word: String,
    pub start_ms: i32,
    pub end_ms: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Line {
    pub timestamp_ms: i32,
    pub text: String,
    pub words: Vec<WordInfo>,
    pub is_right_aligned: bool,
    pub braced_text: Option<String>,
    pub singing_duration_ms: i32,
    pub is_gap_next: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedLrcLine {
    pub timestamp_ms: i32,
    pub text: String,
}

#[derive(Debug)]
pub struct RateRider {
    pub ms_per_syllable: f64,
}

impl RateRider {
    pub fn new(sorted_lines: &[ParsedLrcLine], hd: &language::HeuristicsData) -> Self {
        let mut total_syllables = 0;
        let mut total_duration = 0;

        for idx in 0..sorted_lines.len() {
            if idx < sorted_lines.len() - 1 {
                let duration = sorted_lines[idx + 1].timestamp_ms - sorted_lines[idx].timestamp_ms;
                if duration > 6000 {
                    continue;
                }

                let mut words = parse_line_to_words(
                    sorted_lines[idx].timestamp_ms,
                    &sorted_lines[idx].text,
                    duration,
                    320.0,
                    hd,
                );

                if !words.is_empty() {
                    tempo::apply_flextrack_compression(
                        &mut words,
                        duration,
                        sorted_lines[idx].timestamp_ms,
                        320.0,
                    );
                    let actual_dur = words[words.len() - 1].end_ms - words[0].start_ms;
                    total_duration += actual_dur;
                    total_syllables +=
                        parser::count_line_syllables(&sorted_lines[idx].text, &hd.syllables.vowels);
                }
            }
        }

        let ms_per_syllable = if total_syllables > 0 && total_duration > 0 {
            total_duration as f64 / total_syllables as f64
        } else {
            320.0
        };

        RateRider { ms_per_syllable }
    }

    pub fn estimate_duration(&self, text: &str, vowels: &str) -> f64 {
        let syllables = parser::count_line_syllables(text, vowels);
        syllables as f64 * self.ms_per_syllable
    }
}

pub fn parse_line_to_words(
    line_timestamp_ms: i32,
    line_content: &str,
    line_duration_ms: i32,
    ms_per_syllable: f64,
    hd: &language::HeuristicsData,
) -> Vec<WordInfo> {
    let matches = parser::find_word_tags(line_content);

    // Case A: No word-level tags (Standard LRC)
    if matches.is_empty() {
        let raw_text_cleaned = parser::clean_whitespace(line_content);
        let words_owned: Vec<String> = if raw_text_cleaned.chars().any(|c| {
            parser::is_moracore_kana(c)
                || parser::is_moracore_mora_extender(c)
                || matches!(c, '\u{4E00}'..='\u{9FFF}')
        }) {
            let mut words = Vec::new();
            let mut current = String::new();

            for c in raw_text_cleaned.chars() {
                if c.is_whitespace() || c == '　' {
                    if !current.is_empty() {
                        words.push(current.clone());
                        current.clear();
                    }
                    continue;
                }

                if c == '、' || c == '。' || c == '？' || c == '！' || c == ',' || c == '.' || c == '?' || c == '!' {
                    if !current.is_empty() {
                        current.push(c);
                        words.push(current.clone());
                        current.clear();
                    } else {
                        words.push(c.to_string());
                    }
                    continue;
                }

                let is_kana_or_kanji = parser::is_moracore_kana(c)
                    || parser::is_moracore_mora_extender(c)
                    || matches!(c, '\u{4E00}'..='\u{9FFF}');

                if is_kana_or_kanji {
                    if parser::is_moracore_non_starter(c) {
                        if !current.is_empty() {
                            current.push(c);
                        } else if let Some(last) = words.last_mut() {
                            last.push(c);
                        } else {
                            current.push(c);
                        }
                    } else {
                        if !current.is_empty() {
                            words.push(current.clone());
                            current.clear();
                        }
                        current.push(c);
                    }
                } else {
                    let current_is_cjk = current.chars().next().map_or(false, |first_char| {
                        parser::is_moracore_kana(first_char)
                            || parser::is_moracore_mora_extender(first_char)
                            || matches!(first_char, '\u{4E00}'..='\u{9FFF}')
                    });
                    if current_is_cjk {
                        words.push(current.clone());
                        current.clear();
                    }
                    current.push(c);
                }
            }

            if !current.is_empty() {
                words.push(current);
            }
            words
        } else {
            raw_text_cleaned.split_whitespace().map(|s| s.to_string()).collect()
        };
        let words_raw: Vec<&str> = words_owned.iter().map(|s| s.as_str()).collect();

        if words_raw.is_empty() {
            return Vec::new();
        }

        let safe_duration = std::cmp::max(400, line_duration_ms);
        let active_duration =
            heuristics::calculate_span_sizer_duration(words_raw.len() as i32, safe_duration);

        let mut pauses = Vec::new();
        for (i, &w) in words_raw.iter().enumerate() {
            let mut pause = heuristics::calculate_punctuation_pause(w, ms_per_syllable);
            if i < words_raw.len() - 1 {
                let next_w = words_raw[i + 1];
                if heuristics::apply_echo_pause_check(w, next_w) {
                    pause += 100;
                }
            }
            pauses.push(pause);
        }
        let total_pause_ms: i32 = pauses.iter().sum();

        let available_active_duration = std::cmp::max(
            active_duration - total_pause_ms,
            words_raw.len() as i32 * 120,
        );

        let mut word_weights = Vec::new();
        for (i, &w) in words_raw.iter().enumerate() {
            let cleaned: String = w.chars().filter(|c| c.is_alphanumeric()).collect();
            let mut weight = std::cmp::max(1, cleaned.chars().count() as i32);

            weight =
                heuristics::apply_double_oomph_weight(&cleaned, weight, &hd.double_oomph_prefix);
            weight = heuristics::apply_rapid_comma_emphasis(w, &words_raw, i, weight);

            if i == words_raw.len() - 1 {
                weight =
                    (weight as f64 * heuristics::apply_elastic_end_sustain(w, true, hd)) as i32;
            }
            word_weights.push(weight);
        }

        let total_weight: i32 = word_weights.iter().sum();
        let mut word_infos = Vec::new();
        let mut current_offset = 0;

        for (i, &word) in words_raw.iter().enumerate() {
            let weight = word_weights[i];
            let duration = if total_weight > 0 {
                (weight * available_active_duration) / total_weight
            } else {
                200
            };

            let start_ms = line_timestamp_ms + current_offset;
            let end_ms = start_ms + duration;
            word_infos.push(WordInfo {
                word: word.to_string(),
                start_ms,
                end_ms,
            });
            current_offset += duration + pauses[i];
        }

        return word_infos;
    }

    // Case B: Enhanced LRC with word-level tags
    let mut word_infos = Vec::new();

    let first_match = &matches[0];
    if first_match.start_idx > 0 {
        let leading_text = line_content[..first_match.start_idx].trim();
        if !leading_text.is_empty() {
            let first_tag_ms = first_match.timestamp_ms;
            let words: Vec<&str> = leading_text.split_whitespace().collect();
            if !words.is_empty() {
                let duration = first_tag_ms - line_timestamp_ms;
                let step = if duration > 0 {
                    duration / words.len() as i32
                } else {
                    100
                };
                for (j, &word) in words.iter().enumerate() {
                    word_infos.push(WordInfo {
                        word: word.to_string(),
                        start_ms: line_timestamp_ms + (j as i32 * step),
                        end_ms: line_timestamp_ms + ((j as i32 + 1) * step),
                    });
                }
            }
        }
    }

    for (i, current_match) in matches.iter().enumerate() {
        let current_ms = current_match.timestamp_ms;
        let start_of_text = current_match.end_idx;
        let end_of_text = if i < matches.len() - 1 {
            matches[i + 1].start_idx
        } else {
            line_content.len()
        };

        let word_text = line_content[start_of_text..end_of_text].trim();
        if word_text.is_empty() {
            continue;
        }

        let next_ms = if i < matches.len() - 1 {
            matches[i + 1].timestamp_ms
        } else {
            heuristics::apply_step_strider_fallback(word_text, current_ms, hd)
        };

        let sub_words: Vec<&str> = word_text.split_whitespace().collect();
        if !sub_words.is_empty() {
            let total_duration = next_ms - current_ms;
            let step = if total_duration > 0 {
                total_duration / sub_words.len() as i32
            } else {
                100
            };
            for (j, &sub_word) in sub_words.iter().enumerate() {
                let sub_word_start = current_ms + (j as i32 * step);
                let original_end = current_ms + ((j as i32 + 1) * step);

                let final_end =
                    heuristics::apply_word_warden_cap(sub_word, sub_word_start, original_end);
                word_infos.push(WordInfo {
                    word: sub_word.to_string(),
                    start_ms: sub_word_start,
                    end_ms: final_end,
                });
            }
        }
    }

    word_infos.sort_by_key(|w| w.start_ms);

    if word_infos.len() >= 2 {
        for i in 0..(word_infos.len() - 1) {
            let (left, right) = word_infos.split_at_mut(i + 1);
            let w1 = &mut left[i];
            let w2 = &right[0];
            if heuristics::apply_echo_pause_check(&w1.word, &w2.word) {
                let current_gap = w2.start_ms - w1.end_ms;
                if current_gap < 100 {
                    let duration = w1.end_ms - w1.start_ms;
                    let max_shorten = duration - 100;
                    if max_shorten > 0 {
                        let shorten_by = std::cmp::min(100 - current_gap, max_shorten);
                        w1.end_ms -= shorten_by;
                    }
                }
            }
        }
    }

    word_infos
}

pub fn split_line_if_braced(line: Line) -> Vec<Line> {
    if line.words.is_empty() {
        return vec![line];
    }

    let mut main_words = Vec::new();
    let mut braced_words = Vec::new();
    let mut in_braces = false;

    for w_info in &line.words {
        let word_text = &w_info.word;
        if word_text.starts_with('(') {
            in_braces = true;
        }

        if in_braces {
            braced_words.push(w_info.clone());
        } else {
            main_words.push(w_info.clone());
        }

        if word_text.ends_with(')') {
            in_braces = false;
        }
    }

    if braced_words.is_empty() || main_words.is_empty() {
        return vec![line];
    }

    let main_text = main_words
        .iter()
        .map(|w| w.word.as_str())
        .collect::<Vec<&str>>()
        .join(" ");
    let braced_text = braced_words
        .iter()
        .map(|w| w.word.as_str())
        .collect::<Vec<&str>>()
        .join(" ");

    let main_line = Line {
        timestamp_ms: main_words[0].start_ms,
        text: main_text,
        words: main_words,
        is_right_aligned: false,
        braced_text: None,
        singing_duration_ms: line.singing_duration_ms,
        is_gap_next: line.is_gap_next,
    };

    let braced_line = Line {
        timestamp_ms: braced_words[0].start_ms,
        text: braced_text.clone(),
        words: braced_words,
        is_right_aligned: true,
        braced_text: Some(braced_text),
        singing_duration_ms: line.singing_duration_ms,
        is_gap_next: line.is_gap_next,
    };

    vec![main_line, braced_line]
}

fn parse_lrc_source_line(line: &str) -> Vec<ParsedLrcLine> {
    let mut remaining = line.trim();
    let mut timestamps = Vec::new();

    while let Some(after_open) = remaining.strip_prefix('[') {
        let Some(end_bracket_idx) = after_open.find(']') else {
            break;
        };
        let time_part = &after_open[..end_bracket_idx];
        if let Some(timestamp_ms) = parser::parse_timestamp_ms(time_part) {
            timestamps.push(timestamp_ms);
        }
        remaining = &after_open[end_bracket_idx + 1..];
    }

    let text = remaining.trim();
    timestamps
        .into_iter()
        .map(|timestamp_ms| ParsedLrcLine {
            timestamp_ms,
            text: text.to_string(),
        })
        .collect()
}

struct StandardLineInfo {
    text: String,
    tempo: f64,
}

pub fn parse_lrc(lrc_text: &str, language_mode: &str) -> Vec<Line> {
    let dominant_lang = if language_mode == "auto" {
        language::detect_language(lrc_text)
    } else {
        language_mode.to_string()
    };

    let song_hd = language::resolve_heuristics(&dominant_lang);

    let mut parsed_lines = Vec::new();
    for line in lrc_text.lines() {
        parsed_lines.extend(parse_lrc_source_line(line));
    }

    parsed_lines.sort_by_key(|l| l.timestamp_ms);

    let rate_rider = RateRider::new(&parsed_lines, &song_hd);

    // --- PASS 1: Dry run to build the EchoEcho standard lines database ---
    let mut standard_lines = Vec::new();
    for idx in 0..parsed_lines.len() {
        let current = &parsed_lines[idx];
        let line_duration_ms = if idx < parsed_lines.len() - 1 {
            parsed_lines[idx + 1].timestamp_ms - current.timestamp_ms
        } else {
            5000
        };

        let line_hd = if language_mode == "auto" {
            let mut text_without_tags = String::new();
            let mut in_tag = false;
            for c in current.text.chars() {
                if c == '<' {
                    in_tag = true;
                } else if c == '>' {
                    in_tag = false;
                } else if !in_tag {
                    text_without_tags.push(c);
                }
            }
            let line_detected = language::detect_language(&text_without_tags);
            language::resolve_heuristics(&line_detected)
        } else {
            song_hd.clone()
        };

        let est_singing_ms =
            (rate_rider.estimate_duration(&current.text, &line_hd.syllables.vowels) + 800.0) as i32;
        let is_gap_next = tempo::apply_gap_sentry_check(line_duration_ms, est_singing_ms);
        let singing_duration_ms = if is_gap_next {
            tempo::apply_gap_glide_estimation(
                &current.text,
                line_duration_ms,
                rate_rider.ms_per_syllable,
                &line_hd.syllables.vowels,
            )
        } else {
            line_duration_ms
        };

        let mut words = parse_line_to_words(
            current.timestamp_ms,
            &current.text,
            singing_duration_ms,
            rate_rider.ms_per_syllable,
            &line_hd,
        );

        tempo::apply_flextrack_compression(
            &mut words,
            singing_duration_ms,
            current.timestamp_ms,
            rate_rider.ms_per_syllable,
        );

        if !is_gap_next && !words.is_empty() {
            let original_start = words[0].start_ms;
            let original_end = words[words.len() - 1].end_ms;
            let deadline_ms = current.timestamp_ms + line_duration_ms - 150;
            if original_end <= deadline_ms {
                let syllables = parser::count_line_syllables(&current.text, &line_hd.syllables.vowels);
                if syllables > 0 {
                    let actual_duration = original_end - original_start;
                    let tempo = actual_duration as f64 / syllables as f64;
                    standard_lines.push(StandardLineInfo {
                        text: current.text.clone(),
                        tempo,
                    });
                }
            }
        }
    }

    // --- PASS 2: Final parsing using similarity timing propagation (EchoEcho) and global tempo ---
    let mut final_lines = Vec::new();
    for idx in 0..parsed_lines.len() {
        let current = &parsed_lines[idx];
        let line_duration_ms = if idx < parsed_lines.len() - 1 {
            parsed_lines[idx + 1].timestamp_ms - current.timestamp_ms
        } else {
            5000
        };

        let line_hd = if language_mode == "auto" {
            let mut text_without_tags = String::new();
            let mut in_tag = false;
            for c in current.text.chars() {
                if c == '<' {
                    in_tag = true;
                } else if c == '>' {
                    in_tag = false;
                } else if !in_tag {
                    text_without_tags.push(c);
                }
            }
            let line_detected = language::detect_language(&text_without_tags);
            language::resolve_heuristics(&line_detected)
        } else {
            song_hd.clone()
        };

        // EchoEcho: Find similar standard line to propagate its singing speed
        let mut propagated_tempo = None;
        let mut best_similarity = 0.0;
        for std_line in &standard_lines {
            let sim = tempo::calculate_echo_echo_similarity(&current.text, &std_line.text);
            if sim >= 0.70 && sim > best_similarity {
                best_similarity = sim;
                propagated_tempo = Some(std_line.tempo);
            }
        }

        let line_ms_per_syllable = propagated_tempo.unwrap_or(rate_rider.ms_per_syllable);
        let syllables = parser::count_line_syllables(&current.text, &line_hd.syllables.vowels);

        let est_singing_ms = if let Some(t) = propagated_tempo {
            (syllables as f64 * t + 800.0) as i32
        } else {
            (rate_rider.estimate_duration(&current.text, &line_hd.syllables.vowels) + 800.0) as i32
        };

        let is_gap_next = tempo::apply_gap_sentry_check(line_duration_ms, est_singing_ms);
        let singing_duration_ms = if is_gap_next {
            tempo::apply_gap_glide_estimation(
                &current.text,
                line_duration_ms,
                line_ms_per_syllable,
                &line_hd.syllables.vowels,
            )
        } else {
            line_duration_ms
        };

        let mut clean_text = String::new();
        let mut in_tag = false;
        for c in current.text.chars() {
            if c == '<' {
                in_tag = true;
            } else if c == '>' {
                in_tag = false;
            } else if !in_tag {
                clean_text.push(c);
            }
        }
        let clean_text = parser::clean_whitespace(&clean_text);

        let mut words = parse_line_to_words(
            current.timestamp_ms,
            &current.text,
            singing_duration_ms,
            line_ms_per_syllable,
            &line_hd,
        );

        tempo::apply_flextrack_compression(
            &mut words,
            singing_duration_ms,
            current.timestamp_ms,
            line_ms_per_syllable,
        );

        let initial_line = Line {
            timestamp_ms: current.timestamp_ms,
            text: clean_text,
            words,
            is_right_aligned: false,
            braced_text: None,
            singing_duration_ms,
            is_gap_next,
        };

        let split_lines = split_line_if_braced(initial_line);
        final_lines.extend(split_lines);
    }

    final_lines.sort_by_key(|l| l.timestamp_ms);
    final_lines
}


