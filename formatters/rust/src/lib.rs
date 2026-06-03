use alignator::Line;
use serde::Serialize;

fn format_ms_to_ttml(ms: i32) -> String {
    let total_seconds = ms / 1000;
    let milliseconds = ms % 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        hours, minutes, seconds, milliseconds
    )
}

fn format_ms_to_lrc(ms: i32) -> String {
    let total_seconds = ms / 1000;
    let centiseconds = (ms % 1000) / 10;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}.{:02}", minutes, seconds, centiseconds)
}

/// Formats the aligned lines into Apple Music compatible TTML (XML).
pub fn to_apple_music_ttml(lines: &[Line]) -> String {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    xml.push_str("<tt xmlns=\"http://www.w3.org/ns/ttml\" xmlns:ttm=\"http://www.w3.org/ns/ttml#metadata\" xmlns:tts=\"http://www.w3.org/ns/ttml#styling\" xml:lang=\"en\">\n");
    xml.push_str("  <head>\n");
    xml.push_str("    <metadata>\n");
    xml.push_str("      <ttm:title>Aligned Lyrics</ttm:title>\n");
    xml.push_str("    </metadata>\n");
    xml.push_str("  </head>\n");
    xml.push_str("  <body>\n");
    xml.push_str("    <div>\n");

    for line in lines {
        let line_start = format_ms_to_ttml(line.timestamp_ms);
        let line_end = if !line.words.is_empty() {
            format_ms_to_ttml(line.words[line.words.len() - 1].end_ms)
        } else {
            format_ms_to_ttml(line.timestamp_ms + line.singing_duration_ms)
        };

        xml.push_str(&format!(
            "      <p begin=\"{}\" end=\"{}\">\n",
            line_start, line_end
        ));

        for w in &line.words {
            let w_start = format_ms_to_ttml(w.start_ms - line.timestamp_ms);
            let w_end = format_ms_to_ttml(w.end_ms - line.timestamp_ms);
            // Escape XML entities
            let escaped_word = w
                .word
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&apos;");
            xml.push_str(&format!(
                "        <span begin=\"{}\" end=\"{}\">{}</span>\n",
                w_start, w_end, escaped_word
            ));
        }

        xml.push_str("      </p>\n");
    }

    xml.push_str("    </div>\n");
    xml.push_str("  </body>\n");
    xml.push_str("</tt>\n");
    xml
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifySyllable {
    pub text: String,
    pub start_time_ms: i32,
    pub end_time_ms: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyLine {
    pub start_time_ms: String,
    pub words: String,
    pub syllables: Vec<SpotifySyllable>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyLyrics {
    pub lines: Vec<SpotifyLine>,
}

/// Formats the aligned lines into Spotify-style JSON lyrics.
pub fn to_spotify_json(lines: &[Line]) -> String {
    let mut spotify_lines = Vec::new();
    for line in lines {
        let syllables = line
            .words
            .iter()
            .map(|w| SpotifySyllable {
                text: w.word.clone(),
                start_time_ms: w.start_ms,
                end_time_ms: w.end_ms,
            })
            .collect();

        spotify_lines.push(SpotifyLine {
            start_time_ms: line.timestamp_ms.to_string(),
            words: line.text.clone(),
            syllables,
        });
    }

    let lyrics = SpotifyLyrics {
        lines: spotify_lines,
    };
    serde_json::to_string_pretty(&lyrics).unwrap_or_default()
}

/// Formats the aligned lines into Enhanced LRC text.
pub fn to_enhanced_lrc(lines: &[Line]) -> String {
    let mut lrc = String::new();
    for line in lines {
        lrc.push_str(&format!("[{}]", format_ms_to_lrc(line.timestamp_ms)));
        for w in &line.words {
            lrc.push_str(&format!(" <{}>{}", format_ms_to_lrc(w.start_ms), w.word));
        }
        lrc.push('\n');
    }
    lrc
}

#[cfg(test)]
mod tests {
    use super::*;
    use alignator::WordInfo;

    fn sample_line() -> Line {
        Line {
            timestamp_ms: 10_000,
            text: "Hello & world".to_string(),
            words: vec![
                WordInfo {
                    word: "Hello".to_string(),
                    start_ms: 10_000,
                    end_ms: 10_500,
                },
                WordInfo {
                    word: "&".to_string(),
                    start_ms: 10_500,
                    end_ms: 10_700,
                },
                WordInfo {
                    word: "world".to_string(),
                    start_ms: 10_700,
                    end_ms: 11_200,
                },
            ],
            is_right_aligned: false,
            braced_text: None,
            singing_duration_ms: 1_200,
            is_gap_next: false,
        }
    }

    #[test]
    fn ttml_uses_absolute_paragraph_and_relative_span_timing() {
        let xml = to_apple_music_ttml(&[sample_line()]);

        assert!(xml.contains("<p begin=\"00:00:10.000\" end=\"00:00:11.200\">"));
        assert!(xml.contains("<span begin=\"00:00:00.000\" end=\"00:00:00.500\">Hello</span>"));
        assert!(xml.contains("<span begin=\"00:00:00.500\" end=\"00:00:00.700\">&amp;</span>"));
    }

    #[test]
    fn enhanced_lrc_keeps_absolute_word_timing() {
        let lrc = to_enhanced_lrc(&[sample_line()]);

        assert_eq!(
            lrc,
            "[00:10.00] <00:10.00>Hello <00:10.50>& <00:10.70>world\n"
        );
    }

    #[test]
    fn spotify_json_keeps_existing_wire_shape() {
        let json = to_spotify_json(&[sample_line()]);

        assert!(json.contains("\"startTimeMs\": \"10000\""));
        assert!(json.contains("\"words\": \"Hello & world\""));
        assert!(json.contains("\"startTimeMs\": 10500"));
        assert!(json.contains("\"text\": \"&\""));
    }

    #[test]
    fn ttml_escapes_all_xml_sensitive_word_text() {
        let mut line = sample_line();
        line.text = "<tag> \"quote\" 'apos' &".to_string();
        line.words = vec![WordInfo {
            word: "<tag>\"'&".to_string(),
            start_ms: 10_000,
            end_ms: 10_500,
        }];

        let xml = to_apple_music_ttml(&[line]);

        assert!(xml.contains("&lt;tag&gt;&quot;&apos;&amp;"));
    }

    #[test]
    fn ttml_empty_line_uses_singing_duration_for_end() {
        let mut line = sample_line();
        line.text = String::new();
        line.words.clear();
        line.singing_duration_ms = 2_345;

        let xml = to_apple_music_ttml(&[line]);

        assert!(xml.contains("<p begin=\"00:00:10.000\" end=\"00:00:12.345\">"));
    }
}
