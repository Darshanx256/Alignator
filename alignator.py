#!/usr/bin/env python3
import sys
import os
import re
import time
import json

# Ensure current dir is in sys.path to find generated UniFFI Python bindings
current_dir = os.path.dirname(os.path.realpath(__file__))
if current_dir not in sys.path:
    sys.path.insert(0, current_dir)

HAS_RUST = False
try:
    import alignator_uniffi
    HAS_RUST = True
except Exception as e:
    if "uniffi-bindgen" not in sys.argv:
        print(f"Warning: Failed to load Rust engine via UniFFI: {e}. Falling back to Python engine.")


def load_heuristics_data(lang="en"):
    """
    Loads language-specific heuristics data from heuristics_data/{lang}.json.
    Falls back to default English settings if file doesn't exist or error occurs.
    """
    default_data = {
        "double_oomph_prefix": "oo",
        "elastic_end": {
            "diphthongs": ["ee", "oo", "aa", "ou", "ow", "ay", "ey", "oy", "ea", "ai", "ie", "ue", "ui", "uy"],
            "vowels": ["a", "e", "i", "o", "u", "y"],
            "sonorants": ["r", "l", "m", "n"],
            "special_sonorant_endings": ["ng"]
        },
        "syllables": {
            "vowels": "aeiouyAEIOUY"
        }
    }
    
    script_dir = os.path.dirname(os.path.abspath(__file__))
    config_path = os.path.join(script_dir, "heuristics_data", f"{lang}.json")
    
    if not os.path.exists(config_path):
        # Try prefix matching (e.g. "marathi" matches "marathi_devanagari.json")
        heuristics_dir = os.path.join(script_dir, "heuristics_data")
        if os.path.isdir(heuristics_dir):
            try:
                for filename in os.listdir(heuristics_dir):
                    if filename.endswith(".json") and filename.split(".")[0].startswith(lang):
                        config_path = os.path.join(heuristics_dir, filename)
                        break
            except Exception:
                pass
                
    if not os.path.exists(config_path):
        return default_data
        
    try:
        with open(config_path, "r", encoding="utf-8") as f:
            content = f.read()
            cleaned_lines = []
            for line in content.splitlines():
                parts = line.split("//")
                cleaned_lines.append(parts[0])
            data = json.loads("\n".join(cleaned_lines))
            # If the data is nested under a language name key (e.g. "hindi_devanagari")
            # we automatically unwrap it to get the heuristic parameters.
            if isinstance(data, dict) and len(data) == 1 and "double_oomph_prefix" not in data:
                key = next(iter(data))
                if isinstance(data[key], dict):
                    data = data[key]
            # Basic validation to ensure schema keys exist, fallback to default for missing keys
            validated = {}
            validated["double_oomph_prefix"] = data.get("double_oomph_prefix", default_data["double_oomph_prefix"])
            
            # validate elastic_end
            elastic_end = data.get("elastic_end", {})
            validated["elastic_end"] = {
                "diphthongs": elastic_end.get("diphthongs", default_data["elastic_end"]["diphthongs"]),
                "vowels": elastic_end.get("vowels", default_data["elastic_end"]["vowels"]),
                "sonorants": elastic_end.get("sonorants", default_data["elastic_end"]["sonorants"]),
                "special_sonorant_endings": elastic_end.get("special_sonorant_endings", default_data["elastic_end"]["special_sonorant_endings"])
            }
            
            # validate syllables
            syllables = data.get("syllables", {})
            validated["syllables"] = {
                "vowels": syllables.get("vowels", default_data["syllables"]["vowels"])
            }
            return validated
    except Exception as e:
        print(f"Warning: Failed to load heuristics data for '{lang}' from {config_path}: {e}. Using English defaults.")
        return default_data

def load_all_languages():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    heuristics_dir = os.path.join(script_dir, "heuristics_data")
    
    # Start with English default
    langs = {
        "en": {
            "double_oomph_prefix": "oo",
            "elastic_end": {
                "diphthongs": ["ee", "oo", "aa", "ou", "ow", "ay", "ey", "oy", "ea", "ai", "ie", "ue", "ui", "uy"],
                "vowels": ["a", "e", "i", "o", "u", "y"],
                "sonorants": ["r", "l", "m", "n"],
                "special_sonorant_endings": ["ng"]
            },
            "syllables": {
                "vowels": "aeiouyAEIOUY"
            }
        }
    }
    
    if os.path.isdir(heuristics_dir):
        try:
            for filename in os.listdir(heuristics_dir):
                if filename.endswith(".json"):
                    lang_name = filename.split(".")[0]
                    langs[lang_name] = load_heuristics_data(lang_name)
        except Exception:
            pass
            
    return langs

