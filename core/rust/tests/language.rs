use alignator::language;

#[test]
fn test_locale_language_modes_resolve_to_base_heuristics() {
    assert_eq!(
        language::resolve_heuristics("en-US"),
        language::resolve_heuristics("en")
    );
    assert_eq!(
        language::resolve_heuristics("es-MX"),
        language::resolve_heuristics("spanish")
    );
    assert_eq!(
        language::resolve_heuristics("pt-BR"),
        language::resolve_heuristics("portuguese")
    );
}

#[test]
fn test_auto_detection_is_stable_for_plain_english() {
    for _ in 0..100 {
        assert_eq!(
            language::detect_language("Hello world, this is a plain English lyric."),
            "en"
        );
    }
}

#[test]
fn test_auto_detection_for_japanese_and_korean() {
    assert_eq!(language::detect_language("ありがとう"), "japanese");
    assert_eq!(language::detect_language("ね"), "japanese");
    assert_eq!(language::detect_language("안녕하세요"), "korean");
    assert_eq!(language::detect_language("사랑"), "korean");
}
