use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

/// Get the path to the engram binary
fn engram_bin() -> std::path::PathBuf {
    // The binary is built in target/debug/engram when running tests
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // Remove test binary name
    path.pop(); // Remove deps directory
    path.push("engram");
    path
}

/// Run engram command in a specific directory
fn run_engram(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(engram_bin())
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to execute engram command")
}

/// Helper to get stdout as string
fn stdout_str(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Helper to get stderr as string
fn stderr_str(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

// =============================================================================
// INIT COMMAND TESTS
// =============================================================================

#[test]
fn test_init_creates_engram_structure() {
    let dir = tempdir().unwrap();

    let output = run_engram(dir.path(), &["init"]);

    assert!(
        output.status.success(),
        "init should succeed: {}",
        stderr_str(&output)
    );

    // Verify directory structure
    assert!(dir.path().join(".engram").exists());
    assert!(dir.path().join(".engram").is_dir());
    assert!(dir.path().join(".engram/worklog").exists());
    assert!(dir.path().join(".engram/worklog").is_dir());

    // Verify files created
    assert!(dir.path().join(".engram/AGENTS.md").exists());
    assert!(dir.path().join(".engram/draft.md").exists());
    assert!(dir.path().join(".engram/worklog/SUMMARY.md").exists());
    assert!(dir.path().join(".engram/.gitignore").exists());
    assert!(dir.path().join(".engram/.gitattributes").exists());

    // Verify wrapper scripts created
    assert!(dir.path().join("engram").exists());
    assert!(dir.path().join("engram.cmd").exists());

    // Verify output message
    let stdout = stdout_str(&output);
    assert!(stdout.contains("Initialized Engram"));
    assert!(stdout.contains(".engram/AGENTS.md"));
    assert!(stdout.contains(".engram/draft.md"));
}

#[test]
fn test_init_idempotency_fails_on_second_run() {
    let dir = tempdir().unwrap();

    // First init should succeed
    let output1 = run_engram(dir.path(), &["init"]);
    assert!(output1.status.success());

    // Second init should fail
    let output2 = run_engram(dir.path(), &["init"]);
    assert!(!output2.status.success(), "second init should fail");

    let stderr = stderr_str(&output2);
    assert!(
        stderr.contains("already initialized"),
        "should mention already initialized: {}",
        stderr
    );
}

#[test]
fn test_init_with_warp_flag() {
    let dir = tempdir().unwrap();

    let output = run_engram(dir.path(), &["init", "--warp"]);

    assert!(output.status.success());
    assert!(dir.path().join("WARP.md").exists());

    let content = fs::read_to_string(dir.path().join("WARP.md")).unwrap();
    assert!(content.contains("Engram Protocol"));
}

#[test]
fn test_init_with_junie_flag() {
    let dir = tempdir().unwrap();

    let output = run_engram(dir.path(), &["init", "--junie"]);

    assert!(output.status.success());
    assert!(dir.path().join(".junie/guidelines.md").exists());

    let content = fs::read_to_string(dir.path().join(".junie/guidelines.md")).unwrap();
    assert!(content.contains("Engram Protocol"));
}

#[test]
fn test_init_with_agents_flag() {
    let dir = tempdir().unwrap();

    let output = run_engram(dir.path(), &["init", "--agents"]);

    assert!(output.status.success());
    assert!(dir.path().join("AGENTS.md").exists());

    let content = fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(content.contains("Engram Protocol"));
}

#[test]
fn test_init_with_all_flag() {
    let dir = tempdir().unwrap();

    let output = run_engram(dir.path(), &["init", "--all"]);

    assert!(output.status.success());
    assert!(dir.path().join("WARP.md").exists());
    assert!(dir.path().join(".junie/guidelines.md").exists());
    assert!(dir.path().join("AGENTS.md").exists());
}

#[test]
fn test_init_detection_mode_existing_warp() {
    let dir = tempdir().unwrap();

    // Create existing WARP.md
    fs::write(dir.path().join("WARP.md"), "# Warp\n\nExisting content.\n").unwrap();

    let output = run_engram(dir.path(), &["init"]);

    assert!(output.status.success());

    // Should append to WARP.md
    let content = fs::read_to_string(dir.path().join("WARP.md")).unwrap();
    assert!(content.contains("Existing content"));
    assert!(content.contains("Engram Protocol"));

    // Should NOT create root AGENTS.md
    assert!(!dir.path().join("AGENTS.md").exists());
}

#[test]
fn test_init_directive_idempotency() {
    let dir = tempdir().unwrap();

    // Create WARP.md with Engram Protocol already present
    fs::write(
        dir.path().join("WARP.md"),
        "# Warp\n\n## Engram Protocol\n\nAlready here.\n",
    )
    .unwrap();

    let output = run_engram(dir.path(), &["init", "--warp"]);

    assert!(output.status.success());

    // Should skip, not duplicate
    let content = fs::read_to_string(dir.path().join("WARP.md")).unwrap();
    let count = content.matches("Engram Protocol").count();
    assert_eq!(count, 1, "Directive should not be duplicated");

    let stdout = stdout_str(&output);
    assert!(stdout.contains("Skipped"), "Should mention skipped");
}

// =============================================================================
// COMMIT COMMAND TESTS
// =============================================================================

#[test]
fn test_commit_requires_init() {
    let dir = tempdir().unwrap();

    let output = run_engram(dir.path(), &["commit"]);

    assert!(!output.status.success());
    let stderr = stderr_str(&output);
    assert!(stderr.contains("not initialized") || stderr.contains("init"));
}

#[test]
fn test_commit_requires_valid_draft() {
    let dir = tempdir().unwrap();

    // Initialize first
    run_engram(dir.path(), &["init"]);

    // Try to commit with empty draft (default template)
    let output = run_engram(dir.path(), &["commit"]);

    assert!(!output.status.success());
    // Error should mention draft is empty or invalid
}

#[test]
fn test_commit_first_entry() {
    let dir = tempdir().unwrap();

    // Initialize
    run_engram(dir.path(), &["init"]);

    // Write valid draft
    let draft_content = r#"<summary>Initial project setup</summary>

## Intent
Setting up the project structure

## Changes
- Created main.rs
- Added Cargo.toml

## Verification
Compiled successfully with cargo build"#;
    fs::write(dir.path().join(".engram/draft.md"), draft_content).unwrap();

    // Commit
    let output = run_engram(dir.path(), &["commit"]);

    assert!(
        output.status.success(),
        "commit should succeed: {}",
        stderr_str(&output)
    );

    let stdout = stdout_str(&output);
    assert!(stdout.contains("Committed:"));
    assert!(stdout.contains("000001_")); // First entry
    assert!(stdout.contains("Initial project setup"));
    assert!(stdout.contains("Previous: none"));

    // Verify entry file was created
    let history_dir = dir.path().join(".engram/worklog");
    let entries: Vec<_> = fs::read_dir(&history_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("000001_"))
        .collect();
    assert_eq!(entries.len(), 1, "Should have one entry file");

    // Verify entry content
    let entry_content = fs::read_to_string(entries[0].path()).unwrap();
    assert!(entry_content.contains("Summary: Initial project setup"));
    assert!(entry_content.contains("Previous: none"));

    // Verify SUMMARY.md was updated
    let summary = fs::read_to_string(dir.path().join(".engram/worklog/SUMMARY.md")).unwrap();
    assert!(summary.contains("Initial project setup"));

    // Verify draft was reset
    let new_draft = fs::read_to_string(dir.path().join(".engram/draft.md")).unwrap();
    assert!(new_draft.contains("<summary></summary>"));
}

#[test]
fn test_commit_subsequent_entry() {
    let dir = tempdir().unwrap();

    // Initialize
    run_engram(dir.path(), &["init"]);

    // First commit
    let draft1 = r#"<summary>First commit</summary>

## Intent
First work session

## Changes
- Added feature A

## Verification
Tests pass"#;
    fs::write(dir.path().join(".engram/draft.md"), draft1).unwrap();
    let output1 = run_engram(dir.path(), &["commit"]);
    assert!(output1.status.success());

    // Second commit
    let draft2 = r#"<summary>Second commit</summary>

## Intent
Second work session

## Changes
- Added feature B

## Verification
Tests pass"#;
    fs::write(dir.path().join(".engram/draft.md"), draft2).unwrap();
    let output2 = run_engram(dir.path(), &["commit"]);

    assert!(output2.status.success());

    let stdout = stdout_str(&output2);
    assert!(stdout.contains("000002_")); // Second entry
    assert!(stdout.contains("Second commit"));
    assert!(!stdout.contains("Previous: none")); // Should have a hash

    // Verify chain linkage exists (previous hash is not "none")
    let history_dir = dir.path().join(".engram/worklog");
    let entries: Vec<_> = fs::read_dir(&history_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("000002_"))
        .collect();
    assert_eq!(entries.len(), 1);

    let entry_content = fs::read_to_string(entries[0].path()).unwrap();
    assert!(!entry_content.contains("Previous: none"));
}

// =============================================================================
// VERIFY COMMAND TESTS
// =============================================================================

#[test]
fn test_verify_requires_init() {
    let dir = tempdir().unwrap();

    let output = run_engram(dir.path(), &["verify"]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2)); // EXIT_NOT_INITIALIZED
}

#[test]
fn test_verify_empty_chain() {
    let dir = tempdir().unwrap();

    // Initialize
    run_engram(dir.path(), &["init"]);

    let output = run_engram(dir.path(), &["verify"]);

    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("✓"));
    assert!(stdout.contains("0 entries"));
}