def detect_language(text, loaded_langs):
    """
    Detects the best language pack from loaded_langs by counting matches
    against the vowels/syllables of each language.
    Defaults to 'en' if no high-confidence match is found or if Latin dominates.
    """
    if not text:
        return "en"
        
    scores = {}
    for lang, config in loaded_langs.items():
        vowels = config.get("syllables", {}).get("vowels", "")
        # Count occurrences of these vowel characters in the text
        # Weight non-ASCII characters higher (10x) because they are strong indicators of accents
        count = 0
        for char in text:
            if char in vowels:
                count += 10 if ord(char) > 127 else 1
        scores[lang] = count
        
    # Find the language with the maximum score
    max_lang = "en"
    max_score = 0
    for lang, score in scores.items():
        if score > max_score:
            max_score = score
            max_lang = lang
            
    if max_score == 0:
        return "en"
        
    # Tie-breaker logic for Devanagari (Hindi/Marathi):
    if "hindi" in scores or "marathi_devanagari" in scores:
        hi_score = scores.get("hindi", 0)
        mr_score = scores.get("marathi_devanagari", 0)
        if hi_score > 0 or mr_score > 0:
            # Check for Marathi-specific characters
            marathi_vowels = "ॲऑ"
            if any(c in text for c in marathi_vowels):
                return "marathi_devanagari"
            if hi_score > 0 and mr_score > 0:
                return "hindi"
                
    # Tie-breaker logic for Latin-based scripts (spanish, french, portuguese):
    fr_chars = "èêîôùûœæëïÈÊÎÔÙÛŒÆËÏ"
    pt_chars = "ãõàâÃÕÀÂ"
    es_chars = "ñíóúüÑÍÓÚÜ"
    
    has_fr = any(c in text for c in fr_chars)
    has_pt = any(c in text for c in pt_chars)
    has_es = any(c in text for c in es_chars)
    
    if has_fr:
        return "french"
    elif has_pt:
        return "portuguese"
    elif has_es:
        return "spanish"
                
    return max_lang

# Load all available languages at startup
LOADED_LANGUAGES = load_all_languages()
HEURISTICS_DATA = LOADED_LANGUAGES.get("en")
LANGUAGE_MODE = "auto"
DOMINANT_LANGUAGE = "en"




def calculate_comma_cadence_pause(ms_per_syllable):
    """
    CommaCadence: Calculates dynamic vocal pause duration (ms) at grammatical boundaries
    based on RateRider tempo. Bounded between 80ms and 250ms.
    """
    return int(max(80, min(ms_per_syllable * 0.5, 250)))

def calculate_punctuation_pause(word, ms_per_syllable):
    """
    Calculate Word Punctuation Pause: Calculates the duration (ms) of the pause following a word,
    including the CommaCadence dynamic scaling for commas, colons, and semicolons.
    """
    pause = 120
    if word.endswith("..."):
        pause = 350
    elif word.endswith(".") or word.endswith("!") or word.endswith("?"):
        pause = 300
    elif not (word.endswith(",") or word.endswith(";") or word.endswith(":")):
        pause = 0
    else:
        pause = calculate_comma_cadence_pause(ms_per_syllable)
    return pause

def apply_gap_glide_estimation(text, line_duration_ms, rate_rider):
    """
    GapGlide: Estimates phrase singing duration and calculates gap buffer when followed by
    an instrumental break (> 6000ms).
    """
    # Estimate phrase singing duration, add 800ms rest buffer
    if isinstance(rate_rider, (int, float)):
        est_duration = int(count_line_syllables(text) * rate_rider + 800)
    else:
        est_duration = int(rate_rider.estimate_duration(text) + 800)
    # Leave at least 2000ms for dots in the gap
    allowed_duration = line_duration_ms - 2000
    return max(2500, min(est_duration, allowed_duration))

def apply_elastic_end_sustain(word, is_last_word=True):
    """
    ElasticEnd: Dynamically calculates a timing sustain multiplier (1.0 to 1.4)
    based on the phonetic stretchability of the word's last syllable.
    """
    if not is_last_word:
        return 1.0
        
    syllables = split_into_syllables(word)
    if not syllables:
        return 1.0
        
    last_syl = re.sub(r"[^a-z]", "", syllables[-1].lower())
    if not last_syl:
        return 1.0
        
    diphthongs = tuple(HEURISTICS_DATA["elastic_end"]["diphthongs"])
    vowels = tuple(HEURISTICS_DATA["elastic_end"]["vowels"])
    sonorants = tuple(HEURISTICS_DATA["elastic_end"]["sonorants"])
    special_sonorants = tuple(HEURISTICS_DATA["elastic_end"]["special_sonorant_endings"])
    
    if last_syl.endswith(diphthongs):
        return 1.4
    elif last_syl.endswith(vowels):
        return 1.25
    elif last_syl.endswith(sonorants) or last_syl.endswith(special_sonorants):
        return 1.15
    return 1.0

def apply_step_strider_fallback(word_text, current_ms):
    """
    StepStrider: Estimates the fallback timestamp boundary for the last word of an Enhanced LRC line
    to prevent excessive trailing highlights, scaled dynamically by ElasticEnd sustain.
    """
    base_duration = len(word_text) * 150
    multiplier = apply_elastic_end_sustain(word_text, is_last_word=True)
    
    stretched_duration = int(base_duration * multiplier)
    min_limit = int(300 * multiplier)
    max_limit = int(1200 * multiplier)
    
    return min(max_limit, max(min_limit, stretched_duration)) + current_ms

