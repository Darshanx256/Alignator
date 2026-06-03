# Alignator Multi-Platform Lyric Sync SDK

Alignator is a high-performance, zero-dependency multilingual lyric-to-word synchronization heuristics engine written in **Rust**, with dynamic bridges for **Web/TypeScript/TSX (via WASM)** and **Android/iOS Mobile (via UniFFI)**. 

It takes line-level synchronized lyric inputs (Standard LRC or Enhanced LRC) and uses advanced singing tempo, pause heuristics, and phonetic sustain estimators to output precise word-level and syllable-level timelines.

---

## Monorepo Layout

```
alignator/
├── Cargo.toml                  # Cargo workspace definition
├── README.md                   # This overview
├── heuristics_data/            # Centralized phonetic rules JSON configurations
│   ├── en.json
│   ├── hindi.json
│   ├── spanish.json
│   └── ...
├── core/
│   └── rust/                   # Pure, zero-dependency Rust core synchronization engine
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs          # Parser and RateRider entry point
│           ├── heuristics.rs   # Phonetic sustain, punctuation delay, duplicate gap rules
│           ├── tempo.rs        # RateRider tempo tracker & Fast-Track compressor
│           ├── language.rs     # Language auto-detect and static rules loader
│           └── parser.rs       # Tag matcher & whitespace clean helper
├── formatters/
│   └── rust/                   # Formatter implementations in Rust
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs          # Apple Music TTML, Spotify-style JSON, and Enhanced LRC formatters
└── bridges/
    ├── wasm/                   # WebAssembly JS/TS bindings for Web, Node, TSX, React Native
    │   ├── Cargo.toml
    │   └── src/
    │       └── lib.rs          # wasm-bindgen exports
    └── uniffi/                 # UniFFI native wrapper for Android (Kotlin) & iOS (Swift)
        ├── Cargo.toml
        └── src/
            └── lib.rs          # UniFFI record declarations and scaffolding
```

---

## Getting Started & Compiling

To work with this monorepo, ensure you have the Rust toolchain installed.

### 1. Compile Core Library
```bash
cd core/rust
cargo build --release
```

### 2. Compile WebAssembly Bindings (TSX / JS)
Use [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) to build the JavaScript/TypeScript packages:
```bash
cd bridges/wasm
# For bundlers (Webpack, Vite, React, TSX)
wasm-pack build --target bundler
# For Node.js
wasm-pack build --target nodejs
```

### 3. Generate Mobile Bindings (UniFFI / Swift / Kotlin)
Install `uniffi-bindgen`:
```bash
cargo install uniffi_bindgen
```
Build the dynamic libraries:
```bash
cd bridges/uniffi
cargo build --release
```
Generate native bindings:
```bash
# Generate Kotlin bindings for Android
uniffi-bindgen generate src/lib.rs --language kotlin --out-dir out/kotlin

# Generate Swift bindings for iOS
uniffi-bindgen generate src/lib.rs --language swift --out-dir out/swift
```

---

## Formatters

Alignator translates raw lyric timelines into the following production-ready formats:
*   **Apple Music TTML**: Clean XML structure utilizing `<span begin="..." end="...">` styling tags.
*   **Spotify-Style JSON**: Structured nested JSON representing text-lines with start offsets and an array of word-level syllable structures.
*   **Enhanced LRC**: Traditional LRC string formatted with inline sub-word millisecond tags: `[mm:ss.xx] <mm:ss.xx> word <mm:ss.xx> next_word`.

---

## Technical Heuristics & Core Logic

Alignator employs a series of advanced client-side heuristic rules to generate accurate word-level and syllable-level timelines from simple line-level lyrics:

### 1. RateRider (Syllable-Level Tempo Tracker)
* **Purpose**: Tracks the song's actual average singing speed (syllables per millisecond) to dynamically space out line durations when an instrumental gap follows.
* **Logic**: On startup, it runs a simulated timing pass over all standard lyric lines (lines followed by gaps $\le 6000\text{ ms}$). It counts phonetic syllables using `split_into_syllables` and divides the total active singing duration (after factoring in punctuation delays and time compression) by this count to obtain a highly accurate average millisecond-per-syllable tempo metric.

