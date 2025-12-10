use chrono::Utc;
use std::fs;
use std::io;
use std::path::Path;

use crate::engram::draft::Draft;
use crate::engram::summary::append_entry;
use crate::engram::worklog::{EntryContent, WorklogEntry};
use crate::templates::DRAFT_TEMPLATE;
use crate::utils::hash::{sha256_hex, sha256_short};

const ENGRAM_DIR: &str = ".engram";
const DRAFT_FILE: &str = ".engram/draft.md";
const HISTORY_DIR: &str = ".engram/history";
const SUMMARY_FILE: &str = ".engram/history/SUMMARY.md";

/// Result of a successful commit operation
#[derive(Debug)]
pub struct CommitResult {
    pub filename: String,
    pub summary: String,
    pub previous: String,
}

pub fn run() -> io::Result<()> {
    let result = run_commit()?;

    // Output
    let prev_display = if result.previous == "none" {
        "none".to_string()
    } else {
        format!("{}...", &result.previous[..8])
    };

    println!("Committed: {}", result.filename);
    println!("Summary: {}", result.summary);
    println!("Previous: {}", prev_display);

    Ok(())
}

/// Internal commit logic that can be tested
fn run_commit() -> io::Result<CommitResult> {
    run_commit_in_dir(Path::new("."))
}

/// Commit logic with configurable base directory for testing
fn run_commit_in_dir(base_dir: &Path) -> io::Result<CommitResult> {
    let engram_dir = base_dir.join(ENGRAM_DIR);
    let draft_file = base_dir.join(DRAFT_FILE);
    let history_dir = base_dir.join(HISTORY_DIR);
    let summary_file = base_dir.join(SUMMARY_FILE);

    // 1. Validate environment
    if !engram_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Engram not initialized. Run `engram init` first.",
        ));
    }

    if !draft_file.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "draft.md not found",
        ));
    }

    // 2. Parse draft.md
    let draft_content = fs::read_to_string(&draft_file)?;
    let draft = Draft::parse(&draft_content).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, e.to_string())
    })?;

    // 3. Determine sequence number
    let sequence = get_next_sequence(&history_dir)?;

    // 4. Compute previous hash
    let prev_hash = get_previous_hash(&history_dir, sequence)?;

    // 5. Build entry content
    let entry = EntryContent {
        summary: draft.summary.clone(),
        previous: prev_hash.clone(),
        date: Utc::now(),
        body: draft.body.clone(),
    };
    let entry_content = entry.to_string();

    // 6. Compute content hash
    let short_hash = sha256_short(&entry_content);

    // 7. Write entry file
    let filename = format!("{:03}_{}.md", sequence, short_hash);
    let entry_path = history_dir.join(&filename);
    fs::write(&entry_path, &entry_content)?;

    // 8. Append to SUMMARY.md
    append_entry(&summary_file, &filename, &draft.summary)?;

    // 9. Reset draft.md
    fs::write(&draft_file, DRAFT_TEMPLATE)?;

    Ok(CommitResult {
        filename,
        summary: draft.summary,
        previous: prev_hash,
    })
}

/// Get the next sequence number by finding the highest existing entry
fn get_next_sequence(history_path: &Path) -> io::Result<u32> {
    if !history_path.exists() {
        return Ok(1);
    }

    let mut max_sequence: u32 = 0;

    for entry in fs::read_dir(history_path)? {
        let entry = entry?;
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        if let Some(worklog_entry) = WorklogEntry::from_filename(&filename_str, &history_path.to_path_buf()) {
            if worklog_entry.sequence > max_sequence {
                max_sequence = worklog_entry.sequence;
            }
        }
    }

    Ok(max_sequence + 1)
}