def calculate_span_sizer_duration(words_count, safe_duration):
    """
    SpanSizer: Determines the active highlighted duration range for Case A (Standard LRC) alignment.
    """
    # RangesKt.coerceIn(...) equivalent:
    val_to_coerce = int(safe_duration * 0.82)
    min_limit = words_count * 300
    max_limit = words_count * 900
    coerced = max(min_limit, min(val_to_coerce, max_limit))
    
    # RangesKt.coerceAtMost(...) equivalent:
    return min(coerced, max(safe_duration, words_count * 350))

def apply_echo_pause_check(word1, word2):
    """
    EchoPause: Checks if two adjacent words are duplicates (e.g., "I I" or "I, I")
    excluding hyphenated words.
    """
    cleaned1 = re.sub(r"[^a-zA-Z0-9]", "", word1).lower()
    cleaned2 = re.sub(r"[^a-zA-Z0-9]", "", word2).lower()
    if not cleaned1 or not cleaned2:
        return False
    return cleaned1 == cleaned2

def apply_gap_sentry_check(line_duration_ms, est_singing_duration_ms):
    """
    GapSentry: Dynamically determines if a line is followed by a genuine instrumental gap 
    based on whether the remaining silent duration is sufficient (>= 1000ms).
    """
    if line_duration_ms <= 6000:
        return False
    remaining_gap = line_duration_ms - est_singing_duration_ms
    return remaining_gap >= 1000

def apply_double_oomph_weight(cleaned_word, current_weight):
    """
    DoubleOomph: Amplifies the word weight by 35% if the word begins with double 'o'.
    """
    prefix = HEURISTICS_DATA["double_oomph_prefix"]
    if cleaned_word.lower().startswith(prefix):
        return int(current_weight * 1.35)
    return current_weight

def apply_rapid_comma_emphasis(word, words_list, idx, current_weight):
    """
    Rapid Comma Emphasis: Increments the weight of the previous word by 30%
    if another comma is encountered within the next 3 words on the same line.
    """
    if word.endswith(","):
        has_second_comma_soon = False
        for next_idx in range(idx + 1, min(idx + 4, len(words_list))):
            if words_list[next_idx].endswith(","):
                has_second_comma_soon = True
                break
        if has_second_comma_soon:
            return int(current_weight * 1.3)
    return current_weight

def apply_word_warden_cap(sub_word, sub_word_start, original_end):
    """
    WordWarden: Restricts the maximum active highlighted duration of a syllable/word in Case B
    to avoid stretching during silent word gaps. Returns the final end timestamp.
    """
    max_dur = min((len(sub_word) * 65) + 200, 850)
    if (original_end - sub_word_start) > max_dur:
        return sub_word_start + max_dur
    return original_end

def apply_flextrack_compression(words, singing_duration_ms, line_start_ms, ms_per_syllable):
    """
    FlexTrack: Estimates time compression for a line when followed by an early next-line start.
    Limits the maximum compression ratio dynamically based on RateRider tempo to be stricter on slow tracks (up to 30% compression / factor of 0.70)
    and more relaxed on fast tracks (up to 15% compression / factor of 0.85).
    """
    if not words:
        return
    original_start = words[0]["start_ms"]
    original_end = words[-1]["end_ms"]
    line_end_ms = line_start_ms + singing_duration_ms
    deadline_ms = line_end_ms - 150
    if original_end > deadline_ms and deadline_ms > original_start:
        original_duration = max(original_end - original_start, 1)
        target_deadline_duration = max(deadline_ms - original_start, 1)
        
        # Stricter on slow tracks (up to 0.70 at >=500ms), relaxed on fast tracks (up to 0.85 at <=200ms)
        if ms_per_syllable <= 200.0:
            ratio = 0.85
        elif ms_per_syllable >= 500.0:
            ratio = 0.70
        else:
            ratio = 0.85 - 0.15 * ((ms_per_syllable - 200.0) / 300.0)
            
        min_duration_allowed = int(original_duration * ratio)
        target_duration = max(target_deadline_duration, min_duration_allowed)
        
        for w in words:
            w["start_ms"] = original_start + int(((w["start_ms"] - original_start) / original_duration) * target_duration)
            w["end_ms"] = original_start + int(((w["end_ms"] - original_start) / original_duration) * target_duration)

def levenshtein_distance(s1, s2):
    v1 = list(s1)
    v2 = list(s2)
    len1 = len(v1)
    len2 = len(v2)
    
    dp = [[0] * (len2 + 1) for _ in range(len1 + 1)]
    for i in range(len1 + 1):
        dp[i][0] = i
    for j in range(len2 + 1):
        dp[0][j] = j
        
    for i in range(1, len1 + 1):
        for j in range(1, len2 + 1):
            if v1[i - 1] == v2[j - 1]:
                dp[i][j] = dp[i - 1][j - 1]
            else:
                dp[i][j] = 1 + min(
                    dp[i - 1][j - 1],
                    dp[i - 1][j],
                    dp[i][j - 1]
                )
    return dp[len1][len2]

