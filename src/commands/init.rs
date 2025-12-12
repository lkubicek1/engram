use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::templates::{
    AGENTS_TEMPLATE, DRAFT_TEMPLATE, ROOT_DIRECTIVE_TEMPLATE, SUMMARY_TEMPLATE,
    WRAPPER_CMD_TEMPLATE, WRAPPER_SH_TEMPLATE,
};

/// Directory name for engram data
const ENGRAM_DIR: &str = ".engram";
/// Directory name for worklog entries
const WORKLOG_DIR: &str = "worklog";
/// Marker to detect if Engram directive already exists in a file
const ENGRAM_MARKER: &str = "Engram Protocol";

#[derive(Debug, Clone, Default)]
pub struct InitOptions {
    pub warp: bool,
    pub claude: bool,
    pub junie: bool,
    pub agents: bool,
    pub all: bool,
}

impl InitOptions {
    /// Returns true if any flag is set
    fn any_flag_set(&self) -> bool {
        self.warp || self.claude || self.junie || self.agents || self.all
    }
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
                write!(f, "Engram already initialized (found .engram/).")
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

pub fn run(options: InitOptions) -> io::Result<()> {
    let cwd = std::env::current_dir()?;
    match run_init_in_dir(&cwd, options) {
        Ok(()) => Ok(()),
        Err(InitError::AlreadyInitialized) => {
            eprintln!("Error: Engram already initialized (found .engram/).");
            std::process::exit(1);
        }
        Err(InitError::IoError(e)) => {
            eprintln!("Error: I/O error: {}", e);
            std::process::exit(2);
        }
    }
}

/// Internal implementation that accepts a base directory path.
/// This is used by tests to avoid race conditions with `set_current_dir`.
fn run_init_in_dir(cwd: &Path, options: InitOptions) -> Result<(), InitError> {
    let engram_dir = cwd.join(ENGRAM_DIR);
    let worklog_dir = engram_dir.join(WORKLOG_DIR);

    // Check idempotency: if .engram/ already exists, return error
    if engram_dir.exists() {
        return Err(InitError::AlreadyInitialized);
    }

    // Create directory structure
    fs::create_dir(&engram_dir)?;
    fs::create_dir(&worklog_dir)?;

    // Create .engram/AGENTS.md with full protocol instructions
    let agents_path = engram_dir.join("AGENTS.md");
    fs::write(&agents_path, AGENTS_TEMPLATE)?;

    // Create .engram/draft.md with empty template
    let draft_path = engram_dir.join("draft.md");
    fs::write(&draft_path, DRAFT_TEMPLATE)?;

    // Create .engram/worklog/SUMMARY.md with header only
    let summary_path = worklog_dir.join("SUMMARY.md");
    fs::write(&summary_path, SUMMARY_TEMPLATE)?;

    // Create .engram/.gitignore (ignore downloaded binaries)
    let engram_gitignore_path = engram_dir.join(".gitignore");
    fs::write(&engram_gitignore_path, "bin/\n")?;

    // Create .engram/.gitattributes (force LF line endings for stable hashing)
    let engram_gitattributes_path = engram_dir.join(".gitattributes");
    fs::write(&engram_gitattributes_path, "* text eol=lf\n")?;

    // Create per-repo wrapper scripts (so fresh clones can run `./engram ...`)
    let wrapper_report = write_wrappers(cwd)?;

    // Print success output
    println!("Initialized Engram in {}", cwd.display());
    println!("Created: {}", relative_path(cwd, &agents_path));
    println!("Created: {}", relative_path(cwd, &draft_path));
    println!("Created: {}", relative_path(cwd, &summary_path));
    println!("Created: {}", relative_path(cwd, &engram_gitignore_path));
    println!(
        "Created: {}",
        relative_path(cwd, &engram_gitattributes_path)
    );

    match wrapper_report.sh_status {
        WriteStatus::Created => {
            println!("Created: {}", relative_path(cwd, &wrapper_report.sh_path))
        }
        WriteStatus::SkippedAlreadyExists => println!(
            "Skipped: {} (already exists)",
            relative_path(cwd, &wrapper_report.sh_path)
        ),
    }

    match wrapper_report.cmd_status {
        WriteStatus::Created => {
            println!("Created: {}", relative_path(cwd, &wrapper_report.cmd_path))
        }
        WriteStatus::SkippedAlreadyExists => println!(
            "Skipped: {} (already exists)",
            relative_path(cwd, &wrapper_report.cmd_path)
        ),
    }

    // Handle root-level AI agent instruction files
    handle_root_level_files(cwd, &options)?;

    Ok(())
}

/// Handle creation/appending of root-level AI agent instruction files
fn handle_root_level_files(cwd: &Path, options: &InitOptions) -> Result<(), InitError> {
    if options.any_flag_set() {
        // Flag mode: create/append to specified files
        if options.warp {
            handle_warp_file(cwd)?;
        }
        if options.claude {
            handle_claude_file(cwd)?;
        }
        if options.junie {
            handle_junie_file(cwd)?;
        }
        if options.agents {
            handle_root_agents_file(cwd)?;
        }
    } else {
        // Detection mode: check for existing files and apply defaults
        let warp_exists = cwd.join("WARP.md").exists();
        let claude_exists = cwd.join("CLAUDE.md").exists();
        let junie_dir_exists = cwd.join(".junie").exists();

        if warp_exists {
            // WARP.md exists, append to it
            handle_warp_file(cwd)?;
        }

        if claude_exists {
            // CLAUDE.md exists, append to it
            handle_claude_file(cwd)?;
        }

        if junie_dir_exists {
            // .junie/ directory exists, append to guidelines.md
            handle_junie_file(cwd)?;
        }

        if !warp_exists && !claude_exists && !junie_dir_exists {
            // None exist, create AGENTS.md in project root by default
            handle_root_agents_file(cwd)?;
        }
    }

    Ok(())
}

/// Handle WARP.md file (create or append)
fn handle_warp_file(cwd: &Path) -> Result<(), InitError> {
    let warp_path = cwd.join("WARP.md");
    handle_directive_file(&warp_path, "WARP.md", "# Warp AI Instructions")
}

/// Handle CLAUDE.md file (create or append)
fn handle_claude_file(cwd: &Path) -> Result<(), InitError> {
    let claude_path = cwd.join("CLAUDE.md");
    handle_directive_file(&claude_path, "CLAUDE.md", "# Claude AI Instructions")
}

/// Handle .junie/guidelines.md file (create or append)
fn handle_junie_file(cwd: &Path) -> Result<(), InitError> {
    let junie_dir = cwd.join(".junie");
    let guidelines_path = junie_dir.join("guidelines.md");

    // Create .junie directory if it doesn't exist
    if !junie_dir.exists() {
        fs::create_dir(&junie_dir)?;
    }

    handle_directive_file(
        &guidelines_path,
        ".junie/guidelines.md",
        "# Junie AI Guidelines",
    )
}

/// Handle root AGENTS.md file (create or append)
fn handle_root_agents_file(cwd: &Path) -> Result<(), InitError> {
    let agents_path = cwd.join("AGENTS.md");
    handle_directive_file(&agents_path, "AGENTS.md", "# AI Agent Instructions")
}

/// Generic handler for directive files - creates or appends as needed
fn handle_directive_file(
    path: &Path,
    display_name: &str,
    default_header: &str,
) -> Result<(), InitError> {
    if path.exists() {
        // File exists - check for existing directive and append if not present
        let content = fs::read_to_string(path)?;

        // Idempotency check: don't append if directive already exists
        if content.contains(ENGRAM_MARKER) {
            println!(
                "Skipped: {} (Engram directive already present)",
                display_name
            );
            return Ok(());
        }

        // Append directive after the first heading (if any)
        let new_content = append_directive_after_heading(&content);
        fs::write(path, new_content)?;
        println!("Appended Engram directive to: {}", display_name);
    } else {
        // File doesn't exist - create with header and directive
        let content = format!("{}\n\n{}", default_header, ROOT_DIRECTIVE_TEMPLATE);
        fs::write(path, content)?;
        println!("Created: {}", display_name);
    }

    Ok(())
}

/// Append the directive after the first level-1 heading, or at the start if no heading found
fn append_directive_after_heading(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();

    // Find the first level-1 heading (starts with "# ")
    let heading_index = lines.iter().position(|line| line.starts_with("# "));

    match heading_index {
        Some(idx) => {
            // Insert directive after the heading line
            let mut result = String::new();

            // Add lines up to and including the heading
            for line in &lines[..=idx] {
                result.push_str(line);
                result.push('\n');
            }

            // Add blank line and directive
            result.push('\n');
            result.push_str(ROOT_DIRECTIVE_TEMPLATE);

            // Add remaining content
            if idx + 1 < lines.len() {
                for line in &lines[idx + 1..] {
                    result.push_str(line);
                    result.push('\n');
                }
            }

            result
        }
        None => {
            // No heading found, prepend directive
            format!("{}\n{}", ROOT_DIRECTIVE_TEMPLATE, content)
        }
    }
}

enum WriteStatus {
    Created,
    SkippedAlreadyExists,
}

struct WrapperWriteReport {
    sh_path: PathBuf,
    sh_status: WriteStatus,
    cmd_path: PathBuf,
    cmd_status: WriteStatus,
}

fn write_wrappers(cwd: &Path) -> io::Result<WrapperWriteReport> {
    let version = env!("CARGO_PKG_VERSION");

    let sh_path = cwd.join("engram");
    let sh_status = if sh_path.exists() {
        WriteStatus::SkippedAlreadyExists
    } else {
        let wrapper_sh = WRAPPER_SH_TEMPLATE.replace("__ENGRAM_VERSION__", version);
        fs::write(&sh_path, wrapper_sh)?;
        set_executable(&sh_path)?;
        WriteStatus::Created
    };

    let cmd_path = cwd.join("engram.cmd");
    let cmd_status = if cmd_path.exists() {
        WriteStatus::SkippedAlreadyExists
    } else {
        let wrapper_cmd = WRAPPER_CMD_TEMPLATE.replace("__ENGRAM_VERSION__", version);
        fs::write(&cmd_path, wrapper_cmd)?;
        WriteStatus::Created
    };

    Ok(WrapperWriteReport {
        sh_path,
        sh_status,
        cmd_path,
        cmd_status,
    })
}

#[cfg(unix)]
fn set_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> io::Result<()> {
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

        let result = run_init_in_dir(temp_dir.path(), InitOptions::default());
        assert!(result.is_ok());

        // Verify directory structure
        assert!(temp_dir.path().join(".engram").exists());
        assert!(temp_dir.path().join(".engram").is_dir());
        assert!(temp_dir.path().join(".engram/worklog").exists());
        assert!(temp_dir.path().join(".engram/worklog").is_dir());

        // Verify wrapper scripts
        assert!(temp_dir.path().join("engram").exists());
        assert!(temp_dir.path().join("engram.cmd").exists());

        // Verify hygiene files
        assert!(temp_dir.path().join(".engram/.gitignore").exists());
        assert!(temp_dir.path().join(".engram/.gitattributes").exists());
    }

