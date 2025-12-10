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

/// Read all entries from SUMMARY.md
/// Returns a list of (filename, summary) tuples
pub fn read_entries(summary_path: &Path) -> io::Result<Vec<(String, String)>> {
    let content = fs::read_to_string(summary_path)?;
    let mut entries = Vec::new();
    
    for line in content.lines() {
        // Skip header lines and separator
        if line.starts_with("| Entry") || line.starts_with("|---") || line.starts_with("# ") {
            continue;
        }
        
        // Parse table row: | filename | summary |
        if line.starts_with('|') {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 3 {
                let filename = parts[1].trim().to_string();
                let summary = parts[2].trim().to_string();
                if !filename.is_empty() {
                    entries.push((filename, summary));
                }
            }
        }
    }
    
    Ok(entries)
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
        fs::write(&summary_path, "# Engram Worklog\n\n| Entry | Summary |\n|-------|--------|\n").unwrap();
        
        // Append entry
        append_entry(&summary_path, "001_a1b2c3d4.md", "First commit").unwrap();
        
        let content = fs::read_to_string(&summary_path).unwrap();
        assert!(content.contains("| 001_a1b2c3d4.md | First commit |"));
    }

    #[test]
    fn test_read_entries() {
        let dir = tempdir().unwrap();
        let summary_path = dir.path().join("SUMMARY.md");
        
        let content = r#"# Engram Worklog

| Entry | Summary |
|-------|---------|
| 001_a1b2c3d4.md | First commit |
| 002_e5f6a7b8.md | Second commit |
"#;
        fs::write(&summary_path, content).unwrap();
        
        let entries = read_entries(&summary_path).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], ("001_a1b2c3d4.md".to_string(), "First commit".to_string()));
        assert_eq!(entries[1], ("002_e5f6a7b8.md".to_string(), "Second commit".to_string()));
    }
}