def calculate_echo_echo_similarity(s1, s2):
    dist = levenshtein_distance(s1, s2)
    max_len = max(len(s1), len(s2))
    if max_len == 0:
        return 1.0
    return 1.0 - (dist / max_len)

def format_beat_beacon(line, current_time):
    """
    BeatBeacon: Renders dynamic pulsing dots countdown visualization for instrumental gaps ("•••").
    """
    start_time_ms = line["timestamp_ms"]
    end_time_ms = line.get("end_ms", start_time_ms + 3500)
    duration = max(end_time_ms - start_time_ms, 100)
    elapsed = max(0, min(current_time - start_time_ms, duration))
    progress = elapsed / duration
    if progress < 0.08:
        active_dots = 0
    elif progress < 0.33:
        active_dots = 1
    elif progress < 0.66:
        active_dots = 2
    else:
        active_dots = 3
        
    dot_chars = []
    for d in range(1, 4):
        if active_dots >= d:
            dot_chars.append("\033[1;93m•\033[0m")
        else:
            dot_chars.append("\033[90m•\033[0m")
    return "".join(dot_chars)

def parse_line_to_words(line_timestamp_ms, line_content, line_duration_ms, ms_per_syllable=320.0):
    """
    Python implementation of the Kotlin LyricParser.parseLineToWords method.
    """
    global HEURISTICS_DATA
    old_heuristics = HEURISTICS_DATA
    
    # Regex to find word/syllable timing tags like <00:06.44> or <00:06>
    word_regex = re.compile(r"<(\d+):(\d+)(?:\.(\d+))?>")
    
    # Replace timing tags with spaces and normalize whitespace
    text_with_spaces = word_regex.sub(" ", line_content)
    raw_text_cleaned = re.sub(r"\s+", " ", text_with_spaces).strip()
    
    if LANGUAGE_MODE == "auto":
        detected = detect_language(raw_text_cleaned, LOADED_LANGUAGES)
        HEURISTICS_DATA = LOADED_LANGUAGES.get(detected, LOADED_LANGUAGES["en"])
        # Print when language deviates from or reverts to song dominant language
        last_reported = getattr(parse_line_to_words, "last_reported_lang", DOMINANT_LANGUAGE)
        if detected != last_reported:
            suffix = " (switching heuristics)" if detected != DOMINANT_LANGUAGE else " (reverting to dominant)"
            print(f"[Auto-Detect] Line text '{raw_text_cleaned[:30]}...' detected as: {detected}{suffix}")
            parse_line_to_words.last_reported_lang = detected
            
    try:
        return _parse_line_to_words_impl(line_timestamp_ms, line_content, line_duration_ms, ms_per_syllable)
    finally:
        HEURISTICS_DATA = old_heuristics