    #[test]
    fn test_init_creates_agents_md() {
        let temp_dir = TempDir::new().unwrap();

        let result = run_init_in_dir(temp_dir.path(), InitOptions::default());
        assert!(result.is_ok());

        let agents_path = temp_dir.path().join(".engram/AGENTS.md");
        assert!(agents_path.exists());
        let content = fs::read_to_string(&agents_path).unwrap();
        assert!(content.contains("Engram Protocol: Agent Instructions"));
    }

    #[test]
    fn test_init_creates_draft_md() {
        let temp_dir = TempDir::new().unwrap();

        let result = run_init_in_dir(temp_dir.path(), InitOptions::default());
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

        let result = run_init_in_dir(temp_dir.path(), InitOptions::default());
        assert!(result.is_ok());

        let summary_path = temp_dir.path().join(".engram/worklog/SUMMARY.md");
        assert!(summary_path.exists());
        let content = fs::read_to_string(&summary_path).unwrap();
        assert!(content.contains("| Entry | Summary |"));
    }

    #[test]
    fn test_init_idempotency_check() {
        let temp_dir = TempDir::new().unwrap();

        // First init should succeed
        let result = run_init_in_dir(temp_dir.path(), InitOptions::default());
        assert!(result.is_ok());

        // Second init should fail with AlreadyInitialized
        let result = run_init_in_dir(temp_dir.path(), InitOptions::default());
        assert!(matches!(result, Err(InitError::AlreadyInitialized)));
    }