### 2. GapGlide (Instrumental Gap Estimation)
* **Purpose**: Dynamically spaces out singing duration when an instrumental gap follows a phrase.
* **Logic**: Handled by the `apply_gap_glide_estimation` function, when a phrase is followed by an instrumental gap ($> 6000\text{ ms}$), the player places the countdown indicator line exactly at:
  $$\text{Lyric Timestamp} + \text{max}\left(2500\text{ ms}, \text{min}(\text{Syllables} \times \text{RateRider Tempo} + 800\text{ ms}, \text{Gap Size} - 2000\text{ ms})\right)$$
  This gives long phrases the extra breathing space they naturally need, preventing the text from being prematurely terminated.

### 3. GapSentry (Dynamic Instrumental Gap Detection)
* **Purpose**: Dynamically determines if a line is followed by a genuine instrumental gap before applying the `GapGlide` timing override.
* **Logic**: Implemented in `apply_gap_sentry_check`, it compares the next line's timestamp offset (`line_duration_ms`) against the estimated singing duration (`est_singing_duration_ms`). A line is only treated as having a gap if the remaining silent duration is at least **`1000ms`** (allowing enough time for the `BeatBeacon` dots countdown):
  $$\text{Remaining Gap} = \text{Line Duration} - \text{Estimated Singing Duration} \ge 1000\text{ ms}$$
  If the remaining gap is smaller than `1000ms`, the line is sung continuously without instrumental dots.

### 4. Fast-Track (Active Line Termination)
* **Purpose**: Guarantees that all words in a phrase are marked as fully sung and cleanly finished exactly before the highlight transitions to the next line.
* **Logic**: 
  * **Timing Generation**: Compresses word schedules if they exceed the deadline ($150\text{ ms}$ before the next line begins).
  * **Playback Rendering**: Forces any remaining active or future words on the active line to the completed (Cyan) state as soon as the playback clock is within $100\text{ ms}$ of the next line's start.
* **RateRider Integration**: Since Fast-Track alters the true active singing duration of a line, the `RateRider` first-pass simulation calculates and uses the compressed duration (`deadline_ms - start_ms`) for any lines that trigger compression, aligning the tempo calculation with the actual display speed.

### 5. CommaCadence (Dynamic Punctuation Delays)
* **Purpose**: Dynamically adjusts vocal pause lengths at grammatical boundaries (commas, semicolons, colons) relative to the song's tempo.
* **Logic**: Instead of a static $120\text{ ms}$ pause, when punctuation is encountered, the pause is dynamically scaled:
  $$\text{Pause Length} = \text{max}\left(80\text{ ms}, \text{min}(\text{RateRider Tempo} \times 0.5, 250\text{ ms})\right)$$
  Fast songs get snappy $80\text{ ms}$ pauses, while slower ballads get longer $250\text{ ms}$ breath windows. The remaining duration is distributed among the words so that synchronization is perfectly preserved.

### 6. DoubleOomph (Double 'O' Vocal Emphasis)
* **Purpose**: Increases the relative duration/emphasis allocated to words that start with a double 'o' (e.g. `"ooh"`), which is a common melodic vowel elongation in pop music.
* **Logic**: During the weighting pass, if a word begins with `"oo"` (case-insensitive), its character-length weight is boosted by $35\%$:
  $$\text{Weight} = \text{int}(\text{Alphanumeric Length} \times 1.35)$$
* **RateRider Integration**: Because this alters word weight and layout, it directly scales the singing duration simulated and averaged by `RateRider`.

### 7. Rapid Comma Emphasis (Subtle Clustered Pauses)
* **Purpose**: Adds subtle vocal weight to phrasing lists (e.g. `"Pull up, pull up, pull up"`) where multiple commas are clustered close together.
* **Logic**: If a word ends with a comma, the script inspects the next 3 words in the line. If another word ending with a comma is found within this range, the weight of the word preceding the first comma is increased by $30\%$:
  $$\text{Weight} = \text{int}(\text{Weight} \times 1.30)$$
