use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct WorklogEntry {
    pub sequence: u32,
    pub short_hash: String, // 8 chars
    pub filename: String,   // "002_e5f6a7b8.md"
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct EntryContent {
    pub summary: String,
    pub previous: String, // "none" or 64-char hash
    pub date: DateTime<Utc>,
    pub body: String,
}

impl std::fmt::Display for EntryContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Summary: {}\nPrevious: {}\nDate: {}\n\n---\n\n{}",
            self.summary,
            self.previous,
            self.date.format("%Y-%m-%dT%H:%M:%SZ"),
            self.body
        )
    }
}

impl WorklogEntry {
    /// Parse a worklog entry filename into its components
    /// Format: NNNNNN_HHHHHHHH.md (e.g., "000002_e5f6a7b8.md")
    pub fn from_filename(filename: &str, base_path: &Path) -> Option<Self> {
        let re = regex::Regex::new(r"^(\d{6})_([a-f0-9]{8})\.md$").unwrap();
        let caps = re.captures(filename)?;

        let sequence: u32 = caps[1].parse().ok()?;
        let short_hash = caps[2].to_string();

        Some(WorklogEntry {
            sequence,
            short_hash,
            filename: filename.to_string(),
            path: base_path.join(filename),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_content_to_string() {
        let entry = EntryContent {
            summary: "Test summary".to_string(),
            previous: "none".to_string(),
            date: DateTime::parse_from_rfc3339("2025-06-12T14:32:07Z")
                .unwrap()
                .with_timezone(&Utc),
            body: "## Intent\nTest body".to_string(),
        };

        let output = entry.to_string();
        assert!(output.contains("Summary: Test summary"));
        assert!(output.contains("Previous: none"));
        assert!(output.contains("Date: 2025-06-12T14:32:07Z"));
        assert!(output.contains("## Intent"));
    }

    #[test]
    fn test_worklog_entry_from_filename() {
        let base_path = PathBuf::from(".engram/worklog");
        let entry = WorklogEntry::from_filename("000002_e5f6a7b8.md", base_path.as_path()).unwrap();

        assert_eq!(entry.sequence, 2);
        assert_eq!(entry.short_hash, "e5f6a7b8");
        assert_eq!(entry.filename, "000002_e5f6a7b8.md");
    }

    #[test]
    fn test_worklog_entry_invalid_filename() {
        let base_path = PathBuf::from(".engram/worklog");
        assert!(WorklogEntry::from_filename("invalid.md", base_path.as_path()).is_none());
        assert!(WorklogEntry::from_filename("00002_e5f6a7b8.md", base_path.as_path()).is_none());
        assert!(WorklogEntry::from_filename("000002_e5f6.md", base_path.as_path()).is_none());
    }
}