    #[test]
    fn test_init_fails_if_engram_dir_exists() {
        let temp_dir = TempDir::new().unwrap();

        // Manually create .engram directory
        fs::create_dir(temp_dir.path().join(".engram")).unwrap();

        // Init should fail
        let result = run_init_in_dir(temp_dir.path(), InitOptions::default());
        assert!(matches!(result, Err(InitError::AlreadyInitialized)));
    }

    // === New tests for Phase 2 Task 2: Init extensions ===

    #[test]
    fn test_init_with_warp_flag_creates_warp_md() {
        let temp_dir = TempDir::new().unwrap();

        let options = InitOptions {
            warp: true,
            ..Default::default()
        };
        let result = run_init_in_dir(temp_dir.path(), options);
        assert!(result.is_ok());

        let warp_path = temp_dir.path().join("WARP.md");
        assert!(warp_path.exists());
        let content = fs::read_to_string(&warp_path).unwrap();
        assert!(content.contains("Engram Protocol"));
        assert!(content.contains("# Warp AI Instructions"));
    }

    #[test]
    fn test_init_with_claude_flag_creates_claude_md() {
        let temp_dir = TempDir::new().unwrap();

        let options = InitOptions {
            claude: true,
            ..Default::default()
        };
        let result = run_init_in_dir(temp_dir.path(), options);
        assert!(result.is_ok());

        let claude_path = temp_dir.path().join("CLAUDE.md");
        assert!(claude_path.exists());
        let content = fs::read_to_string(&claude_path).unwrap();
        assert!(content.contains("Engram Protocol"));
        assert!(content.contains("# Claude AI Instructions"));
    }