def _parse_line_to_words_impl(line_timestamp_ms, line_content, line_duration_ms, ms_per_syllable=320.0):
    """
    Internal implementation of the Kotlin LyricParser.parseLineToWords method.
    """
    # Regex to find word/syllable timing tags like <00:06.44> or <00:06>
    word_regex = re.compile(r"<(\d+):(\d+)(?:\.(\d+))?>")
    matches = list(word_regex.finditer(line_content))
    
    # Replace timing tags with spaces and normalize whitespace
    text_with_spaces = word_regex.sub(" ", line_content)
    raw_text_cleaned = re.sub(r"\s+", " ", text_with_spaces).strip()

    
    # Case A: No word-level timing tags (Standard LRC) -> Apply Heuristic Alignator
    if not matches:
        # Check if CJK/Japanese characters exist in raw_text_cleaned
        is_japanese = any(
            (0x3040 <= ord(c) <= 0x30FF) or (0x31F0 <= ord(c) <= 0x31FF) or 
            (0xFF65 <= ord(c) <= 0xFF9F) or (0x4E00 <= ord(c) <= 0x9FFF) or
            c == 'ー'
            for c in raw_text_cleaned
        )
        
        if is_japanese:
            words_raw = []
            current = []
            for c in raw_text_cleaned:
                if c.isspace() or c == '　':
                    if current:
                        words_raw.append("".join(current))
                        current = []
                    continue
                
                if c in ('、', '。', '？', '！', ',', '.', '?', '!'):
                    if current:
                        current.append(c)
                        words_raw.append("".join(current))
                        current = []
                    else:
                        words_raw.append(c)
                    continue
                
                is_kana_or_kanji = (
                    (0x3040 <= ord(c) <= 0x309F) or (0x30A0 <= ord(c) <= 0x30FF) or 
                    (0x31F0 <= ord(c) <= 0x31FF) or (0xFF65 <= ord(c) <= 0xFF9F) or
                    (0x4E00 <= ord(c) <= 0x9FFF) or c == 'ー'
                )
                
                if is_kana_or_kanji:
                    is_non_starter = c in "ぁぃぅぇぉゃゅょゎァィゥェォャュョヮ"
                    if is_non_starter:
                        if current:
                            current.append(c)
                        elif words_raw:
                            words_raw[-1] += c
                        else:
                            current.append(c)
                    else:
                        if current:
                            words_raw.append("".join(current))
                            current = []
                        current.append(c)
                else:
                    # ASCII
                    if current:
                        first_char = current[0]
                        u = ord(first_char)
                        current_is_cjk = (
                            (0x3040 <= u <= 0x30FF) or (0x31F0 <= u <= 0x31FF) or 
                            (0xFF65 <= u <= 0xFF9F) or (0x4E00 <= u <= 0x9FFF) or
                            first_char == 'ー'
                        )
                        if current_is_cjk:
                            words_raw.append("".join(current))
                            current = []
                    current.append(c)
            if current:
                words_raw.append("".join(current))
        else:
            words_raw = [w for w in re.split(r"\s+", raw_text_cleaned) if len(w) > 0]

        if not words_raw:
            return []
            
        safe_duration = max(400, line_duration_ms)
        active_duration = calculate_span_sizer_duration(len(words_raw), safe_duration)
        
        # Calculate pauses based on word endings and duplicate word adjacent pairs (EchoPause)
        pauses = []
        for i, w in enumerate(words_raw):
            pause = calculate_punctuation_pause(w, ms_per_syllable)
            if i < len(words_raw) - 1:
                next_w = words_raw[i+1]
                if apply_echo_pause_check(w, next_w):
                    pause += 100  # Introduce duplicate word gap (100ms)
            pauses.append(pause)
        total_pause_ms = sum(pauses)
            
        available_active_duration = max(active_duration - total_pause_ms, len(words_raw) * 120)
        
        # Alphanumeric character weights with heuristics
        word_weights = []
        alphanumeric_regex = re.compile(r"[^a-zA-Z0-9]")
        for i, w in enumerate(words_raw):
            cleaned = alphanumeric_regex.sub("", w)
            weight = max(1, len(cleaned))
            
            # Apply DoubleOomph
            weight = apply_double_oomph_weight(cleaned, weight)
                
            # Apply Rapid Comma Emphasis
            weight = apply_rapid_comma_emphasis(w, words_raw, i, weight)
            
            # Apply ElasticEnd sustain to the last word in the line
            if i == len(words_raw) - 1:
                weight = int(weight * apply_elastic_end_sustain(w, is_last_word=True))
                    
            word_weights.append(weight)
            
        total_weight = sum(word_weights)
        word_infos = []
        current_offset = 0
        
        for i, word in enumerate(words_raw):
            weight = word_weights[i]
            if total_weight > 0:
                duration = int((weight * available_active_duration) / total_weight)
            else:
                duration = 200
                
            start_ms = line_timestamp_ms + current_offset
            end_ms = start_ms + duration
            word_infos.append({
                "word": word,
                "start_ms": start_ms,
                "end_ms": end_ms
            })
            current_offset += duration + pauses[i]
            
        return word_infos

    # Case B: Enhanced LRC with word-level tags
    word_infos = []
    
    # 1. Parse text before the first tag
    first_match = matches[0]
    if first_match.start() > 0:
        leading_text = line_content[:first_match.start()].strip()
        if leading_text:
            first_tag_ms = parse_match_ms(first_match.groups())
            words = [w for w in re.split(r"\s+", leading_text) if len(w) > 0]
            if words:
                duration = first_tag_ms - line_timestamp_ms
                step = duration // len(words) if duration > 0 else 100
                for j, word in enumerate(words):
                    word_infos.append({
                        "word": word,
                        "start_ms": line_timestamp_ms + (j * step),
                        "end_ms": line_timestamp_ms + ((j + 1) * step)
                    })
                    
    # 2. Parse text between tags and after the last tag
    for i, current_match in enumerate(matches):
        current_ms = parse_match_ms(current_match.groups())
        start_of_text = current_match.end()
        end_of_text = matches[i + 1].start() if i < len(matches) - 1 else len(line_content)
        
        word_text = line_content[start_of_text:end_of_text].strip()
        if not word_text:
            continue
            
        if i < len(matches) - 1:
            next_ms = parse_match_ms(matches[i + 1].groups())
        else:
            # Estimate next time bound (StepStrider fallback)
            next_ms = apply_step_strider_fallback(word_text, current_ms)
            
        sub_words = [w for w in re.split(r"\s+", word_text) if len(w) > 0]
        if sub_words:
            total_duration = next_ms - current_ms
            step = total_duration // len(sub_words) if total_duration > 0 else 100
            for j, sub_word in enumerate(sub_words):
                sub_word_start = current_ms + (j * step)
                original_end = current_ms + ((j + 1) * step)
                
                # Apply WordWarden
                final_end = apply_word_warden_cap(sub_word, sub_word_start, original_end)
                
                word_infos.append({
                    "word": sub_word,
                    "start_ms": sub_word_start,
                    "end_ms": final_end
                })
                
    word_infos.sort(key=lambda x: x["start_ms"])
    
    # Apply EchoPause to Case B (guarantee 100ms visual gap for duplicate words)
    for i in range(len(word_infos) - 1):
        w1 = word_infos[i]
        w2 = word_infos[i+1]
        if apply_echo_pause_check(w1["word"], w2["word"]):
            current_gap = w2["start_ms"] - w1["end_ms"]
            if current_gap < 100:
                duration = w1["end_ms"] - w1["start_ms"]
                max_shorten = duration - 100
                if max_shorten > 0:
                    shorten_by = min(100 - current_gap, max_shorten)
                    w1["end_ms"] -= int(shorten_by)
                    
    return word_infos