* **RateRider Integration**: Skews the tempo average slightly to account for the deliberate elongation singers apply during rapid lists.

### 8. BeatBeacon (Dynamic Instrumental Gap Countdown)
* **Purpose**: Generates a dynamic, visual countdown during instrumental breaks to build anticipation for the next line.
* **Logic**: For line-level gaps $> 6000\text{ ms}$, the program inserts a special `"•••"` gap marker line. During visualization, the progress is calculated:
  $$\text{progress} = \frac{\text{current\_time} - \text{start\_time}}{\text{end\_time} - \text{start\_time}}$$
  The dot highlighting updates dynamically:
  * $\text{progress} < 0.08$: `•••` (all dimmed)
  * $\text{progress} < 0.33$: `•` (active/yellow), `••` (dimmed)
  * $\text{progress} < 0.66$: `••` (active/yellow), `•` (dimmed)
  * $\text{progress} \ge 0.66$: `•••` (all active/yellow)

### 9. WordWarden (Enhanced LRC Word-Level Gap Capping)
* **Purpose**: Automatically injects natural pauses between words in Enhanced LRC files (with syllable tags) if the singer pauses between words.
* **Logic**: Handled by the `apply_word_warden_cap` function, the active duration of each subword is capped using:
  $$\text{maxDur} = \text{min}(\text{len}(\text{word}) \times 65\text{ ms} + 200\text{ ms}, 850\text{ ms})$$
  If the duration between consecutive word tags is larger than this limit, the word's highlight finishes early, leaving the remaining time before the next word as a silent word gap.

### 10. StepStrider (Enhanced LRC Fallback Boundary Estimator)
* **Purpose**: Estimates the boundary timestamp for the last word of an Enhanced LRC line when there is no trailing syllable tag to mark the end.
* **Logic**: Handled by the `apply_step_strider_fallback` function, which bounds the last word's estimated duration between $300\text{ ms}$ and $1200\text{ ms}$:
  $$\text{Next Boundary} = \text{min}\left(1200\text{ ms}, \text{max}(300\text{ ms}, \text{len}(\text{word\_text}) \times 150\text{ ms})\right) + \text{current\_timestamp}$$

### 11. SpanSizer (Standard LRC Active Duration Boundary)
* **Purpose**: Bounds the overall active duration of standard lyric lines to prevent them from stretching too far or being too short.
* **Logic**: Implemented in `calculate_span_sizer_duration`, it constrains the active lyric span relative to the number of words:
  $$\text{Coerced} = \text{max}\left(\text{Words} \times 300\text{ ms}, \text{min}(\text{Safe Duration} \times 0.82, \text{Words} \times 900\text{ ms})\right)$$
  $$\text{Active Duration} = \text{min}\left(\text{Coerced}, \text{max}(\text{Safe Duration}, \text{Words} \times 350\text{ ms})\right)$$

### 12. ElasticEnd (Phonetic Last Syllable Sustain)
* **Purpose**: Dynamically adjusts the sustain duration of the last word in a phrase based on the phonetic properties of its last syllable.
* **Logic**: Implemented in `apply_elastic_end_sustain`, it parses the last syllable of the word and applies a stretch multiplier based on its sustain capability:
  * **Category A: Diphthongs / Double Vowels** (`"ee"`, `"oo"`, `"aa"`, `"ou"`, `"ow"`, `"ay"`, `"ey"`, `"oy"`, `"ea"`, `"ai"`, `"ie"`, `"ue"`, `"ui"`, `"uy"`) -> **`1.40`** multiplier (40% boost).
  * **Category B: Single Open Vowels** (`"a"`, `"e"`, `"i"`, `"o"`, `"u"`, `"y"`) -> **`1.25`** multiplier (25% boost).
  * **Category C: Sonorant Consonants** (`"r"`, `"l"`, `"m"`, `"n"`, or ending in `"ng"`) -> **`1.15`** multiplier (15% boost).
  * **Category D: Hard Stops** (e.g. `"t"`, `"k"`, `"p"`) -> **`1.00`** multiplier (no boost).