    #[test]
    fn test_init_with_junie_flag_creates_guidelines_md() {
        let temp_dir = TempDir::new().unwrap();

        let options = InitOptions {
            junie: true,
            ..Default::default()
        };
        let result = run_init_in_dir(temp_dir.path(), options);
        assert!(result.is_ok());

        let guidelines_path = temp_dir.path().join(".junie/guidelines.md");
        assert!(guidelines_path.exists());
        let content = fs::read_to_string(&guidelines_path).unwrap();
        assert!(content.contains("Engram Protocol"));
        assert!(content.contains("# Junie AI Guidelines"));
    }

    #[test]
    fn test_init_with_agents_flag_creates_root_agents_md() {
        let temp_dir = TempDir::new().unwrap();

        let options = InitOptions {
            agents: true,
            ..Default::default()
        };
        let result = run_init_in_dir(temp_dir.path(), options);
        assert!(result.is_ok());

        let agents_path = temp_dir.path().join("AGENTS.md");
        assert!(agents_path.exists());
        let content = fs::read_to_string(&agents_path).unwrap();
        assert!(content.contains("Engram Protocol"));
        assert!(content.contains("# AI Agent Instructions"));
    }

    #[test]
    fn test_init_with_all_flag_creates_all_files() {
        let temp_dir = TempDir::new().unwrap();

        let options = InitOptions {
            all: true,
            warp: true,
            claude: true,
            junie: true,
            agents: true,
        };
        let result = run_init_in_dir(temp_dir.path(), options);
        assert!(result.is_ok());

        // Check all files exist
        assert!(temp_dir.path().join("WARP.md").exists());
        assert!(temp_dir.path().join("CLAUDE.md").exists());
        assert!(temp_dir.path().join(".junie/guidelines.md").exists());
        assert!(temp_dir.path().join("AGENTS.md").exists());
    }

    #[test]
    fn test_init_appends_to_existing_warp_md() {
        let temp_dir = TempDir::new().unwrap();

        // Create existing WARP.md
        let warp_path = temp_dir.path().join("WARP.md");
        fs::write(
            &warp_path,
            "# My Warp Instructions\n\nSome existing content.\n",
        )
        .unwrap();

        let options = InitOptions {
            warp: true,
            ..Default::default()
        };
        let result = run_init_in_dir(temp_dir.path(), options);
        assert!(result.is_ok());

        let content = fs::read_to_string(&warp_path).unwrap();
        assert!(content.contains("# My Warp Instructions"));
        assert!(content.contains("Engram Protocol"));
        assert!(content.contains("Some existing content."));
    }

