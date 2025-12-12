use std::fs;
use std::io;
use std::path::Path;
use std::process;

use crate::engram::chain::{parse_date, parse_previous_hash};
use crate::engram::worklog::WorklogEntry;
use crate::utils::hash::{sha256_hex, sha256_short};

const ENGRAM_DIR: &str = ".engram";
const WORKLOG_DIR: &str = ".engram/worklog";

/// Exit codes per spec
const EXIT_SUCCESS: i32 = 0;
const EXIT_CHAIN_BROKEN: i32 = 1;
const EXIT_NOT_INITIALIZED: i32 = 2;

/// Result of a successful verification
#[derive(Debug)]
pub struct VerifyResult {
    pub entry_count: usize,
    pub first_entry: Option<(String, String)>, // (filename, date)
    pub latest_entry: Option<(String, String)>, // (filename, date)
}

/// Error types for verification failures
#[derive(Debug)]
pub enum VerifyError {
    /// Engram not initialized
    NotInitialized,
    /// Chain linkage broken - Previous hash mismatch
    ChainBroken {
        filename: String,
        expected: String,
        found: String,
    },
    /// Content hash doesn't match filename hash
    HashMismatch {
        filename: String,
        content_hash: String,
        filename_hash: String,
    },
    /// Missing Previous: line in entry
    MissingPreviousLine(String),
    /// I/O error
    IoError(io::Error),
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerifyError::NotInitialized => {
                write!(f, "Engram not initialized. Run `engram init` first.")
            }
            VerifyError::ChainBroken {
                filename,
                expected,
                found,
            } => {
                write!(
                    f,
                    "Chain broken at entry {}\nExpected Previous: {}\nFound Previous: {}",
                    filename, expected, found
                )
            }
            VerifyError::HashMismatch {
                filename,
                content_hash,
                filename_hash,
            } => {
                write!(
                    f,
                    "Hash mismatch at {}\nContent hashes to: {}\nFilename claims: {}",
                    filename, content_hash, filename_hash
                )
            }
            VerifyError::MissingPreviousLine(filename) => {
                write!(f, "Missing 'Previous:' line in {}", filename)
            }
            VerifyError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for VerifyError {}

impl From<io::Error> for VerifyError {
    fn from(error: io::Error) -> Self {
        VerifyError::IoError(error)
    }
}

pub fn run() -> io::Result<()> {
    match verify_chain() {
        Ok(result) => {
            println!("✓ Chain verified: {} entries", result.entry_count);
            if let Some((first_file, first_date)) = result.first_entry {
                println!("  First: {} ({})", first_file, first_date);
            }
            if let Some((latest_file, latest_date)) = result.latest_entry {
                println!("  Latest: {} ({})", latest_file, latest_date);
            }
            process::exit(EXIT_SUCCESS);
        }
        Err(VerifyError::NotInitialized) => {
            eprintln!("Engram not initialized. Run `engram init` first.");
            process::exit(EXIT_NOT_INITIALIZED);
        }
        Err(VerifyError::ChainBroken {
            filename,
            expected,
            found,
        }) => {
            eprintln!("✗ Chain broken at entry {}", filename);
            eprintln!();
            eprintln!("Expected Previous: {}", expected);
            eprintln!("Found Previous:    {}", found);
            eprintln!();
            eprintln!("The worklog has been tampered with or corrupted.");
            process::exit(EXIT_CHAIN_BROKEN);
        }
        Err(VerifyError::HashMismatch {
            filename,
            content_hash,
            filename_hash,
        }) => {
            eprintln!("✗ Hash mismatch at {}", filename);
            eprintln!();
            eprintln!("Content hashes to: {}", content_hash);
            eprintln!("Filename claims:   {}", filename_hash);
            eprintln!();
            eprintln!("The worklog has been tampered with or corrupted.");
            process::exit(EXIT_CHAIN_BROKEN);
        }
        Err(VerifyError::MissingPreviousLine(filename)) => {
            eprintln!("✗ Invalid entry: Missing 'Previous:' line in {}", filename);
            process::exit(EXIT_CHAIN_BROKEN);
        }
        Err(VerifyError::IoError(e)) => {
            eprintln!("Error: {}", e);
            process::exit(EXIT_CHAIN_BROKEN);
        }
    }
}

/// Internal verification logic that can be tested
pub fn verify_chain() -> Result<VerifyResult, VerifyError> {
    verify_chain_in_dir(Path::new("."))
}

/// Verification logic with configurable base directory for testing
pub fn verify_chain_in_dir(base_dir: &Path) -> Result<VerifyResult, VerifyError> {
    let engram_dir = base_dir.join(ENGRAM_DIR);
    let worklog_dir = base_dir.join(WORKLOG_DIR);

    // 1. Validate environment
    if !engram_dir.exists() || !worklog_dir.exists() {
        return Err(VerifyError::NotInitialized);
    }

    // 2. List and sort entries by sequence number
    let mut entries = collect_entries(&worklog_dir)?;

    if entries.is_empty() {
        return Ok(VerifyResult {
            entry_count: 0,
            first_entry: None,
            latest_entry: None,
        });
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
        let embedded_prev = parse_previous_hash(&content)
            .ok_or_else(|| VerifyError::MissingPreviousLine(entry.filename.clone()))?;

        // Check chain linkage
        if embedded_prev != expected_prev {
            return Err(VerifyError::ChainBroken {
                filename: entry.filename.clone(),
                expected: expected_prev,
                found: embedded_prev,
            });
        }

        // Check filename hash matches content hash
        let content_hash = sha256_hex(&content);
        let content_short_hash = sha256_short(&content);

        if content_short_hash != entry.short_hash {
            return Err(VerifyError::HashMismatch {
                filename: entry.filename.clone(),
                content_hash: content_short_hash,
                filename_hash: entry.short_hash.clone(),
            });
        }

        // Track first entry info
        if first_entry.is_none() {
            let date = parse_date(&content).unwrap_or_else(|| "unknown".to_string());
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

    Ok(VerifyResult {
        entry_count: entries.len(),
        first_entry,
        latest_entry,
    })
}

/// Collect all valid worklog entries from the worklog directory
fn collect_entries(history_path: &Path) -> io::Result<Vec<WorklogEntry>> {
    let mut entries = Vec::new();

    for dir_entry in fs::read_dir(history_path)? {
        let dir_entry = dir_entry?;
        let filename = dir_entry.file_name();
        let filename_str = filename.to_string_lossy();

        // Only process valid entry files (NNNNNN_HHHHHHHH.md pattern)
        if let Some(entry) = WorklogEntry::from_filename(&filename_str, history_path) {
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
        fs::write(history_path.join("000001_a1b2c3d4.md"), "content").unwrap();
        fs::write(history_path.join("000002_e5f6a7b8.md"), "content").unwrap();
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
        fs::write(history_path.join("000003_11111111.md"), "content").unwrap();
        fs::write(history_path.join("000001_a1b2c3d4.md"), "content").unwrap();
        fs::write(history_path.join("000002_e5f6a7b8.md"), "content").unwrap();

        let mut entries = collect_entries(&history_path).unwrap();
        entries.sort_by_key(|e| e.sequence);

        assert_eq!(entries[0].sequence, 1);
        assert_eq!(entries[1].sequence, 2);
        assert_eq!(entries[2].sequence, 3);
    }

    // Tests for verify_chain_in_dir

    #[test]
    fn test_verify_fails_if_not_initialized() {
        let dir = tempdir().unwrap();
        // Don't create .engram directory

        let result = verify_chain_in_dir(dir.path());
        assert!(matches!(result, Err(VerifyError::NotInitialized)));
    }

    #[test]
    fn test_verify_fails_if_history_missing() {
        let dir = tempdir().unwrap();
        // Create .engram but not history
        fs::create_dir(dir.path().join(".engram")).unwrap();

        let result = verify_chain_in_dir(dir.path());
        assert!(matches!(result, Err(VerifyError::NotInitialized)));
    }

    #[test]
    fn test_verify_empty_chain() {
        let dir = tempdir().unwrap();
        setup_engram_dir(dir.path());

        let result = verify_chain_in_dir(dir.path());
        assert!(result.is_ok());

        let verify_result = result.unwrap();
        assert_eq!(verify_result.entry_count, 0);
        assert!(verify_result.first_entry.is_none());
        assert!(verify_result.latest_entry.is_none());
    }

    #[test]
    fn test_verify_valid_single_entry() {
        let dir = tempdir().unwrap();
        setup_engram_dir(dir.path());

        // Create a valid first entry with correct hash in filename
        let content = "Summary: First entry\nPrevious: none\nDate: 2025-06-12T14:32:07Z\n\n---\n\nBody content";
        let short_hash = sha256_short(content);
        let filename = format!("000001_{}.md", short_hash);
        fs::write(dir.path().join(".engram/worklog").join(&filename), content).unwrap();

        let result = verify_chain_in_dir(dir.path());
        assert!(result.is_ok());

        let verify_result = result.unwrap();
        assert_eq!(verify_result.entry_count, 1);
        assert!(verify_result.first_entry.is_some());
        assert!(verify_result.latest_entry.is_some());
        assert_eq!(verify_result.first_entry.as_ref().unwrap().0, filename);
        assert_eq!(verify_result.first_entry.as_ref().unwrap().1, "2025-06-12");
    }

    #[test]
    fn test_verify_valid_chain_multiple_entries() {
        let dir = tempdir().unwrap();
        setup_engram_dir(dir.path());

        // Create first entry
        let content1 =
            "Summary: First entry\nPrevious: none\nDate: 2025-06-12T14:32:07Z\n\n---\n\nBody 1";
        let short_hash1 = sha256_short(content1);
        let filename1 = format!("000001_{}.md", short_hash1);
        fs::write(
            dir.path().join(".engram/worklog").join(&filename1),
            content1,
        )
        .unwrap();

        // Create second entry with correct previous hash
        let prev_hash = sha256_hex(content1);
        let content2 = format!(
            "Summary: Second entry\nPrevious: {}\nDate: 2025-06-13T10:00:00Z\n\n---\n\nBody 2",
            prev_hash
        );
        let short_hash2 = sha256_short(&content2);
        let filename2 = format!("000002_{}.md", short_hash2);
        fs::write(
            dir.path().join(".engram/worklog").join(&filename2),
            &content2,
        )
        .unwrap();

        let result = verify_chain_in_dir(dir.path());
        assert!(result.is_ok());

        let verify_result = result.unwrap();
        assert_eq!(verify_result.entry_count, 2);
        assert_eq!(verify_result.first_entry.as_ref().unwrap().0, filename1);
        assert_eq!(verify_result.latest_entry.as_ref().unwrap().0, filename2);
    }

    #[test]
    fn test_verify_chain_broken_wrong_previous() {
        let dir = tempdir().unwrap();
        setup_engram_dir(dir.path());

        // Create first entry
        let content1 =
            "Summary: First entry\nPrevious: none\nDate: 2025-06-12T14:32:07Z\n\n---\n\nBody 1";
        let short_hash1 = sha256_short(content1);
        let filename1 = format!("000001_{}.md", short_hash1);
        fs::write(
            dir.path().join(".engram/worklog").join(&filename1),
            content1,
        )
        .unwrap();

        // Create second entry with WRONG previous hash
        let wrong_prev = "0000000000000000000000000000000000000000000000000000000000000000";
        let content2 = format!(
            "Summary: Second entry\nPrevious: {}\nDate: 2025-06-13T10:00:00Z\n\n---\n\nBody 2",
            wrong_prev
        );
        let short_hash2 = sha256_short(&content2);
        let filename2 = format!("000002_{}.md", short_hash2);
        fs::write(
            dir.path().join(".engram/worklog").join(&filename2),
            &content2,
        )
        .unwrap();

        let result = verify_chain_in_dir(dir.path());
        assert!(result.is_err());

        match result {
            Err(VerifyError::ChainBroken {
                filename,
                expected,
                found,
            }) => {
                assert_eq!(filename, filename2);
                assert_eq!(expected, sha256_hex(content1));
                assert_eq!(found, wrong_prev);
            }
            _ => panic!("Expected ChainBroken error"),
        }
    }

    #[test]
    fn test_verify_chain_broken_first_entry_not_none() {
        let dir = tempdir().unwrap();
        setup_engram_dir(dir.path());

        // Create first entry with wrong Previous (should be "none")
        let content = "Summary: First entry\nPrevious: 0000000000000000000000000000000000000000000000000000000000000000\nDate: 2025-06-12T14:32:07Z\n\n---\n\nBody";
        let short_hash = sha256_short(content);
        let filename = format!("000001_{}.md", short_hash);
        fs::write(dir.path().join(".engram/worklog").join(&filename), content).unwrap();

        let result = verify_chain_in_dir(dir.path());
        assert!(result.is_err());

        match result {
            Err(VerifyError::ChainBroken {
                expected, found, ..
            }) => {
                assert_eq!(expected, "none");
                assert_eq!(
                    found,
                    "0000000000000000000000000000000000000000000000000000000000000000"
                );
            }
            _ => panic!("Expected ChainBroken error"),
        }
    }

    #[test]
    fn test_verify_hash_mismatch() {
        let dir = tempdir().unwrap();
        setup_engram_dir(dir.path());

        // Create entry with correct Previous but wrong filename hash
        let content =
            "Summary: First entry\nPrevious: none\nDate: 2025-06-12T14:32:07Z\n\n---\n\nBody";
        // Use a wrong hash in filename (not matching content)
        let wrong_hash = "00000000";
        let filename = format!("000001_{}.md", wrong_hash);
        fs::write(dir.path().join(".engram/worklog").join(&filename), content).unwrap();

        let result = verify_chain_in_dir(dir.path());
        assert!(result.is_err());

        match result {
            Err(VerifyError::HashMismatch {
                filename: f,
                content_hash,
                filename_hash,
            }) => {
                assert_eq!(f, filename);
                assert_eq!(content_hash, sha256_short(content));
                assert_eq!(filename_hash, wrong_hash);
            }
            _ => panic!("Expected HashMismatch error"),
        }
    }

    #[test]
    fn test_verify_missing_previous_line() {
        let dir = tempdir().unwrap();
        setup_engram_dir(dir.path());

        // Create entry without Previous: line
        let content = "Summary: First entry\nDate: 2025-06-12T14:32:07Z\n\n---\n\nBody";
        let short_hash = sha256_short(content);
        let filename = format!("000001_{}.md", short_hash);
        fs::write(dir.path().join(".engram/worklog").join(&filename), content).unwrap();

        let result = verify_chain_in_dir(dir.path());
        assert!(result.is_err());

        match result {
            Err(VerifyError::MissingPreviousLine(f)) => {
                assert_eq!(f, filename);
            }
            _ => panic!("Expected MissingPreviousLine error"),
        }
    }

    /// Helper to set up a valid .engram directory structure for testing
    fn setup_engram_dir(base: &std::path::Path) {
        fs::create_dir(base.join(".engram")).unwrap();
        fs::create_dir(base.join(".engram/worklog")).unwrap();
    }
}