#[test]
fn test_verify_valid_chain() {
    let dir = tempdir().unwrap();

    // Initialize and create entries
    run_engram(dir.path(), &["init"]);

    // First commit
    let draft1 = r#"<summary>First entry</summary>

## Intent
Test

## Changes
- Test

## Verification
Pass"#;
    fs::write(dir.path().join(".engram/draft.md"), draft1).unwrap();
    run_engram(dir.path(), &["commit"]);

    // Second commit
    let draft2 = r#"<summary>Second entry</summary>

## Intent
Test

## Changes
- Test

## Verification
Pass"#;
    fs::write(dir.path().join(".engram/draft.md"), draft2).unwrap();
    run_engram(dir.path(), &["commit"]);

    // Verify
    let output = run_engram(dir.path(), &["verify"]);

    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("✓"));
    assert!(stdout.contains("2 entries"));
    assert!(stdout.contains("First:"));
    assert!(stdout.contains("Latest:"));
}

#[test]
fn test_verify_detects_tampered_chain() {
    let dir = tempdir().unwrap();

    // Initialize and create an entry
    run_engram(dir.path(), &["init"]);

    let draft = r#"<summary>Test entry</summary>

## Intent
Test

## Changes
- Test

## Verification
Pass"#;
    fs::write(dir.path().join(".engram/draft.md"), draft).unwrap();
    run_engram(dir.path(), &["commit"]);

    // Tamper with the entry file (modify content)
    let history_dir = dir.path().join(".engram/worklog");
    let entry: std::path::PathBuf = fs::read_dir(&history_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with("000001_"))
        .map(|e| e.path())
        .unwrap();

    let original = fs::read_to_string(&entry).unwrap();
    let tampered = original.replace("Test entry", "TAMPERED entry");
    fs::write(&entry, tampered).unwrap();

    // Verify should fail
    let output = run_engram(dir.path(), &["verify"]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1)); // EXIT_CHAIN_BROKEN

    let stderr = stderr_str(&output);
    assert!(stderr.contains("✗") || stderr.contains("mismatch") || stderr.contains("Hash"));
}

