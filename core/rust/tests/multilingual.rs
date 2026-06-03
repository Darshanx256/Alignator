use alignator::{heuristics, language, parser};

#[test]
fn test_moracore_japanese_mora_counting() {
    let hd = language::resolve_heuristics("japanese");

    // Test yōon (small kana) fusion: "きょう" has 3 characters, but fuses to 2 morae
    let syllables_kyo = parser::split_into_syllables("きょう", &hd.syllables.vowels);
    assert_eq!(syllables_kyo, vec!["きょ", "う"]);
    assert_eq!(parser::count_line_syllables("きょう", &hd.syllables.vowels), 2);

    // Test standard morae: "ありがとう" has 5 morae
    let syllables_ari = parser::split_into_syllables("ありがとう", &hd.syllables.vowels);
    assert_eq!(syllables_ari, vec!["あ", "り", "が", "と", "う"]);
    assert_eq!(parser::count_line_syllables("ありがとう", &hd.syllables.vowels), 5);

    // Test Katakana long vowel mark: "ラーメン" has 4 morae
    let syllables_ram = parser::split_into_syllables("ラーメン", &hd.syllables.vowels);
    assert_eq!(syllables_ram, vec!["ラ", "ー", "メ", "ン"]);
    assert_eq!(parser::count_line_syllables("ラーメン", &hd.syllables.vowels), 4);
}

#[test]
fn test_moracore_japanese_elastic_end_sustain() {
    let hd = language::resolve_heuristics("japanese");

    // "う" is a vowel, should receive vowel sustain (1.25x)
    let sustain_vowel = heuristics::apply_elastic_end_sustain("ありがとう", true, &hd);
    assert_eq!(sustain_vowel, 1.25);

    // "ん" is a sonorant, should receive sonorant sustain (1.15x)
    let sustain_sonorant = heuristics::apply_elastic_end_sustain("ラーメン", true, &hd);
    assert_eq!(sustain_sonorant, 1.15);
}

#[test]
fn test_batchimbound_korean_syllable_counting() {
    let hd = language::resolve_heuristics("korean");

    // Test Hangul syllables split: "안녕하세요" has 5 syllables
    let syllables_hello = parser::split_into_syllables("안녕하세요", &hd.syllables.vowels);
    assert_eq!(syllables_hello, vec!["안", "녕", "하", "세", "요"]);
    assert_eq!(parser::count_line_syllables("안녕하세요", &hd.syllables.vowels), 5);

    // "사랑" has 2 syllables
    let syllables_sarang = parser::split_into_syllables("사랑", &hd.syllables.vowels);
    assert_eq!(syllables_sarang, vec!["사", "랑"]);
    assert_eq!(parser::count_line_syllables("사랑", &hd.syllables.vowels), 2);
}

#[test]
fn test_batchimbound_korean_elastic_end_sustain() {
    let hd = language::resolve_heuristics("korean");

    // "요" ends with the diphthong vowel "ㅛ", should receive diphthong sustain (1.4x)
    let sustain_diphthong = heuristics::apply_elastic_end_sustain("안녕하세요", true, &hd);
    assert_eq!(sustain_diphthong, 1.4);

    // "가" ends with the monophthong vowel "ㅏ", should receive vowel sustain (1.25x)
    let sustain_vowel = heuristics::apply_elastic_end_sustain("가", true, &hd);
    assert_eq!(sustain_vowel, 1.25);

    // "랑" ends with batchim "ㅇ" (special sonorant), should receive sonorant sustain (1.15x)
    let sustain_sonorant = heuristics::apply_elastic_end_sustain("사랑", true, &hd);
    assert_eq!(sustain_sonorant, 1.15);
}

#[test]
fn test_moracore_japanese_lrc_line_splitting() {
    let lines = alignator::parse_lrc("[00:10.00]蒼い、蒼い、あの空", "japanese");
    assert_eq!(lines.len(), 1);
    let line = &lines[0];
    assert_eq!(line.text, "蒼い、蒼い、あの空");
    
    // Expecting 7 morae/words: ["蒼", "い、", "蒼", "い、", "あ", "の", "空"]
    assert_eq!(line.words.len(), 7);
    assert_eq!(line.words[0].word, "蒼");
    assert_eq!(line.words[1].word, "い、");
    assert_eq!(line.words[2].word, "蒼");
    assert_eq!(line.words[3].word, "い、");
    assert_eq!(line.words[4].word, "あ");
    assert_eq!(line.words[5].word, "の");
    assert_eq!(line.words[6].word, "空");
}