    #[test]
    fn test_init_idempotency_skips_if_directive_exists() {
        let temp_dir = TempDir::new().unwrap();

        // Create existing WARP.md with Engram Protocol already present
        let warp_path = temp_dir.path().join("WARP.md");
        fs::write(
            &warp_path,
            "# Warp\n\n## Engram Protocol\n\nAlready here.\n",
        )
        .unwrap();

        let options = InitOptions {
            warp: true,
            ..Default::default()
        };
        let result = run_init_in_dir(temp_dir.path(), options);
        assert!(result.is_ok());

        // Content should not be duplicated
        let content = fs::read_to_string(&warp_path).unwrap();
        let count = content.matches("Engram Protocol").count();
        assert_eq!(count, 1, "Directive should not be duplicated");
    }

    #[test]
    fn test_init_detection_mode_with_existing_warp() {
        let temp_dir = TempDir::new().unwrap();

        // Create existing WARP.md (detection mode should append to it)
        let warp_path = temp_dir.path().join("WARP.md");
        fs::write(&warp_path, "# Warp\n\nExisting content.\n").unwrap();

        // No flags - detection mode
        let result = run_init_in_dir(temp_dir.path(), InitOptions::default());
        assert!(result.is_ok());

        // WARP.md should have directive appended
        let content = fs::read_to_string(&warp_path).unwrap();
        assert!(content.contains("Engram Protocol"));

        // Root AGENTS.md should NOT be created (WARP.md exists)
        assert!(!temp_dir.path().join("AGENTS.md").exists());
    }

    #[test]
    fn test_init_detection_mode_with_existing_junie_dir() {
        let temp_dir = TempDir::new().unwrap();

        // Create existing .junie directory (detection mode should create guidelines.md)
        fs::create_dir(temp_dir.path().join(".junie")).unwrap();

        // No flags - detection mode
        let result = run_init_in_dir(temp_dir.path(), InitOptions::default());
        assert!(result.is_ok());

        // guidelines.md should be created with directive
        let guidelines_path = temp_dir.path().join(".junie/guidelines.md");
        assert!(guidelines_path.exists());
        let content = fs::read_to_string(&guidelines_path).unwrap();
        assert!(content.contains("Engram Protocol"));

        // Root AGENTS.md should NOT be created (.junie exists)
        assert!(!temp_dir.path().join("AGENTS.md").exists());
    }

    #[test]
    fn test_init_detection_mode_with_existing_claude() {
        let temp_dir = TempDir::new().unwrap();

        // Create existing CLAUDE.md (detection mode should append to it)
        let claude_path = temp_dir.path().join("CLAUDE.md");
        fs::write(&claude_path, "# Claude\n\nExisting content.\n").unwrap();

        // No flags - detection mode
        let result = run_init_in_dir(temp_dir.path(), InitOptions::default());
        assert!(result.is_ok());

        // CLAUDE.md should have directive appended
        let content = fs::read_to_string(&claude_path).unwrap();
        assert!(content.contains("Existing content"));
        assert!(content.contains("Engram Protocol"));

        // Root AGENTS.md should NOT be created (CLAUDE.md exists)
        assert!(!temp_dir.path().join("AGENTS.md").exists());
    }

    #[test]
    fn test_init_detection_mode_default_creates_root_agents() {
        let temp_dir = TempDir::new().unwrap();

        // No flags, no existing WARP.md or .junie - should create root AGENTS.md
        let result = run_init_in_dir(temp_dir.path(), InitOptions::default());
        assert!(result.is_ok());

        let agents_path = temp_dir.path().join("AGENTS.md");
        assert!(agents_path.exists());
        let content = fs::read_to_string(&agents_path).unwrap();
        assert!(content.contains("Engram Protocol"));
    }

    #[test]
    fn test_append_directive_after_heading() {
        let content = "# My Title\n\nSome content here.\n\n## Section\n\nMore content.\n";
        let result = append_directive_after_heading(content);

        // Should have heading first, then directive, then rest of content
        assert!(result.starts_with("# My Title\n"));
        assert!(result.contains("Engram Protocol"));
        assert!(result.contains("Some content here."));
    }

    #[test]
    fn test_append_directive_no_heading() {
        let content = "Just some content without a heading.\n";
        let result = append_directive_after_heading(content);

        // Directive should be prepended
        assert!(result.starts_with("## 🔒 Engram Protocol"));
        assert!(result.contains("Just some content without a heading."));
    }
}
