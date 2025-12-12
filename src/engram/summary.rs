use std::fs;
use std::io;
use std::path::Path;

/// Append an entry to the SUMMARY.md file
/// Format: | {filename} | {summary} |
pub fn append_entry(summary_path: &Path, filename: &str, summary: &str) -> io::Result<()> {
    let line = format!("| {} | {} |\n", filename, summary);
    let mut content = fs::read_to_string(summary_path)?;
    content.push_str(&line);
    fs::write(summary_path, content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_append_entry() {
        let dir = tempdir().unwrap();
        let summary_path = dir.path().join("SUMMARY.md");

        // Create initial summary file
        fs::write(
            &summary_path,
            "# Engram Worklog\n\n| Entry | Summary |\n|-------|--------|\n",
        )
        .unwrap();

        // Append entry
        append_entry(&summary_path, "000001_a1b2c3d4.md", "First commit").unwrap();

        let content = fs::read_to_string(&summary_path).unwrap();
        assert!(content.contains("| 000001_a1b2c3d4.md | First commit |"));
    }
}
