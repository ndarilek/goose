use regex::Regex;
use std::sync::OnceLock;
use unicode_normalization::UnicodeNormalization;

pub fn bytes_to_hex(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = bytes.as_ref();
    let mut output = String::with_capacity(bytes.len() * 2);

    for &byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }

    output
}

fn is_in_unicode_tag_range(c: char) -> bool {
    matches!(c, '\u{E0000}'..='\u{E007F}')
}

pub fn contains_unicode_tags(text: &str) -> bool {
    text.chars().any(is_in_unicode_tag_range)
}

pub fn sanitize_unicode_tags(text: &str) -> String {
    let normalized: String = text.nfc().collect();

    normalized
        .chars()
        .filter(|&c| !is_in_unicode_tag_range(c))
        .collect()
}

pub fn safe_truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

pub fn is_openai_responses_model(model_name: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re =
        RE.get_or_init(|| Regex::new(r"(?:^|[-/])(?:o[0-9]+(?:$|-)|gpt-5(?:$|[-.]))").unwrap());
    re.is_match(&model_name.to_ascii_lowercase())
}

pub fn extract_reasoning_effort(model_name: &str) -> (String, Option<String>) {
    if !is_openai_responses_model(model_name) {
        return (model_name.to_string(), None);
    }

    let lower = model_name.to_ascii_lowercase();
    for effort in ["none", "low", "medium", "high", "xhigh"] {
        let suffix = format!("-{effort}");
        if lower.ends_with(&suffix) {
            let base_len = model_name.len() - suffix.len();
            if let Some(base) = model_name.get(..base_len) {
                return (base.to_string(), Some(effort.to_string()));
            }
        }
    }

    (model_name.to_string(), None)
}
