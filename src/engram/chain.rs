use regex::Regex;

/// Parse the Previous hash from entry content
/// Returns the hash string ("none" or 64-char hex)
pub fn parse_previous_hash(content: &str) -> Option<String> {
    let re = Regex::new(r"^Previous: ([a-f0-9]{64}|none)$").unwrap();
    for line in content.lines() {
        if let Some(caps) = re.captures(line) {
            return Some(caps[1].to_string());
        }
    }
    None
}

/// Parse the Summary from entry content
pub fn parse_summary(content: &str) -> Option<String> {
    let re = Regex::new(r"^Summary: (.+)$").unwrap();
    for line in content.lines() {
        if let Some(caps) = re.captures(line) {
            return Some(caps[1].to_string());
        }
    }
    None
}

/// Parse the Date from entry content
pub fn parse_date(content: &str) -> Option<String> {
    let re = Regex::new(r"^Date: (.+)$").unwrap();
    for line in content.lines() {
        if let Some(caps) = re.captures(line) {
            return Some(caps[1].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_previous_hash_none() {
        let content = "Summary: Test\nPrevious: none\nDate: 2025-06-12T14:32:07Z";
        assert_eq!(parse_previous_hash(content), Some("none".to_string()));
    }

    #[test]
    fn test_parse_previous_hash_full() {
        let content = "Summary: Test\nPrevious: a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2\nDate: 2025-06-12T14:32:07Z";
        assert_eq!(
            parse_previous_hash(content),
            Some("a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2".to_string())
        );
    }

    #[test]
    fn test_parse_previous_hash_missing() {
        let content = "Summary: Test\nDate: 2025-06-12T14:32:07Z";
        assert_eq!(parse_previous_hash(content), None);
    }

    #[test]
    fn test_parse_summary() {
        let content =
            "Summary: Added JWT authentication\nPrevious: none\nDate: 2025-06-12T14:32:07Z";
        assert_eq!(
            parse_summary(content),
            Some("Added JWT authentication".to_string())
        );
    }

    #[test]
    fn test_parse_date() {
        let content = "Summary: Test\nPrevious: none\nDate: 2025-06-12T14:32:07Z";
        assert_eq!(
            parse_date(content),
            Some("2025-06-12T14:32:07Z".to_string())
        );
    }
}
