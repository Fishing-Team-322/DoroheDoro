pub fn decode_line(input: &str) -> Option<String> {
    let normalized = input.trim_end_matches(&['\r', '\n'][..]).to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::decode_line;

    #[test]
    fn trims_newlines() {
        assert_eq!(decode_line("hello\n").as_deref(), Some("hello"));
        assert_eq!(decode_line("hello\r\n").as_deref(), Some("hello"));
    }

    #[test]
    fn ignores_blank_lines() {
        assert_eq!(decode_line("\n"), None);
    }
}
