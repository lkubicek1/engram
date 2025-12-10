pub const AGENTS_TEMPLATE: &str = r#"# Engram Protocol: Agent Instructions

This project uses Engram for persistent agent memory. Follow this protocol for all work sessions.

## Before Starting Work

1. Read `.engram/draft.md` to check for unfinished work
2. If draft contains work-in-progress, either:
   - Resume and complete that work, OR
   - Document why you are abandoning it and commit
3. Read `.engram/worklog/SUMMARY.md` to understand recent project worklog

## During Work

1. Work on your assigned task normally
2. Keep mental note of all changes for the report

## After Completing Work

1. Update `.engram/draft.md` with your work report:
   - Fill in the `<summary>` tag with ONE sentence describing the change
   - Document Intent: why the change was made
   - Document Changes: specific files and functions modified
   - Document Verification: how you tested/validated

2. Run `./engram commit` to finalize the entry

## Rules

- **NEVER** modify files in `.engram/worklog/` directly
- **NEVER** leave `draft.md` empty after doing work
- **NEVER** manually calculate or enter hashes
- **ALWAYS** run `./engram commit` to finalize work (the tool handles hashing)

## ⛔ CRITICAL: Data Security

**NEVER log, record, or include sensitive data in ANY Engram documentation.**

This includes but is not limited to:
- Passwords, passphrases, or authentication credentials
- API keys, tokens, or secrets
- Private keys, certificates, or encryption keys
- Database connection strings with credentials
- Environment variables containing secrets
- Personal identifying information (PII)
- Any data marked as confidential or sensitive

**This applies to:**
- The `<summary>` tag
- Intent, Changes, and Verification sections in `draft.md`
- Any content that will be committed to worklog

**Instead:**
- Reference secrets by name only (e.g., "Updated the DATABASE_PASSWORD environment variable")
- Describe changes generically (e.g., "Rotated API credentials for payment service")
- Use placeholders in examples (e.g., `API_KEY=<redacted>`)

**Violation of this policy creates permanent security vulnerabilities in the repository worklog.**

## Verifying Worklog Integrity

Run `./engram verify` to validate the hash chain integrity at any time.

## Understanding the Hash Chain

Each worklog entry contains the SHA256 hash of the previous entry's content. This creates a tamper-evident chain:
- If any historical entry is modified, its hash changes
- This breaks the link from the next entry
- `./engram verify` detects this immediately

The hash in the filename is the hash of that file's own content, providing a quick integrity check.
"#;