def parse_match_ms(groups):
    """Parse regex match groups into ms."""
    minutes = int(groups[0]) if groups[0] else 0
    seconds = int(groups[1]) if groups[1] else 0
    fraction_text = groups[2] if groups[2] else ""
    
    fraction_ms = 0
    if len(fraction_text) == 1:
        fraction_ms = int(fraction_text) * 100
    elif len(fraction_text) == 2:
        fraction_ms = int(fraction_text) * 10
    elif len(fraction_text) == 3:
        fraction_ms = int(fraction_text)
    elif len(fraction_text) > 3:
        fraction_ms = int(fraction_text[:3])
        
    return ((60 * minutes) + seconds) * 1000 + fraction_ms

def split_line_if_braced(line):
    """
    Splits bracketed backing vocals out into separate right-aligned lines.
    """
    words = line.get("words", [])
    if not words:
        return [line]
        
    main_words = []
    braced_words = []
    
    in_braces = False
    for w_info in words:
        word_text = w_info["word"]
        if word_text.startswith('('):
            in_braces = True
            
        if in_braces:
            braced_words.append(w_info)
        else:
            main_words.append(w_info)
            
        if word_text.endswith(')'):
            in_braces = False
            
    if not braced_words or not main_words:
        return [line]
        
    # Split into two lines
    # 1. Main Line (Left aligned)
    main_text = " ".join([w["word"] for w in main_words])
    main_line = {
        "timestamp_ms": main_words[0]["start_ms"],
        "text": main_text,
        "words": main_words,
        "is_right_aligned": False,
        "braced_text": None,
        "singing_duration_ms": line.get("singing_duration_ms"),
        "is_gap_next": line.get("is_gap_next", False)
    }
    
    # 2. Backing Line (Right aligned)
    braced_text = " ".join([w["word"] for w in braced_words])
    braced_line = {
        "timestamp_ms": braced_words[0]["start_ms"],
        "text": braced_text,
        "words": braced_words,
        "is_right_aligned": True,
        "braced_text": braced_text,
        "singing_duration_ms": line.get("singing_duration_ms"),
        "is_gap_next": line.get("is_gap_next", False)
    }
    
    return [main_line, braced_line]

def split_into_syllables(word):
    """
    Python implementation of the splitIntoSyllables method from MainActivity.kt.
    """
    if len(word) <= 3:
        return [word]
    syllables = []
    current = []
    vowels = HEURISTICS_DATA["syllables"]["vowels"]
    i = 0
    while i < len(word):
        c = word[i]
        current.append(c)
        is_vowel = c in vowels
        if is_vowel and i < len(word) - 1:
            next_char = word[i + 1]
            if next_char not in vowels:
                if i < len(word) - 2 and word[i + 2] in vowels:
                    syllables.append("".join(current))
                    current = []
                elif i < len(word) - 3 and word[i + 2] not in vowels and word[i + 3] in vowels:
                    current.append(next_char)
                    i += 1
                    syllables.append("".join(current))
                    current = []
        i += 1
    if current:
        syllables.append("".join(current))
    rejoined = "".join(syllables)
    if rejoined != word:
        return [word]
    return syllables

def count_line_syllables(text):
    """Counts the total syllables in a line of text using split_into_syllables."""
    clean_text = re.sub(r"<[^>]+>", "", text)
    global HEURISTICS_DATA
    old_heuristics = HEURISTICS_DATA
    if LANGUAGE_MODE == "auto":
        detected = detect_language(clean_text, LOADED_LANGUAGES)
        HEURISTICS_DATA = LOADED_LANGUAGES.get(detected, LOADED_LANGUAGES["en"])
    try:
        words = re.findall(r"\b\w+\b", clean_text)
        total = 0
        for w in words:
            total += len(split_into_syllables(w))
        return max(1, total)
    finally:
        HEURISTICS_DATA = old_heuristics

