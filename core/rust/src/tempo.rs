use crate::parser::count_line_syllables;
use crate::WordInfo;

pub fn apply_gap_sentry_check(line_duration_ms: i32, est_singing_duration_ms: i32) -> bool {
    if line_duration_ms <= 6000 {
        return false;
    }
    let remaining_gap = line_duration_ms - est_singing_duration_ms;
    remaining_gap >= 1000
}

pub fn apply_gap_glide_estimation(
    text: &str,
    line_duration_ms: i32,
    ms_per_syllable: f64,
    vowels: &str,
) -> i32 {
    let syllables = count_line_syllables(text, vowels);
    let est_duration = (syllables as f64 * ms_per_syllable + 800.0) as i32;
    let allowed_duration = line_duration_ms - 2000;
    std::cmp::max(2500, std::cmp::min(est_duration, allowed_duration))
}

/// FlexTrack: Estimates time compression for a line when followed by an early next-line start.
/// Limits the maximum compression ratio dynamically based on RateRider tempo to be stricter on slow tracks (up to 30% compression / factor of 0.70)
/// and more relaxed on fast tracks (up to 15% compression / factor of 0.85).
pub fn apply_flextrack_compression(
    words: &mut [WordInfo],
    singing_duration_ms: i32,
    line_start_ms: i32,
    ms_per_syllable: f64,
) {
    if words.is_empty() {
        return;
    }
    let original_start = words[0].start_ms;
    let original_end = words[words.len() - 1].end_ms;
    let line_end_ms = line_start_ms + singing_duration_ms;
    let deadline_ms = line_end_ms - 150;
    
    if original_end > deadline_ms && deadline_ms > original_start {
        let original_duration = std::cmp::max(original_end - original_start, 1);
        let target_deadline_duration = std::cmp::max(deadline_ms - original_start, 1);
        
        // Stricter on slow tracks (up to 0.70 at >=500ms), relaxed on fast tracks (up to 0.85 at <=200ms)
        let ratio = if ms_per_syllable <= 200.0 {
            0.85
        } else if ms_per_syllable >= 500.0 {
            0.70
        } else {
            0.85 - 0.15 * ((ms_per_syllable - 200.0) / 300.0)
        };
        
        let min_duration_allowed = (original_duration as f64 * ratio) as i32;
        let target_duration = std::cmp::max(target_deadline_duration, min_duration_allowed);
        
        for w in words.iter_mut() {
            let offset_start = w.start_ms - original_start;
            let offset_end = w.end_ms - original_start;

            w.start_ms = original_start
                + ((offset_start as f64 / original_duration as f64) * target_duration as f64)
                    as i32;
            w.end_ms = original_start
                + ((offset_end as f64 / original_duration as f64) * target_duration as f64) as i32;
        }
    }
}

/// EchoEcho: Computes Levenshtein distance between two strings.
pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let v1: Vec<char> = s1.chars().collect();
    let v2: Vec<char> = s2.chars().collect();
    let len1 = v1.len();
    let len2 = v2.len();
    
    let mut dp = vec![vec![0; len2 + 1]; len1 + 1];
    for i in 0..=len1 {
        dp[i][0] = i;
    }
    for j in 0..=len2 {
        dp[0][j] = j;
    }
    
    for i in 1..=len1 {
        for j in 1..=len2 {
            if v1[i - 1] == v2[j - 1] {
                dp[i][j] = dp[i - 1][j - 1];
            } else {
                dp[i][j] = 1 + std::cmp::min(
                    dp[i - 1][j - 1],
                    std::cmp::min(dp[i - 1][j], dp[i][j - 1])
                );
            }
        }
    }
    dp[len1][len2]
}

/// EchoEcho: Calculates text similarity ratio (0.0 to 1.0) based on Levenshtein distance.
pub fn calculate_echo_echo_similarity(s1: &str, s2: &str) -> f64 {
    let dist = levenshtein_distance(s1, s2);
    let max_len = std::cmp::max(s1.chars().count(), s2.chars().count());
    if max_len == 0 {
        return 1.0;
    }
    1.0 - (dist as f64 / max_len as f64)
}
