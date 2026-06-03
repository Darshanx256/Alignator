uniffi::setup_scaffolding!();

#[derive(uniffi::Record, Clone)]
pub struct UdiWordInfo {
    pub word: String,
    pub start_ms: i32,
    pub end_ms: i32,
}

#[derive(uniffi::Record, Clone)]
pub struct UdiLine {
    pub timestamp_ms: i32,
    pub text: String,
    pub words: Vec<UdiWordInfo>,
    pub is_right_aligned: bool,
    pub braced_text: Option<String>,
    pub singing_duration_ms: i32,
    pub is_gap_next: bool,
}

#[uniffi::export]
pub fn parse_lrc(lrc_text: String, language_mode: String) -> Vec<UdiLine> {
    let lines = alignator::parse_lrc(&lrc_text, &language_mode);
    lines
        .into_iter()
        .map(|l| UdiLine {
            timestamp_ms: l.timestamp_ms,
            text: l.text,
            words: l
                .words
                .into_iter()
                .map(|w| UdiWordInfo {
                    word: w.word,
                    start_ms: w.start_ms,
                    end_ms: w.end_ms,
                })
                .collect(),
            is_right_aligned: l.is_right_aligned,
            braced_text: l.braced_text,
            singing_duration_ms: l.singing_duration_ms,
            is_gap_next: l.is_gap_next,
        })
        .collect()
}

#[uniffi::export]
pub fn to_apple_music_ttml(lines: Vec<UdiLine>) -> String {
    let core_lines: Vec<alignator::Line> = lines
        .into_iter()
        .map(|l| alignator::Line {
            timestamp_ms: l.timestamp_ms,
            text: l.text,
            words: l
                .words
                .into_iter()
                .map(|w| alignator::WordInfo {
                    word: w.word,
                    start_ms: w.start_ms,
                    end_ms: w.end_ms,
                })
                .collect(),
            is_right_aligned: l.is_right_aligned,
            braced_text: l.braced_text,
            singing_duration_ms: l.singing_duration_ms,
            is_gap_next: l.is_gap_next,
        })
        .collect();
    alignator_formatters::to_apple_music_ttml(&core_lines)
}

#[uniffi::export]
pub fn to_spotify_json(lines: Vec<UdiLine>) -> String {
    let core_lines: Vec<alignator::Line> = lines
        .into_iter()
        .map(|l| alignator::Line {
            timestamp_ms: l.timestamp_ms,
            text: l.text,
            words: l
                .words
                .into_iter()
                .map(|w| alignator::WordInfo {
                    word: w.word,
                    start_ms: w.start_ms,
                    end_ms: w.end_ms,
                })
                .collect(),
            is_right_aligned: l.is_right_aligned,
            braced_text: l.braced_text,
            singing_duration_ms: l.singing_duration_ms,
            is_gap_next: l.is_gap_next,
        })
        .collect();
    alignator_formatters::to_spotify_json(&core_lines)
}

#[uniffi::export]
pub fn to_enhanced_lrc(lines: Vec<UdiLine>) -> String {
    let core_lines: Vec<alignator::Line> = lines
        .into_iter()
        .map(|l| alignator::Line {
            timestamp_ms: l.timestamp_ms,
            text: l.text,
            words: l
                .words
                .into_iter()
                .map(|w| alignator::WordInfo {
                    word: w.word,
                    start_ms: w.start_ms,
                    end_ms: w.end_ms,
                })
                .collect(),
            is_right_aligned: l.is_right_aligned,
            braced_text: l.braced_text,
            singing_duration_ms: l.singing_duration_ms,
            is_gap_next: l.is_gap_next,
        })
        .collect();
    alignator_formatters::to_enhanced_lrc(&core_lines)
}
