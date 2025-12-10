use std::fs;
use std::io;
use std::path::Path;

use crate::commands::verify::{verify_chain, VerifyError};
use crate::engram::chain::{parse_date, parse_summary};
use crate::engram::draft::Draft;
use crate::engram::worklog::WorklogEntry;

const ENGRAM_DIR: &str = ".engram";
const DRAFT_FILE: &str = ".engram/draft.md";
const HISTORY_DIR: &str = ".engram/history";

pub fn run() -> io::Result<()> {
    run_status_in_dir(Path::new("."))
}

fn run_status_in_dir(base_dir: &Path) -> io::Result<()> {
    let engram_dir = base_dir.join(ENGRAM_DIR);
    let draft_file = base_dir.join(DRAFT_FILE);
    let history_dir = base_dir.join(HISTORY_DIR);

    // Check if engram is initialized
    if !engram_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Engram not initialized. Run `engram init` first.",
        ));
    }

    // Print header
    println!("Engram Status");
    println!("─────────────");

    // Get history info
    let (entry_count, latest_entry) = get_history_info(&history_dir)?;
    println!("History: {} entries", entry_count);

    // Display latest entry info if available
    if let Some((filename, date, summary)) = latest_entry {
        println!("Latest:  {} ({})", filename, date);
        println!("         \"{}\"", summary);
    }

    println!();

    // Get draft status
    let draft_status = get_draft_status(&draft_file);
    match draft_status {
        DraftStatus::HasContent(summary) => {
            println!("Draft:   Has content (uncommitted work)");
            println!("         Summary: \"{}\"", summary);
        }
        DraftStatus::Empty => {
            println!("Draft:   Empty (ready for new work)");
        }
        DraftStatus::NotFound => {
            println!("Draft:   Not found");
        }
    }

    println!();

    // Verify chain
    match verify_chain() {
        Ok(_) => {
            println!("Chain:   ✓ Verified");
        }
        Err(VerifyError::NotInitialized) => {
            println!("Chain:   Not initialized");
        }
        Err(e) => {
            println!("Chain:   ✗ {}", e);
        }
    }

    Ok(())
}

/// Status of the draft file
enum DraftStatus {
    HasContent(String),  // Contains the summary
    Empty,
    NotFound,
}

/// Get draft status - whether it has content and the summary if available
fn get_draft_status(draft_path: &Path) -> DraftStatus {
    if !draft_path.exists() {
        return DraftStatus::NotFound;
    }

    let content = match fs::read_to_string(draft_path) {
        Ok(c) => c,
        Err(_) => return DraftStatus::NotFound,
    };

    match Draft::parse(&content) {
        Ok(draft) => DraftStatus::HasContent(draft.summary),
        Err(_) => DraftStatus::Empty,
    }
}

/// Get history information: entry count and latest entry details
fn get_history_info(history_path: &Path) -> io::Result<(usize, Option<(String, String, String)>)> {
    if !history_path.exists() {
        return Ok((0, None));
    }

    let mut entries: Vec<WorklogEntry> = Vec::new();

    for dir_entry in fs::read_dir(history_path)? {
        let dir_entry = dir_entry?;
        let filename = dir_entry.file_name();
        let filename_str = filename.to_string_lossy();

        if let Some(entry) = WorklogEntry::from_filename(&filename_str, &history_path.to_path_buf()) {
            entries.push(entry);
        }
    }

    let entry_count = entries.len();

    if entries.is_empty() {
        return Ok((0, None));
    }

    // Sort by sequence number descending to get the latest
    entries.sort_by_key(|e| std::cmp::Reverse(e.sequence));
    let latest = &entries[0];

    // Read the latest entry to get date and summary
    let content = fs::read_to_string(&latest.path)?;
    let date = parse_date(&content).unwrap_or_else(|| "unknown".to_string());
    let summary = parse_summary(&content).unwrap_or_else(|| "No summary".to_string());

    Ok((entry_count, Some((latest.filename.clone(), date, summary))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use crate::utils::hash::sha256_short;

    #[test]
    fn test_get_draft_status_not_found() {
        let dir = tempdir().unwrap();
        let draft_path = dir.path().join("draft.md");
        
        let status = get_draft_status(&draft_path);
        assert!(matches!(status, DraftStatus::NotFound));
    }

    #[test]
    fn test_get_draft_status_empty() {
        let dir = tempdir().unwrap();
        let draft_path = dir.path().join("draft.md");
        
        // Empty summary in draft
        fs::write(&draft_path, "<summary></summary>\n\n<!-- comments only -->").unwrap();
        
        let status = get_draft_status(&draft_path);
        assert!(matches!(status, DraftStatus::Empty));
    }

    #[test]
    fn test_get_draft_status_has_content() {
        let dir = tempdir().unwrap();
        let draft_path = dir.path().join("draft.md");
        
        fs::write(&draft_path, "<summary>Test summary</summary>\n\n## Intent\nSome content").unwrap();
        
        let status = get_draft_status(&draft_path);
        match status {
            DraftStatus::HasContent(summary) => assert_eq!(summary, "Test summary"),
            _ => panic!("Expected HasContent status"),
        }
    }

    #[test]
    fn test_get_history_info_empty() {
        let dir = tempdir().unwrap();
        let history_path = dir.path().join("history");
        fs::create_dir(&history_path).unwrap();

        let (count, latest) = get_history_info(&history_path).unwrap();
        assert_eq!(count, 0);
        assert!(latest.is_none());
    }

    #[test]
    fn test_get_history_info_with_entries() {
        let dir = tempdir().unwrap();
        let history_path = dir.path().join("history");
        fs::create_dir(&history_path).unwrap();

        // Create first entry
        let content1 = "Summary: First entry\nPrevious: none\nDate: 2025-06-12T14:32:07Z\n\n---\n\nBody 1";
        let short_hash1 = sha256_short(content1);
        let filename1 = format!("001_{}.md", short_hash1);
        fs::write(history_path.join(&filename1), content1).unwrap();

        // Create second entry
        let content2 = "Summary: Second entry\nPrevious: somehash\nDate: 2025-06-13T10:00:00Z\n\n---\n\nBody 2";
        let short_hash2 = sha256_short(content2);
        let filename2 = format!("002_{}.md", short_hash2);
        fs::write(history_path.join(&filename2), content2).unwrap();

        let (count, latest) = get_history_info(&history_path).unwrap();
        assert_eq!(count, 2);
        
        let (filename, date, summary) = latest.unwrap();
        assert_eq!(filename, filename2);
        assert_eq!(date, "2025-06-13T10:00:00Z");
        assert_eq!(summary, "Second entry");
    }

    #[test]
    fn test_run_status_not_initialized() {
        let dir = tempdir().unwrap();
        let result = run_status_in_dir(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_run_status_initialized_empty() {
        let dir = tempdir().unwrap();
        setup_engram_dir(dir.path());

        let result = run_status_in_dir(dir.path());
        assert!(result.is_ok());
    }

    fn setup_engram_dir(base: &Path) {
        fs::create_dir(base.join(".engram")).unwrap();
        fs::create_dir(base.join(".engram/history")).unwrap();
        fs::write(base.join(".engram/draft.md"), "<summary></summary>\n\n## Intent\n<!-- comment -->").unwrap();
    }
}
