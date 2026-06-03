use alignator::{heuristics, parse_lrc, WordInfo};

#[test]
fn test_python_beta_timing_fixture() {
    let lrc = "[00:00.00] Odd, tiny, commas, bloom...\n[00:03.40] <00:03.50> go <00:04.00> go";
    let lines = parse_lrc(lrc, "en");

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].timestamp_ms, 0);
    assert_eq!(lines[0].singing_duration_ms, 3400);
    assert_eq!(
        lines[0].words,
        vec![
            WordInfo {
                word: "Odd,".to_string(),
                start_ms: 0,
                end_ms: 288,
            },
            WordInfo {
                word: "tiny,".to_string(),
                start_ms: 491,
                end_ms: 972,
            },
            WordInfo {
                word: "commas,".to_string(),
                start_ms: 1175,
                end_ms: 1752,
            },
            WordInfo {
                word: "bloom...".to_string(),
                start_ms: 1955,
                end_ms: 2436,
            },
        ]
    );
    assert_eq!(
        lines[1].words,
        vec![
            WordInfo {
                word: "go".to_string(),
                start_ms: 3500,
                end_ms: 3830,
            },
            WordInfo {
                word: "go".to_string(),
                start_ms: 4000,
                end_ms: 4330,
            },
        ]
    );
}

#[test]
fn test_python_beta_truncation_helpers() {
    assert_eq!(heuristics::calculate_comma_cadence_pause(321.9), 160);
    assert_eq!(heuristics::calculate_span_sizer_duration(3, 1234), 1011);
    assert_eq!(heuristics::apply_double_oomph_weight("oooo", 4, "oo"), 5);
    assert_eq!(
        heuristics::apply_rapid_comma_emphasis("a,", &["a,", "b", "c,"], 0, 1),
        1
    );
}

#[test]
fn test_python_beta_gap_glide_fixture() {
    let lines = parse_lrc("[00:00.00] Hello world\n[00:08.00] Next line", "en");

    assert_eq!(lines.len(), 2);
    assert!(lines[0].is_gap_next);
    assert_eq!(lines[0].singing_duration_ms, 2500);
    assert_eq!(
        lines[0].words,
        vec![
            WordInfo {
                word: "Hello".to_string(),
                start_ms: 0,
                end_ms: 900,
            },
            WordInfo {
                word: "world".to_string(),
                start_ms: 900,
                end_ms: 1800,
            },
        ]
    );
    assert!(!lines[1].is_gap_next);
    assert_eq!(lines[1].singing_duration_ms, 5000);
}

#[test]
fn test_python_beta_braced_split_fixture() {
    let lines = parse_lrc(
        "[00:00.00] Lead (backing voice) tail\n[00:04.00] Next",
        "en",
    );

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].timestamp_ms, 0);
    assert_eq!(lines[0].text, "Lead tail");
    assert!(!lines[0].is_right_aligned);
    assert_eq!(lines[1].timestamp_ms, 656);
    assert_eq!(lines[1].text, "(backing voice)");
    assert!(lines[1].is_right_aligned);
    assert_eq!(lines[1].braced_text.as_deref(), Some("(backing voice)"));
    assert_eq!(
        lines[1].words,
        vec![
            WordInfo {
                word: "(backing".to_string(),
                start_ms: 656,
                end_ms: 1804,
            },
            WordInfo {
                word: "voice)".to_string(),
                start_ms: 1804,
                end_ms: 2624,
            },
        ]
    );
}

#[test]
fn test_python_beta_accented_latin_weighting_fixture() {
    let lines = parse_lrc("[00:00.00] Café naïve ooze\n[00:03.00] Next", "en");

    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0].words,
        vec![
            WordInfo {
                word: "Café".to_string(),
                start_ms: 0,
                end_ms: 656,
            },
            WordInfo {
                word: "naïve".to_string(),
                start_ms: 656,
                end_ms: 1476,
            },
            WordInfo {
                word: "ooze".to_string(),
                start_ms: 1476,
                end_ms: 2460,
            },
        ]
    );
}

#[test]
fn test_echoecho_and_flextrack_heuristics() {
    // 1. Test EchoEcho Similarity Tempo Propagation
    // Line 1 is standard, establishes a slow tempo for "Hello world hello world"
    // Line 2 is identical text but followed by a large gap (11 seconds).
    // Line 3 is a fast line that shifts the global RateRider tempo to be much faster.
    // With EchoEcho, Line 2 should propagate the slow tempo of Line 1 instead of using the global average.
    let lrc = "\
[00:00.00] Hello world hello world
[00:04.00] Hello world hello world
[00:15.00] Fast fast fast fast fast fast
[00:17.00] Next";
    let lines = parse_lrc(lrc, "en");
    
    // We expect Line 2 (which is lines[1]) to use the propagated tempo of Line 1.
    // Line 1 has a tempo around 546.6ms/syllable.
    // Under EchoEcho, Line 2's singing duration is calculated using Line 1's tempo:
    // 6 syllables * ~546ms/syllable + 800ms = ~4080ms.
    // Without EchoEcho, it would use the global RateRider average (~423ms/syllable), yielding ~3340ms.
    assert_eq!(lines[1].text, "Hello world hello world");
    assert!(lines[1].is_gap_next);
    assert!(lines[1].singing_duration_ms > 4000 && lines[1].singing_duration_ms < 4150);

    // 2. Test FlexTrack Capped Compression
    // A line with 9 words/syllables would normally have an active duration of 2700ms.
    // If it is followed by a next-line start at 1000ms, the deadline is 850ms.
    // Old mustCompress would force it to end at 850ms (a 68% compression).
    // FlexTrack caps compression at 15% (factor of 0.85), meaning the minimum duration is 2700 * 0.85 = 2295ms.
    let lrc_flextrack = "\
[00:00.00] Hello world hello world hello world hello world hello
[00:01.00] Next line";
    let lines_flextrack = parse_lrc(lrc_flextrack, "en");
    
    let words = &lines_flextrack[0].words;
    let duration = words[words.len() - 1].end_ms - words[0].start_ms;
    // The duration should be squished by exactly 15% (to 2291ms), allowing it to overflow past the next line's start (1000ms).
    assert_eq!(duration, 2291);

    // 3. Test FlexTrack Dynamic Strictness on Slow Tracks
    // Line 1 is a slow standard line (5 seconds) establishing a slow tempo of ~683ms/syllable.
    // Line 2 is a compressed line (1 second).
    // Because the tempo is slow (>= 500ms/syllable), FlexTrack should apply a stricter compression limit of 0.70.
    // Meaning the 1800ms duration should compress to 1800 * 0.70 = 1260ms.
    let lrc_slow = "\
[00:00.00] Slow slow slow slow slow slow
[00:05.00] Slow slow slow slow slow slow
[00:06.00] Next line";
    let lines_slow = parse_lrc(lrc_slow, "en");
    let words_slow = &lines_slow[1].words;
    let duration_slow = words_slow[words_slow.len() - 1].end_ms - words_slow[0].start_ms;
    assert_eq!(duration_slow, 1260);
}
