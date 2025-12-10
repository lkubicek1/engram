use std::fs;
use std::io;
use std::path::Path;
use std::process;

use crate::engram::chain::{parse_date, parse_previous_hash};
use crate::engram::worklog::WorklogEntry;
use crate::utils::hash::{sha256_hex, sha256_short};

const ENGRAM_DIR: &str = ".engram";
const HISTORY_DIR: &str = ".engram/history";

/// Exit codes per spec
const EXIT_SUCCESS: i32 = 0;
const EXIT_CHAIN_BROKEN: i32 = 1;
const EXIT_NOT_INITIALIZED: i32 = 2;

pub fn run() -> io::Result<()> {
    // 1. Validate environment
    if !Path::new(ENGRAM_DIR).exists() || !Path::new(HISTORY_DIR).exists() {
        eprintln!("Engram not initialized. Run `engram init` first.");
        process::exit(EXIT_NOT_INITIALIZED);
    }

    // 2. List and sort entries by sequence number
    let history_path = Path::new(HISTORY_DIR);
    let mut entries = collect_entries(history_path)?;

    if entries.is_empty() {
        println!("✓ Chain verified: 0 entries");
        process::exit(EXIT_SUCCESS);
    }

    // Sort by sequence number ascending
    entries.sort_by_key(|e| e.sequence);

    // 3. Verify chain
    let mut expected_prev = "none".to_string();
    let mut first_entry: Option<(String, String)> = None;
    let mut latest_entry: Option<(String, String)> = None;

    for entry in &entries {
        let content = fs::read_to_string(&entry.path)?;

        // Extract embedded previous hash
        let embedded_prev = parse_previous_hash(&content).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Missing 'Previous:' line in {}", entry.filename),
            )
        })?;

        // Check chain linkage
        if embedded_prev != expected_prev {
            eprintln!("✗ Chain broken at entry {}", entry.filename);
            eprintln!();
            eprintln!("Expected Previous: {}", expected_prev);
            eprintln!("Found Previous:    {}", embedded_prev);
            eprintln!();
            eprintln!("The history has been tampered with or corrupted.");
            process::exit(EXIT_CHAIN_BROKEN);
        }

        // Check filename hash matches content hash
        let content_hash = sha256_hex(&content);
        let content_short_hash = sha256_short(&content);

        if content_short_hash != entry.short_hash {
            eprintln!("✗ Hash mismatch at {}", entry.filename);
            eprintln!();
            eprintln!("Content hashes to: {}", content_short_hash);
            eprintln!("Filename claims:   {}", entry.short_hash);
            eprintln!();
            eprintln!("The history has been tampered with or corrupted.");
            process::exit(EXIT_CHAIN_BROKEN);
        }

        // Track first entry info
        if first_entry.is_none() {
            let date = parse_date(&content).unwrap_or_else(|| "unknown".to_string());
            // Extract just the date part (YYYY-MM-DD)
            let date_short = date.split('T').next().unwrap_or(&date).to_string();
            first_entry = Some((entry.filename.clone(), date_short));
        }

        // Track latest entry info
        let date = parse_date(&content).unwrap_or_else(|| "unknown".to_string());
        let date_short = date.split('T').next().unwrap_or(&date).to_string();
        latest_entry = Some((entry.filename.clone(), date_short));

        // Update expected_prev for next iteration (full 64-char hash)
        expected_prev = content_hash;
    }

    // 4. Report success
    println!("✓ Chain verified: {} entries", entries.len());

    if let Some((first_file, first_date)) = first_entry {
        println!("  First: {} ({})", first_file, first_date);
    }
    if let Some((latest_file, latest_date)) = latest_entry {
        println!("  Latest: {} ({})", latest_file, latest_date);
    }

    process::exit(EXIT_SUCCESS);
}

/// Collect all valid worklog entries from the history directory
fn collect_entries(history_path: &Path) -> io::Result<Vec<WorklogEntry>> {
    let mut entries = Vec::new();

    for dir_entry in fs::read_dir(history_path)? {
        let dir_entry = dir_entry?;
        let filename = dir_entry.file_name();
        let filename_str = filename.to_string_lossy();

        // Only process valid entry files (NNN_HHHHHHHH.md pattern)
        if let Some(entry) = WorklogEntry::from_filename(&filename_str, &history_path.to_path_buf()) {
            entries.push(entry);
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
    fn test_collect_entries_empty() {
        let dir = tempdir().unwrap();
        let history_path = dir.path().join("history");
        fs::create_dir(&history_path).unwrap();

        let entries = collect_entries(&history_path).unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_collect_entries_with_files() {
        let dir = tempdir().unwrap();
        let history_path = dir.path().join("history");
        fs::create_dir(&history_path).unwrap();

        // Create some entry files
        fs::write(history_path.join("001_a1b2c3d4.md"), "content").unwrap();
        fs::write(history_path.join("002_e5f6a7b8.md"), "content").unwrap();
        fs::write(history_path.join("SUMMARY.md"), "summary").unwrap(); // Should be ignored

        let entries = collect_entries(&history_path).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_collect_entries_sorted() {
        let dir = tempdir().unwrap();
        let history_path = dir.path().join("history");
        fs::create_dir(&history_path).unwrap();

        // Create entries out of order
        fs::write(history_path.join("003_11111111.md"), "content").unwrap();
        fs::write(history_path.join("001_a1b2c3d4.md"), "content").unwrap();
        fs::write(history_path.join("002_e5f6a7b8.md"), "content").unwrap();

        let mut entries = collect_entries(&history_path).unwrap();
        entries.sort_by_key(|e| e.sequence);

        assert_eq!(entries[0].sequence, 1);
        assert_eq!(entries[1].sequence, 2);
        assert_eq!(entries[2].sequence, 3);
    }
}