* **Integration**:
  * **Case A (Standard LRC)**: Scales the weighting of the line's final word.
  * **Case B (Enhanced LRC)**: Extends both the estimated duration and the min/max limits inside the `StepStrider` fallback handler.

### 13. EchoPause (Duplicate Word Pause Heuristic)
* **Purpose**: Automatically introduces a short phrasing pause between adjacent identical words (e.g., `"I I"` or `"I, I"`) to separate repeated notes visually and phonetically.
* **Logic**: Implemented in `apply_echo_pause_check`, it compares normalized adjacent words (stripped of case and punctuation). If they match, a pause is injected:
  * **Case A (Standard LRC)**: Injects a **`100ms`** pause after the first word, reducing available singing duration to distribute spacing.
  * **Case B (Enhanced LRC)**: Shortens the end highlighting boundary of the first word to guarantee a minimum **`100ms`** visual gap before the next duplicate word highlight starts (maintaining a minimum word duration of $100\text{ ms}$).
* **Hyphenated Compounds Exception**: Compounds connected by hyphens in a single string (like `"I-I"` or `"Bang-bang"`) are split as single words and do not trigger duplicate pause logic.

### 14. MoraCore (Japanese Mora-Timed Syllable Splitter)
* **Purpose**: Dynamically parses Japanese Kana into morae (musical beats) instead of standard Latin syllables to resolve timing beats.
* **Logic**: Fuses small Kana (yōon/non-starter kana like `ゃ`, `ゅ`, `ょ`, `ゎ`, etc.) into the preceding consonant-vowel Kana, counting them as a single fused mora. It counts standard Kana (`あ`, `か`, etc.), the moraic nasal (`ん`/`ン`), the sokuon held consonant (`っ`/`ッ`), and long vowel markers (`ー`) as individual, equal-duration morae.

### 15. BatchimBound (Korean Hangul & Batchim Sustainer)
* **Purpose**: Decomposes Hangul syllable blocks into Jamo to accurately apply phonetic sustain rules on final batchim consonants or vowel nuclei.
* **Logic**: Decomposes combined Hangul characters (`\u{AC00}`..=`\u{D7A3}`) into initial consonant (Choseong), vowel nucleus (Jungseong), and final consonant (Jongseong/batchim). If a batchim is present and matches a sonorant batchim (`ㄴ`, `ㄹ`, `ㅁ`, `ㅇ`), it receives a sonorant sustain (`1.15x`). If no batchim is present, the vowel nucleus determines if it receives diphthong (`1.40x`) or monophthong vowel (`1.25x`) sustain.

---

## Code Examples

### 1. Rust Usage
```rust
use alignator::parse_lrc;
use alignator_formatters::to_spotify_json;

fn main() {
    let lrc = "[00:12.30] Hello, world!";
    let lines = parse_lrc(lrc, "auto");
    
    let spotify_json = to_spotify_json(&lines);
    println!("{}", spotify_json);
}
```

### 2. TypeScript / TSX / React Native Usage
```typescript
import init, { parseLrc, toAppleMusicTtml } from "alignator-wasm";

async function run() {
    // Initialize WebAssembly
    await init();
    
    const lrc = "[00:12.30] (Hello, world!)";
    
    // Aligns lyrics, automatically detecting language and splitting backing vocals
    const lines = parseLrc(lrc, "auto");
    
    const ttmlXml = toAppleMusicTtml(lines);
    console.log(ttmlXml);
}
```

### 3. Android Kotlin Usage
```kotlin
import uniffi.alignator.parseLrc
import uniffi.alignator.toSpotifyJson

fun syncLyrics(lrcText: String) {
    val lines = parseLrc(lrcText, "auto")
    val jsonString = toSpotifyJson(lines)
    // Render in UI
}
```

### 4. iOS Swift Usage
```swift
import Alignator

func syncLyrics(lrcText: String) {
    let lines = parseLrc(lrcText: lrcText, languageMode: "auto")
    let jsonString = toSpotifyJson(lines: lines)
    // Render in UI
}
```
