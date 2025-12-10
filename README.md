# Engram

**Persistent, tamper-evident memory for AI coding agents.**

Engram is a CLI tool that gives AI coding agents the ability to maintain context across sessions. It implements a hash-linked changelog (similar to blockchain) that allows agents to track their worklog, understand what was done previously, and resume work seamlessly.

> ⚠️ **Development Status:** This project is in early development (Phase 1). The CLI framework is in place, but command implementations are not yet complete. See [Development Status](#development-status) for details.

## The Problem

AI coding agents lack memory between sessions. When an agent returns to a project, it has no record of:

- What work was previously completed
- What decisions were made and why  
- What the current state of in-progress work is

This leads to repeated explanations, inconsistent approaches, and lost context.

## The Solution

Engram provides a lightweight, hash-linked changelog system that:

- **Lives in your repository** — All worklog stored in `.engram/` directory
- **Requires minimal context** — Agents read only the summary index and current draft
- **Provides cryptographic verification** — Hash chain ensures worklog integrity
- **Resets workspace after commits** — Clean slate for each unit of work

## How It Works

```
project-root/
├── .engram/
│   ├── AGENTS.md           # Protocol instructions for AI agents
│   ├── draft.md            # Agent workspace (reset after commit)
│   └── worklog/
│       ├── SUMMARY.md      # Index: filename | one-sentence summary
│       ├── 001_a1b2c3d4.md # First entry (Previous: none)
│       ├── 002_e5f6a7b8.md # Second entry (Previous: hash of 001)
│       └── ...
```

Each worklog entry contains:
- A one-sentence summary
- The SHA256 hash of the previous entry (creating a tamper-evident chain)
- A timestamp
- Detailed documentation of intent, changes, and verification

## Installation

### From Source

Requires [Rust](https://rustup.rs/) 1.70 or later.

```bash
git clone https://github.com/lkubicek1/engram.git
cd engram
cargo build --release
```

The binary will be at `target/release/engram`.

## Usage

### Initialize a Project

```bash
engram init
```

This creates the `.engram/` directory structure with:
- `AGENTS.md` — Instructions for AI agents
- `draft.md` — Workspace template
- `worklog/SUMMARY.md` — Empty worklog index

Options:
- `--warp` — Also create/append to `WARP.md` 
- `--junie` — Also create/append to `.junie/guidelines.md`
- `--agents` — Also create/append to root `AGENTS.md`
- `--all` — All of the above

### Commit Work

After completing work, agents fill in `.engram/draft.md`:

```markdown
<summary>Added JWT authentication to the login endpoint</summary>

## Intent
The API needed stateless session management.

## Changes
- Created `src/auth/jwt.rs`
- Modified `src/routes/login.rs`

## Verification
- `cargo test auth::` - 12 tests passing
```

Then commit:

```bash
engram commit
```

This:
1. Validates the draft has a summary and content
2. Creates a new worklog entry with hash linking
3. Updates `SUMMARY.md`
4. Resets `draft.md` for the next unit of work

### Verify Worklog

```bash
engram verify
```

Validates the entire hash chain to detect any tampering or corruption.

### Check Status

```bash
engram status
```

Shows current state: worklog count, latest entry, draft status, and chain verification.

## For AI Agents

When working on an Engram-enabled project:

1. **Before starting:** Read `.engram/draft.md` to check for unfinished work
2. **During work:** Keep track of changes for the report
3. **After completing:** Fill in `draft.md` and run `./engram commit`

**Rules:**
- Never modify files in `.engram/worklog/` directly
- Never leave `draft.md` empty after doing work
- Always use `engram commit` to finalize (handles hashing automatically)

**⛔ CRITICAL — Data Security:**
- **NEVER** log passwords, API keys, secrets, tokens, credentials, or PII in any Engram documentation
- Reference secrets by name only (e.g., "Updated `DATABASE_PASSWORD`"), never include actual values
- See `.engram/AGENTS.md` for the full security policy

## Development Status

Engram is being developed in phases:

### Phase 1: Core CLI (In Progress)
- [x] Project setup with dependencies
- [x] CLI framework with clap
- [x] Core data structures (draft, worklog, summary parsing)
- [x] `init` command implementation
- [x] `commit` command implementation  
- [x] `verify` command implementation
- [x] Unit tests for commands

### Phase 2: Polish
- [x] `status` command
- [x] `init` append logic for WARP.md, AGENTS.md, .junie/guidelines.md
- [ ] Idempotency checks
- [ ] Error messages and user feedback
- [ ] Integration tests

### Phase 3: Distribution
- [ ] GitHub Actions workflow
- [ ] Wrapper scripts (engram, engram.cmd)
- [ ] Dogfood: initialize Engram in the Engram repo
- [ ] Tag v0.1.0 release

## Design Principles

1. **Traceability > Convenience** — Every entry cryptographically links to the previous state
2. **Low Context Overhead** — Agents read only summary index and current draft, not full history
3. **Environment Agnostic** — Pure Rust binary with zero runtime dependencies
4. **Self-Bootstrapping** — Wrapper script downloads binary on first use (planned)

## ⛔ Security: Sensitive Data Policy

**Agents must NEVER log, record, or include sensitive data in ANY Engram documentation.**

Engram creates a permanent, tamper-evident history that is typically committed to version control. Any sensitive data recorded becomes a **permanent security vulnerability** that cannot be easily removed without breaking the hash chain.

**Prohibited data includes:**
- Passwords, passphrases, or authentication credentials
- API keys, tokens, or secrets
- Private keys, certificates, or encryption keys
- Database connection strings with credentials
- Environment variables containing secrets
- Personal identifying information (PII)

**Safe practices:**
- Reference secrets by name only: "Updated the `DATABASE_PASSWORD` environment variable"
- Describe changes generically: "Rotated API credentials for payment service"
- Use placeholders in examples: `API_KEY=<redacted>`

This policy is enforced through agent instructions in `.engram/AGENTS.md`.

## Technical Details

- **Language:** Rust 2021 edition
- **Dependencies:** clap (CLI), sha2 (hashing), chrono (timestamps), regex (parsing)
- **Hash Algorithm:** SHA256
- **Entry Format:** `NNN_HHHHHHHH.md` (sequence number + first 8 chars of content hash)

## License

MIT License — see [LICENSE](LICENSE) for details.