class RateRider:
    """
    RateRider tracks the song's average pace (syllables per millisecond).
    It helps estimate how long a line actually takes to sing when followed by an instrumental gap.
    """
    def __init__(self, sorted_lines):
        total_syllables = 0
        total_duration = 0
        
        # Incorporate the flextrack compression and punctuation delays to get the exact singing tempo
        for idx, line in enumerate(sorted_lines):
            if idx < len(sorted_lines) - 1:
                duration = sorted_lines[idx + 1]["timestamp_ms"] - line["timestamp_ms"]
                # Skip long gap lines to keep the tempo average accurate
                if duration > 6000:
                    continue
                
                # Get the words generated with a default rate (320.0ms) to measure pauses
                words = parse_line_to_words(line["timestamp_ms"], line["text"], duration, ms_per_syllable=320.0)
                if words:
                    # Apply flextrack compression heuristic
                    apply_flextrack_compression(words, duration, line["timestamp_ms"], 320.0)
                    actual_dur = words[-1]["end_ms"] - words[0]["start_ms"]
                    total_duration += actual_dur
                    total_syllables += count_line_syllables(line["text"])
                
        if total_syllables > 0 and total_duration > 0:
            self.ms_per_syllable = total_duration / total_syllables
        else:
            self.ms_per_syllable = 320.0  # Safe default (320ms per syllable)
            
    def estimate_duration(self, text):
        syllables = count_line_syllables(text)
        return syllables * self.ms_per_syllable

class StandardLineInfo:
    def __init__(self, text, syllables, tempo):
        self.text = text
        self.syllables = syllables
        self.tempo = tempo