// =============================================================================
// STATUS COMMAND TESTS
// =============================================================================

#[test]
fn test_status_requires_init() {
    let dir = tempdir().unwrap();

    let output = run_engram(dir.path(), &["status"]);

    assert!(!output.status.success());
    let stderr = stderr_str(&output);
    assert!(stderr.contains("not initialized") || stderr.contains("init"));
}

#[test]
fn test_status_empty_state() {
    let dir = tempdir().unwrap();

    // Initialize
    run_engram(dir.path(), &["init"]);

    let output = run_engram(dir.path(), &["status"]);

    assert!(output.status.success());

    let stdout = stdout_str(&output);
    assert!(stdout.contains("Engram Status"));
    assert!(stdout.contains("0 entries"));
    assert!(stdout.contains("Draft:"));
    assert!(stdout.contains("Empty") || stdout.contains("ready"));
    assert!(stdout.contains("Chain:"));
    assert!(stdout.contains("✓"));
}

#[test]
fn test_status_with_history() {
    let dir = tempdir().unwrap();

    // Initialize and commit
    run_engram(dir.path(), &["init"]);

    let draft = r#"<summary>Test work completed</summary>

## Intent
Test

## Changes
- Test

## Verification
Pass"#;
    fs::write(dir.path().join(".engram/draft.md"), draft).unwrap();
    run_engram(dir.path(), &["commit"]);

    let output = run_engram(dir.path(), &["status"]);

    assert!(output.status.success());

    let stdout = stdout_str(&output);
    assert!(stdout.contains("1 entries"));
    assert!(stdout.contains("Latest:"));
    assert!(stdout.contains("Test work completed"));
}

