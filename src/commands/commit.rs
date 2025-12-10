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

pub fn run() -> io::Result<()> {
    // 1. Validate environment
    if !Path::new(ENGRAM_DIR).exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Engram not initialized. Run `engram init` first.",
        ));
    }

    if !Path::new(DRAFT_FILE).exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "draft.md not found",
        ));
    }

    // 2. Parse draft.md
    let draft_content = fs::read_to_string(DRAFT_FILE)?;
    let draft = Draft::parse(&draft_content).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, e.to_string())
    })?;

    // 3. Determine sequence number
    let history_path = Path::new(HISTORY_DIR);
    let sequence = get_next_sequence(history_path)?;

    // 4. Compute previous hash
    let prev_hash = get_previous_hash(history_path, sequence)?;

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
    let entry_path = history_path.join(&filename);
    fs::write(&entry_path, &entry_content)?;

    // 8. Append to SUMMARY.md
    let summary_path = Path::new(SUMMARY_FILE);
    append_entry(summary_path, &filename, &draft.summary)?;

    // 9. Reset draft.md
    fs::write(DRAFT_FILE, DRAFT_TEMPLATE)?;

    // Output
    let prev_display = if prev_hash == "none" {
        "none".to_string()
    } else {
        format!("{}...", &prev_hash[..8])
    };

    println!("Committed: {}", filename);
    println!("Summary: {}", draft.summary);
    println!("Previous: {}", prev_display);

    Ok(())
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
}
