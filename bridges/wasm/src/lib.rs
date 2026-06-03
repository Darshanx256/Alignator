use alignator::{parse_lrc, Line};
use alignator_formatters::{to_apple_music_ttml, to_enhanced_lrc, to_spotify_json};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = parseLrc)]
pub fn js_parse_lrc(lrc_text: &str, language_mode: &str) -> Result<JsValue, JsValue> {
    let lines = parse_lrc(lrc_text, language_mode);
    serde_wasm_bindgen::to_value(&lines).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = toAppleMusicTtml)]
pub fn js_to_apple_music_ttml(val: JsValue) -> Result<String, JsValue> {
    let lines: Vec<Line> =
        serde_wasm_bindgen::from_value(val).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(to_apple_music_ttml(&lines))
}

#[wasm_bindgen(js_name = toSpotifyJson)]
pub fn js_to_spotify_json(val: JsValue) -> Result<String, JsValue> {
    let lines: Vec<Line> =
        serde_wasm_bindgen::from_value(val).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(to_spotify_json(&lines))
}

#[wasm_bindgen(js_name = toEnhancedLrc)]
pub fn js_to_enhanced_lrc(val: JsValue) -> Result<String, JsValue> {
    let lines: Vec<Line> =
        serde_wasm_bindgen::from_value(val).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(to_enhanced_lrc(&lines))
}