def parse_lrc(lrc_text):
    """
    Python implementation of LyricParser.parseLrc method.
    Delegates to Rust via UniFFI if available, falling back to the Python heuristics.
    """
    global HEURISTICS_DATA, DOMINANT_LANGUAGE
    if HAS_RUST:
        try:
            rust_lines = alignator_uniffi.parse_lrc(lrc_text, LANGUAGE_MODE)
            final_lines = []
            for line in rust_lines:
                words = []
                if line.words is not None:
                    for w in line.words:
                        words.append({
                            "word": w.word,
                            "start_ms": w.start_ms,
                            "end_ms": w.end_ms
                        })
                else:
                    words = None
                
                final_lines.append({
                    "timestamp_ms": line.timestamp_ms,
                    "text": line.text,
                    "words": words,
                    "is_right_aligned": line.is_right_aligned,
                    "braced_text": line.braced_text,
                    "singing_duration_ms": line.singing_duration_ms,
                    "is_gap_next": line.is_gap_next
                })
            return final_lines
        except Exception as e:
            print(f"Warning: Rust parse_lrc failed: {e}. Falling back to Python engine.")

    if LANGUAGE_MODE == "auto":
        DOMINANT_LANGUAGE = detect_language(lrc_text, LOADED_LANGUAGES)
        song_hd = LOADED_LANGUAGES.get(DOMINANT_LANGUAGE, LOADED_LANGUAGES["en"])
        print(f"[Auto-Detect] Song dominant language: {DOMINANT_LANGUAGE}")
    else:
        DOMINANT_LANGUAGE = LANGUAGE_MODE
        song_hd = LOADED_LANGUAGES.get(DOMINANT_LANGUAGE, LOADED_LANGUAGES["en"])
        
    HEURISTICS_DATA = song_hd
        
    # Reset last reported language for parse_line_to_words
    if hasattr(parse_line_to_words, "last_reported_lang"):
        delattr(parse_line_to_words, "last_reported_lang")

    lines = re.split(r"[\n\r]+", lrc_text)
    parsed_lines = []
    
    # Match [mm:ss.xx]Text or [mm:ss]Text
    line_regex = re.compile(r"^\[(\d+):(\d+)(?:\.(\d+))?\](.*)$")
    
    for line in lines:
        trimmed = line.strip()
        match = line_regex.match(trimmed)
        if match:
            minutes = int(match.group(1))
            seconds = int(match.group(2))
            fraction_text = match.group(3) or ""
            raw_text = match.group(4).strip()
            
            fraction_ms = 0
            if len(fraction_text) == 1:
                fraction_ms = int(fraction_text) * 100
            elif len(fraction_text) == 2:
                fraction_ms = int(fraction_text) * 10
            elif len(fraction_text) == 3:
                fraction_ms = int(fraction_text)
            elif len(fraction_text) > 3:
                fraction_ms = int(fraction_text[:3])
                
            total_ms = ((60 * minutes) + seconds) * 1000 + fraction_ms
            parsed_lines.append({
                "timestamp_ms": total_ms,
                "text": raw_text
            })
            
    sorted_lines = sorted(parsed_lines, key=lambda x: x["timestamp_ms"])
    
    # Initialize the RateRider tempo tracker
    rate_rider = RateRider(sorted_lines)
    
    # --- PASS 1: Dry run to build the EchoEcho standard lines database ---
    standard_lines = []
    for idx, current in enumerate(sorted_lines):
        line_duration_ms = sorted_lines[idx + 1]["timestamp_ms"] - current["timestamp_ms"] if idx < len(sorted_lines) - 1 else 5000

        if LANGUAGE_MODE == "auto":
            word_regex = re.compile(r"<(\d+):(\d+)(?:\.(\d+))?>")
            text_without_tags = word_regex.sub("", current["text"])
            text_without_tags = re.sub(r"\s+", " ", text_without_tags).strip()
            line_lang = detect_language(text_without_tags, LOADED_LANGUAGES)
            line_hd = LOADED_LANGUAGES.get(line_lang, LOADED_LANGUAGES["en"])
        else:
            line_hd = song_hd

        old_hd = HEURISTICS_DATA
        HEURISTICS_DATA = line_hd
        try:
            est_singing_ms = int(rate_rider.estimate_duration(current["text"]) + 800)
            is_gap_next = apply_gap_sentry_check(line_duration_ms, est_singing_ms)
            if is_gap_next:
                singing_duration_ms = apply_gap_glide_estimation(current["text"], line_duration_ms, rate_rider.ms_per_syllable)
            else:
                singing_duration_ms = line_duration_ms

            words = parse_line_to_words(current["timestamp_ms"], current["text"], singing_duration_ms, rate_rider.ms_per_syllable)
            apply_flextrack_compression(words, singing_duration_ms, current["timestamp_ms"], rate_rider.ms_per_syllable)

            if not is_gap_next and words:
                original_start = words[0]["start_ms"]
                original_end = words[-1]["end_ms"]
                deadline_ms = current["timestamp_ms"] + line_duration_ms - 150
                if original_end <= deadline_ms:
                    syllables = count_line_syllables(current["text"])
                    if syllables > 0:
                        actual_duration = original_end - original_start
                        tempo_val = actual_duration / syllables
                        standard_lines.append(StandardLineInfo(current["text"], syllables, tempo_val))
        finally:
            HEURISTICS_DATA = old_hd

    # --- PASS 2: Final parsing using similarity timing propagation (EchoEcho) and global tempo ---
    final_lines = []
    for idx, current in enumerate(sorted_lines):
        line_duration_ms = sorted_lines[idx + 1]["timestamp_ms"] - current["timestamp_ms"] if idx < len(sorted_lines) - 1 else 5000

        if LANGUAGE_MODE == "auto":
            word_regex = re.compile(r"<(\d+):(\d+)(?:\.(\d+))?>")
            text_without_tags = word_regex.sub("", current["text"])
            text_without_tags = re.sub(r"\s+", " ", text_without_tags).strip()
            line_lang = detect_language(text_without_tags, LOADED_LANGUAGES)
            line_hd = LOADED_LANGUAGES.get(line_lang, LOADED_LANGUAGES["en"])
        else:
            line_hd = song_hd

        old_hd = HEURISTICS_DATA
        HEURISTICS_DATA = line_hd
        try:
            # EchoEcho: Find similar standard line to propagate its singing speed
            propagated_tempo = None
            best_similarity = 0.0
            for std_line in standard_lines:
                sim = calculate_echo_echo_similarity(current["text"], std_line.text)
                if sim >= 0.70 and sim > best_similarity:
                    best_similarity = sim
                    propagated_tempo = std_line.tempo

            line_ms_per_syllable = propagated_tempo if propagated_tempo is not None else rate_rider.ms_per_syllable
            syllables = count_line_syllables(current["text"])

            if propagated_tempo is not None:
                est_singing_ms = int(syllables * propagated_tempo + 800)
            else:
                est_singing_ms = int(rate_rider.estimate_duration(current["text"]) + 800)

            is_gap_next = apply_gap_sentry_check(line_duration_ms, est_singing_ms)
            if is_gap_next:
                singing_duration_ms = apply_gap_glide_estimation(current["text"], line_duration_ms, line_ms_per_syllable)
            else:
                singing_duration_ms = line_duration_ms

            # Clean text by removing word-level tags
            word_regex = re.compile(r"<(\d+):(\d+)(?:\.(\d+))?>")
            clean_text = word_regex.sub("", current["text"])
            clean_text = re.sub(r"\s+", " ", clean_text).strip()

            words = parse_line_to_words(current["timestamp_ms"], current["text"], singing_duration_ms, line_ms_per_syllable)
            apply_flextrack_compression(words, singing_duration_ms, current["timestamp_ms"], line_ms_per_syllable)

            initial_line = {
                "timestamp_ms": current["timestamp_ms"],
                "text": clean_text,
                "words": words,
                "is_right_aligned": False,
                "braced_text": None,
                "singing_duration_ms": singing_duration_ms,
                "is_gap_next": is_gap_next
            }

            split_lines = split_line_if_braced(initial_line)
            final_lines.extend(split_lines)
        finally:
            HEURISTICS_DATA = old_hd
        
    final_lines.sort(key=lambda x: x["timestamp_ms"])
    return final_lines

# Re-exports for backwards compatibility with test and visualizer scripts
def inject_gaps_and_dots(*args, **kwargs):
    from cli import inject_gaps_and_dots as _impl
    return _impl(*args, **kwargs)

def run_karaoke(*args, **kwargs):
    from cli import run_karaoke as _impl
    return _impl(*args, **kwargs)
