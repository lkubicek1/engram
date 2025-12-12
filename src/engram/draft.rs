use regex::Regex;
use std::fmt;

#[derive(Debug)]
pub struct Draft {
    pub summary: String,
    pub body: String,
}

#[derive(Debug)]
pub enum DraftError {
    MissingSummaryTag,
    EmptySummary,
    EmptyBody,
}

impl fmt::Display for DraftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DraftError::MissingSummaryTag => {
                write!(f, "Missing <summary> tag in draft.md")
            }
            DraftError::EmptySummary => {
                write!(f, "Summary cannot be empty. Fill in the <summary> tag.")
            }
            DraftError::EmptyBody => {
                write!(f, "Draft body is empty. Document your changes.")
            }
        }
    }
}

impl std::error::Error for DraftError {}

impl Draft {
    pub fn parse(content: &str) -> Result<Self, DraftError> {
        // Extract <summary>...</summary>
        let re = Regex::new(r"<summary>(.*?)</summary>").unwrap();
        let caps = re.captures(content).ok_or(DraftError::MissingSummaryTag)?;

        let summary = caps[1].trim().to_string();
        if summary.is_empty() {
            return Err(DraftError::EmptySummary);
        }

        // Extract body after </summary>
        let body_start = content
            .find("</summary>")
            .map(|i| i + "</summary>".len())
            .unwrap_or(0);

        let body = content[body_start..].trim().to_string();

        // Check if body has content beyond template comments
        let body_without_comments = remove_html_comments(&body);
        if body_without_comments.trim().is_empty() {
            return Err(DraftError::EmptyBody);
        }

        Ok(Draft { summary, body })
    }
}

fn remove_html_comments(text: &str) -> String {
    let re = Regex::new(r"<!--.*?-->").unwrap();
    re.replace_all(text, "").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_draft() {
        let content = r#"<summary>Added new feature</summary>

## Intent
This is the intent section.

## Changes
- Modified file.rs

## Verification
Ran tests."#;

        let draft = Draft::parse(content).unwrap();
        assert_eq!(draft.summary, "Added new feature");
        assert!(draft.body.contains("Intent"));
    }

    #[test]
    fn test_parse_missing_summary_tag() {
        let content = "No summary tag here";
        let result = Draft::parse(content);
        assert!(matches!(result, Err(DraftError::MissingSummaryTag)));
    }

    #[test]
    fn test_parse_empty_summary() {
        let content = "<summary></summary>\n\n## Intent\nSome content";
        let result = Draft::parse(content);
        assert!(matches!(result, Err(DraftError::EmptySummary)));
    }

    #[test]
    fn test_parse_empty_body() {
        let content = "<summary>Summary here</summary>\n\n<!-- just comments -->";
        let result = Draft::parse(content);
        assert!(matches!(result, Err(DraftError::EmptyBody)));
    }
}
