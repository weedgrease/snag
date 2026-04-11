/// Truncates a string to fit within `max_len` bytes, respecting UTF-8 char boundaries.
/// Appends '…' if truncated.
pub fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len || max_len <= 1 {
        return s.to_string();
    }
    let target = max_len.saturating_sub(1);
    let boundary = s
        .char_indices()
        .take_while(|(i, _)| *i < target)
        .last()
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    format!("{}…", &s[..boundary])
}