#[test]
fn test_status_with_uncommitted_draft() {
    let dir = tempdir().unwrap();

    // Initialize
    run_engram(dir.path(), &["init"]);

    // Write draft with content but don't commit
    let draft = r#"<summary>Work in progress</summary>

## Intent
Working on something

## Changes
- Started feature

## Verification
Not yet"#;
    fs::write(dir.path().join(".engram/draft.md"), draft).unwrap();

    let output = run_engram(dir.path(), &["status"]);

    assert!(output.status.success());

    let stdout = stdout_str(&output);
    assert!(stdout.contains("Has content") || stdout.contains("uncommitted"));
    assert!(stdout.contains("Work in progress"));
}

// =============================================================================
// HELP AND USAGE TESTS
// =============================================================================

#[test]
fn test_help_flag() {
    let dir = tempdir().unwrap();

    let output = run_engram(dir.path(), &["--help"]);

    assert!(output.status.success());

    let stdout = stdout_str(&output);
    assert!(stdout.contains("engram"));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("commit"));
    assert!(stdout.contains("verify"));
    assert!(stdout.contains("status"));
}

#[test]
fn test_version_flag() {
    let dir = tempdir().unwrap();

    let output = run_engram(dir.path(), &["--version"]);

    assert!(output.status.success());

    let stdout = stdout_str(&output);
    assert!(stdout.contains("engram"));
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn test_no_args_shows_help() {
    let dir = tempdir().unwrap();

    let output = run_engram(dir.path(), &[]);

    // Should show help/usage (not error)
    let stdout = stdout_str(&output);
    let stderr = stderr_str(&output);
    let combined = format!("{}{}", stdout, stderr);

    assert!(combined.contains("engram") || combined.contains("Usage"));
}

// =============================================================================
// END-TO-END WORKFLOW TEST
// =============================================================================

#[test]
fn test_full_workflow() {
    let dir = tempdir().unwrap();

    // 1. Initialize
    let output = run_engram(dir.path(), &["init"]);
    assert!(output.status.success(), "init failed");

    // 2. Check status (empty)
    let output = run_engram(dir.path(), &["status"]);
    assert!(output.status.success(), "status failed");
    assert!(stdout_str(&output).contains("0 entries"));

    // 3. Verify empty chain
    let output = run_engram(dir.path(), &["verify"]);
    assert!(output.status.success(), "verify failed");

    // 4. First commit
    let draft1 = r#"<summary>Project initialization</summary>

## Intent
Set up the project structure and dependencies

## Changes
- Created directory structure
- Added configuration files

## Verification
Build succeeds"#;
    fs::write(dir.path().join(".engram/draft.md"), draft1).unwrap();
    let output = run_engram(dir.path(), &["commit"]);
    assert!(output.status.success(), "first commit failed");
    assert!(stdout_str(&output).contains("000001_"));

    // 5. Verify after first commit
    let output = run_engram(dir.path(), &["verify"]);
    assert!(output.status.success(), "verify after first commit failed");
    assert!(stdout_str(&output).contains("1 entries"));

    // 6. Second commit
    let draft2 = r#"<summary>Added core functionality</summary>

## Intent
Implement the main feature

## Changes
- Added main module
- Implemented core logic

## Verification
All tests pass"#;
    fs::write(dir.path().join(".engram/draft.md"), draft2).unwrap();
    let output = run_engram(dir.path(), &["commit"]);
    assert!(output.status.success(), "second commit failed");
    assert!(stdout_str(&output).contains("000002_"));

    // 7. Final status check
    let output = run_engram(dir.path(), &["status"]);
    assert!(output.status.success(), "final status failed");
    let stdout = stdout_str(&output);
    assert!(stdout.contains("2 entries"));
    assert!(stdout.contains("Added core functionality"));
    assert!(stdout.contains("✓"));

    // 8. Final verification
    let output = run_engram(dir.path(), &["verify"]);
    assert!(output.status.success(), "final verify failed");
    assert!(stdout_str(&output).contains("2 entries"));
}
