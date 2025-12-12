use std::fs;
use std::io;
use std::path::Path;

use crate::commands::verify::{verify_chain, VerifyError};
use crate::engram::chain::{parse_date, parse_summary};
use crate::engram::draft::Draft;
use crate::engram::worklog::WorklogEntry;

const ENGRAM_DIR: &str = ".engram";
const DRAFT_FILE: &str = ".engram/draft.md";
const WORKLOG_DIR: &str = ".engram/worklog";

pub fn run() -> io::Result<()> {
    run_status_in_dir(Path::new("."))
}

fn run_status_in_dir(base_dir: &Path) -> io::Result<()> {
    let engram_dir = base_dir.join(ENGRAM_DIR);
    let draft_file = base_dir.join(DRAFT_FILE);
    let worklog_dir = base_dir.join(WORKLOG_DIR);

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

    // Validate worklog directory exists
    if !worklog_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Engram not initialized. Run `engram init` first.",
        ));
    }

    // Get worklog info
    let info = get_worklog_info(&worklog_dir)?;
    println!("Worklog: {} entries", info.entry_count);

    // Display latest entry info if available
    if let Some(latest) = info.latest {
        println!("Latest:  {} ({})", latest.filename, latest.date);
        println!("         \"{}\"", latest.summary);
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
    HasContent(String), // Contains the summary
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

struct LatestWorklogEntry {
    filename: String,
    date: String,
    summary: String,
}

struct WorklogInfo {
    entry_count: usize,
    latest: Option<LatestWorklogEntry>,
}

/// Get worklog information: entry count and latest entry details
fn get_worklog_info(worklog_path: &Path) -> io::Result<WorklogInfo> {
    if !worklog_path.exists() {
        return Ok(WorklogInfo {
            entry_count: 0,
            latest: None,
        });
    }

    let mut entries: Vec<WorklogEntry> = Vec::new();

    for dir_entry in fs::read_dir(worklog_path)? {
        let dir_entry = dir_entry?;
        let filename = dir_entry.file_name();
        let filename_str = filename.to_string_lossy();

        if let Some(entry) = WorklogEntry::from_filename(&filename_str, worklog_path) {
            entries.push(entry);
        }
    }

    let entry_count = entries.len();

    if entries.is_empty() {
        return Ok(WorklogInfo {
            entry_count: 0,
            latest: None,
        });
    }

    // Sort by sequence number descending to get the latest
    entries.sort_by_key(|e| std::cmp::Reverse(e.sequence));
    let latest = &entries[0];

    // Read the latest entry to get date and summary
    let content = fs::read_to_string(&latest.path)?;
    let date = parse_date(&content).unwrap_or_else(|| "unknown".to_string());
    let summary = parse_summary(&content).unwrap_or_else(|| "No summary".to_string());

    Ok(WorklogInfo {
        entry_count,
        latest: Some(LatestWorklogEntry {
            filename: latest.filename.clone(),
            date,
            summary,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::hash::sha256_short;
    use std::fs;
    use tempfile::tempdir;

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

        fs::write(
            &draft_path,
            "<summary>Test summary</summary>\n\n## Intent\nSome content",
        )
        .unwrap();

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

        let info = get_worklog_info(&history_path).unwrap();
        assert_eq!(info.entry_count, 0);
        assert!(info.latest.is_none());
    }

    #[test]
    fn test_get_history_info_with_entries() {
        let dir = tempdir().unwrap();
        let history_path = dir.path().join("history");
        fs::create_dir(&history_path).unwrap();

        // Create first entry
        let content1 =
            "Summary: First entry\nPrevious: none\nDate: 2025-06-12T14:32:07Z\n\n---\n\nBody 1";
        let short_hash1 = sha256_short(content1);
        let filename1 = format!("000001_{}.md", short_hash1);
        fs::write(history_path.join(&filename1), content1).unwrap();

        // Create second entry
        let content2 = "Summary: Second entry\nPrevious: somehash\nDate: 2025-06-13T10:00:00Z\n\n---\n\nBody 2";
        let short_hash2 = sha256_short(content2);
        let filename2 = format!("000002_{}.md", short_hash2);
        fs::write(history_path.join(&filename2), content2).unwrap();

        let info = get_worklog_info(&history_path).unwrap();
        assert_eq!(info.entry_count, 2);

        let latest = info.latest.unwrap();
        assert_eq!(latest.filename, filename2);
        assert_eq!(latest.date, "2025-06-13T10:00:00Z");
        assert_eq!(latest.summary, "Second entry");
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
        fs::create_dir(base.join(".engram/worklog")).unwrap();
        fs::write(
            base.join(".engram/draft.md"),
            "<summary></summary>\n\n## Intent\n<!-- comment -->",
        )
        .unwrap();
    }
}