/// Get the hash of the previous entry (or "none" if this is the first entry)
fn get_previous_hash(history_path: &Path, current_sequence: u32) -> io::Result<String> {
    if current_sequence == 1 {
        return Ok("none".to_string());
    }

    // Find the previous entry (sequence - 1)
    let prev_sequence = current_sequence - 1;

    for entry in fs::read_dir(history_path)? {
        let entry = entry?;
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        if let Some(worklog_entry) = WorklogEntry::from_filename(&filename_str, &history_path.to_path_buf()) {
            if worklog_entry.sequence == prev_sequence {
                // Read the file content and compute its hash
                let content = fs::read_to_string(&worklog_entry.path)?;
                return Ok(sha256_hex(&content));
            }
        }
    }

    // If we can't find the previous entry, something is wrong
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Previous entry {:03}_*.md not found", prev_sequence),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_get_next_sequence_empty() {
        let dir = tempdir().unwrap();
        let history_path = dir.path().join("history");
        fs::create_dir(&history_path).unwrap();

        let seq = get_next_sequence(&history_path).unwrap();
        assert_eq!(seq, 1);
    }

    #[test]
    fn test_get_next_sequence_with_entries() {
        let dir = tempdir().unwrap();
        let history_path = dir.path().join("history");
        fs::create_dir(&history_path).unwrap();

        // Create some entry files
        fs::write(history_path.join("001_a1b2c3d4.md"), "content").unwrap();
        fs::write(history_path.join("002_e5f6a7b8.md"), "content").unwrap();
        fs::write(history_path.join("SUMMARY.md"), "summary").unwrap(); // Should be ignored

        let seq = get_next_sequence(&history_path).unwrap();
        assert_eq!(seq, 3);
    }

    #[test]
    fn test_get_previous_hash_first_entry() {
        let dir = tempdir().unwrap();
        let history_path = dir.path().join("history");
        fs::create_dir(&history_path).unwrap();

        let hash = get_previous_hash(&history_path, 1).unwrap();
        assert_eq!(hash, "none");
    }

    #[test]
    fn test_get_previous_hash_subsequent_entry() {
        let dir = tempdir().unwrap();
        let history_path = dir.path().join("history");
        fs::create_dir(&history_path).unwrap();

        let content = "Summary: Test\nPrevious: none\nDate: 2025-06-12T14:32:07Z\n\n---\n\nBody";
        fs::write(history_path.join("001_a1b2c3d4.md"), content).unwrap();

        let hash = get_previous_hash(&history_path, 2).unwrap();
        assert_eq!(hash.len(), 64); // Full SHA256 hash
        assert_eq!(hash, sha256_hex(content));
    }

    // Tests for run_commit_in_dir

    #[test]
    fn test_commit_fails_if_not_initialized() {
        let dir = tempdir().unwrap();
        // Don't create .engram directory

        let result = run_commit_in_dir(dir.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(err.to_string().contains("not initialized"));
    }

    #[test]
    fn test_commit_fails_if_draft_missing() {
        let dir = tempdir().unwrap();
        // Create .engram but not draft.md
        fs::create_dir(dir.path().join(".engram")).unwrap();
        fs::create_dir(dir.path().join(".engram/history")).unwrap();

        let result = run_commit_in_dir(dir.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(err.to_string().contains("draft.md"));
    }

    #[test]
    fn test_commit_fails_if_draft_empty_summary() {
        let dir = tempdir().unwrap();
        setup_engram_dir(dir.path());

        // Write draft with empty summary
        let draft_content = "<summary></summary>\n\n## Intent\nSome content here";
        fs::write(dir.path().join(".engram/draft.md"), draft_content).unwrap();

        let result = run_commit_in_dir(dir.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_commit_fails_if_draft_empty_body() {
        let dir = tempdir().unwrap();
        setup_engram_dir(dir.path());

        // Write draft with summary but empty body (only comments)
        let draft_content = "<summary>Test summary</summary>\n\n<!-- just comments -->";
        fs::write(dir.path().join(".engram/draft.md"), draft_content).unwrap();

        let result = run_commit_in_dir(dir.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_commit_success_first_entry() {
        let dir = tempdir().unwrap();
        setup_engram_dir(dir.path());

        // Write valid draft
        let draft_content = r#"<summary>Initial setup</summary>

## Intent
Setting up the project

## Changes
- Created main.rs

## Verification
Compiled successfully"#;
        fs::write(dir.path().join(".engram/draft.md"), draft_content).unwrap();

        let result = run_commit_in_dir(dir.path());
        assert!(result.is_ok());

        let commit_result = result.unwrap();
        assert!(commit_result.filename.starts_with("001_"));
        assert!(commit_result.filename.ends_with(".md"));
        assert_eq!(commit_result.summary, "Initial setup");
        assert_eq!(commit_result.previous, "none");

        // Verify entry file was created
        let history_dir = dir.path().join(".engram/history");
        let entry_path = history_dir.join(&commit_result.filename);
        assert!(entry_path.exists());

        // Verify entry content
        let entry_content = fs::read_to_string(&entry_path).unwrap();
        assert!(entry_content.contains("Summary: Initial setup"));
        assert!(entry_content.contains("Previous: none"));

        // Verify SUMMARY.md was updated
        let summary_content = fs::read_to_string(dir.path().join(".engram/history/SUMMARY.md")).unwrap();
        assert!(summary_content.contains(&commit_result.filename));
        assert!(summary_content.contains("Initial setup"));

        // Verify draft was reset
        let new_draft = fs::read_to_string(dir.path().join(".engram/draft.md")).unwrap();
        assert!(new_draft.contains("<summary></summary>"));
    }

    #[test]
    fn test_commit_success_subsequent_entry() {
        let dir = tempdir().unwrap();
        setup_engram_dir(dir.path());

        // Create first entry manually
        let first_entry_content = "Summary: First\nPrevious: none\nDate: 2025-06-12T14:32:07Z\n\n---\n\nFirst body";
        fs::write(
            dir.path().join(".engram/history/001_a1b2c3d4.md"),
            first_entry_content,
        ).unwrap();

        // Write valid draft for second entry
        let draft_content = r#"<summary>Second commit</summary>

## Intent
Adding more features

## Changes
- Modified lib.rs

## Verification
Tests pass"#;
        fs::write(dir.path().join(".engram/draft.md"), draft_content).unwrap();

        let result = run_commit_in_dir(dir.path());
        assert!(result.is_ok());

        let commit_result = result.unwrap();
        assert!(commit_result.filename.starts_with("002_"));
        assert_eq!(commit_result.summary, "Second commit");
        // Previous should be the hash of first entry content
        assert_eq!(commit_result.previous, sha256_hex(first_entry_content));

        // Verify entry file contains correct previous hash
        let entry_path = dir.path().join(".engram/history").join(&commit_result.filename);
        let entry_content = fs::read_to_string(&entry_path).unwrap();
        assert!(entry_content.contains(&format!("Previous: {}", sha256_hex(first_entry_content))));
    }

    /// Helper to set up a valid .engram directory structure for testing
    fn setup_engram_dir(base: &Path) {
        use crate::templates::SUMMARY_TEMPLATE;
        
        fs::create_dir(base.join(".engram")).unwrap();
        fs::create_dir(base.join(".engram/history")).unwrap();
        fs::write(base.join(".engram/history/SUMMARY.md"), SUMMARY_TEMPLATE).unwrap();
        fs::write(base.join(".engram/draft.md"), DRAFT_TEMPLATE).unwrap();
    }
}
