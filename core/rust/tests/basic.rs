use alignator::{parse_lrc, parser};

#[test]
fn test_parse_lrc_basic() {
    let lrc = "[00:02.00] Hello, world!\n[00:05.00] This is a test.";
    let lines = parse_lrc(lrc, "en");

    assert_eq!(lines.len(), 2);

    // Check line 1
    assert_eq!(lines[0].timestamp_ms, 2000);
    assert_eq!(lines[0].text, "Hello, world!");
    assert_eq!(lines[0].words.len(), 2);
    assert_eq!(lines[0].words[0].word, "Hello,");

    // Check line 2
    assert_eq!(lines[1].timestamp_ms, 5000);
    assert_eq!(lines[1].text, "This is a test.");
}

#[test]
fn test_parse_lrc_with_word_tags() {
    let lrc = "[00:02.00] <00:02.10> Hello <00:02.80> world!";
    let lines = parse_lrc(lrc, "en");

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].words.len(), 2);
    assert_eq!(lines[0].words[0].word, "Hello");
    assert_eq!(lines[0].words[0].start_ms, 2100);
    assert_eq!(lines[0].words[1].word, "world!");
    assert_eq!(lines[0].words[1].start_ms, 2800);
}

#[test]
fn test_parse_lrc_expands_multiple_line_timestamps() {
    let lrc = "[00:01.00][00:02.00]Echo line\n[00:03.00]Next";
    let lines = parse_lrc(lrc, "en");

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].timestamp_ms, 1000);
    assert_eq!(lines[0].text, "Echo line");
    assert_eq!(lines[1].timestamp_ms, 2000);
    assert_eq!(lines[1].text, "Echo line");
    assert_eq!(lines[2].timestamp_ms, 3000);
    assert_eq!(lines[2].text, "Next");
}

#[test]
fn test_invalid_timestamps_are_ignored() {
    assert_eq!(parser::parse_timestamp_ms("-01:10.00"), None);
    assert_eq!(parser::parse_timestamp_ms("00:60.00"), None);
    assert_eq!(parser::parse_timestamp_ms("999999999999:00.00"), None);

    let lrc =
        "[00:01.00]Valid\n[-01:10.00]Negative\n[00:60.00]Bad seconds\n[999999999999:00.00]Huge";
    let lines = parse_lrc(lrc, "en");

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].timestamp_ms, 1000);
    assert_eq!(lines[0].text, "Valid");
}
