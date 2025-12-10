pub const ROOT_DIRECTIVE_TEMPLATE: &str = r#"## 🔒 Engram Protocol (MANDATORY)

This project uses Engram for agent memory. You MUST follow this workflow:

### Before Starting Work
1. Run `cat .engram/draft.md` to check for unfinished work
2. If draft exists with content, resume that task OR document why you're abandoning it

### After Completing ANY Task
1. Update `.engram/draft.md` with your work report:
   - `<summary>` tag: ONE sentence describing the change
   - Intent section: why this change was made
   - Changes section: files and functions modified
   - Verification section: how you tested it
2. Run `./engram commit` to finalize

### Rules
- NEVER modify files in `.engram/worklog/`
- NEVER leave `draft.md` empty after doing work
- If uncertain, read `.engram/AGENTS.md` for the full protocol

### ⛔ CRITICAL: Data Security
**NEVER log sensitive data** (passwords, API keys, secrets, tokens, credentials, PII) in any Engram documentation. Reference secrets by name only, never include actual values. See `.engram/AGENTS.md` for full security policy.

"#;
