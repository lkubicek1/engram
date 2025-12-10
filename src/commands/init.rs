use std::fs;
use std::io;
use std::path::Path;

use crate::templates::{AGENTS_TEMPLATE, DRAFT_TEMPLATE, SUMMARY_TEMPLATE};

/// Directory name for engram data
const ENGRAM_DIR: &str = ".engram";
/// Directory name for history entries
const HISTORY_DIR: &str = "history";

#[derive(Debug, Clone, Default)]
pub struct InitOptions {
    pub warp: bool,
    pub junie: bool,
    pub agents: bool,
    pub all: bool,
}

/// Error type for init command with specific exit codes
#[derive(Debug)]
pub enum InitError {
    /// Already initialized (exit code 1)
    AlreadyInitialized,
    /// File I/O error (exit code 2)
    IoError(io::Error),
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitError::AlreadyInitialized => {
                write!(f, "Engram already initialized. Use --force to reinitialize.")
            }
            InitError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for InitError {}

impl From<io::Error> for InitError {
    fn from(error: io::Error) -> Self {
        InitError::IoError(error)
    }
}

pub fn run(_options: InitOptions) -> io::Result<()> {
    match run_init() {
        Ok(()) => Ok(()),
        Err(InitError::AlreadyInitialized) => {
            eprintln!("Error: Engram already initialized. Use --force to reinitialize.");
            std::process::exit(1);
        }
        Err(InitError::IoError(e)) => {
            eprintln!("Error: I/O error: {}", e);
            std::process::exit(2);
        }
    }
}

fn run_init() -> Result<(), InitError> {
    let cwd = std::env::current_dir()?;
    let engram_dir = cwd.join(ENGRAM_DIR);
    let history_dir = engram_dir.join(HISTORY_DIR);

    // Check idempotency: if .engram/ already exists, return error
    if engram_dir.exists() {
        return Err(InitError::AlreadyInitialized);
    }

    // Create directory structure
    fs::create_dir(&engram_dir)?;
    fs::create_dir(&history_dir)?;

    // Create .engram/AGENTS.md with full protocol instructions
    let agents_path = engram_dir.join("AGENTS.md");
    fs::write(&agents_path, AGENTS_TEMPLATE)?;

    // Create .engram/draft.md with empty template
    let draft_path = engram_dir.join("draft.md");
    fs::write(&draft_path, DRAFT_TEMPLATE)?;

    // Create .engram/history/SUMMARY.md with header only
    let summary_path = history_dir.join("SUMMARY.md");
    fs::write(&summary_path, SUMMARY_TEMPLATE)?;

    // Print success output
    println!("Initialized Engram in {}", cwd.display());
    println!("Created: {}", relative_path(&cwd, &agents_path));
    println!("Created: {}", relative_path(&cwd, &draft_path));
    println!("Created: {}", relative_path(&cwd, &summary_path));

    Ok(())
}

/// Helper to display relative path from current directory
fn relative_path(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_init_creates_directory_structure() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = run_init();
        assert!(result.is_ok());

        // Verify directory structure
        assert!(temp_dir.path().join(".engram").exists());
        assert!(temp_dir.path().join(".engram").is_dir());
        assert!(temp_dir.path().join(".engram/history").exists());
        assert!(temp_dir.path().join(".engram/history").is_dir());
    }

    #[test]
    fn test_init_creates_agents_md() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = run_init();
        assert!(result.is_ok());

        let agents_path = temp_dir.path().join(".engram/AGENTS.md");
        assert!(agents_path.exists());
        let content = fs::read_to_string(&agents_path).unwrap();
        assert!(content.contains("Engram Protocol: Agent Instructions"));
    }

    #[test]
    fn test_init_creates_draft_md() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = run_init();
        assert!(result.is_ok());

        let draft_path = temp_dir.path().join(".engram/draft.md");
        assert!(draft_path.exists());
        let content = fs::read_to_string(&draft_path).unwrap();
        assert!(content.contains("<summary></summary>"));
        assert!(content.contains("## Intent"));
        assert!(content.contains("## Changes"));
        assert!(content.contains("## Verification"));
    }

    #[test]
    fn test_init_creates_summary_md() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = run_init();
        assert!(result.is_ok());

        let summary_path = temp_dir.path().join(".engram/history/SUMMARY.md");
        assert!(summary_path.exists());
        let content = fs::read_to_string(&summary_path).unwrap();
        assert!(content.contains("| Entry | Summary |"));
    }

    #[test]
    fn test_init_idempotency_check() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // First init should succeed
        let result = run_init();
        assert!(result.is_ok());

        // Second init should fail with AlreadyInitialized
        let result = run_init();
        assert!(matches!(result, Err(InitError::AlreadyInitialized)));
    }

    #[test]
    fn test_init_fails_if_engram_dir_exists() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Manually create .engram directory
        fs::create_dir(temp_dir.path().join(".engram")).unwrap();

        // Init should fail
        let result = run_init();
        assert!(matches!(result, Err(InitError::AlreadyInitialized)));
    }
}
