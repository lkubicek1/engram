use clap::{Parser, Subcommand};

mod commands;
mod engram;
mod templates;
mod utils;

const LONG_ABOUT: &str = "\
Engram provides AI coding agents with persistent, tamper-evident memory across sessions.

It implements a hash-linked changelog (similar to blockchain) that allows agents to track 
their work history and resume context. Each entry cryptographically links to the previous 
state, creating a verifiable chain of work history.

The system lives in the repository (.engram/ directory) and requires minimal context 
window to understand current state - agents read only the summary index and current 
draft, not full history.";

const AFTER_HELP: &str = "\
EXAMPLES:
    Initialize Engram in a new project:
        $ engram init

    Initialize with Warp AI directive:
        $ engram init --warp

    Initialize with all AI agent directives:
        $ engram init --all

    Commit your work after updating .engram/draft.md:
        $ engram commit

    Verify the integrity of the hash chain:
        $ engram verify

    Check current Engram status:
        $ engram status

WORKFLOW:
    1. Run 'engram init' to set up Engram in your project
    2. Before starting work, check .engram/draft.md for unfinished work
    3. After completing work, update .engram/draft.md with your report
    4. Run 'engram commit' to finalize the entry
    5. Use 'engram verify' to validate history integrity at any time

SECURITY:
    ⛔ NEVER log sensitive data (passwords, API keys, secrets, tokens, 
    credentials, PII) in any Engram documentation. Reference secrets by 
    name only, never include actual values.

Learn more: https://github.com/lkubicek1/engram";

#[derive(Parser)]
#[command(name = "engram")]
#[command(version)]
#[command(about = "Persistent, tamper-evident memory for AI coding agents")]
#[command(long_about = LONG_ABOUT)]
#[command(after_help = AFTER_HELP)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize Engram in the current directory
    #[command(
        long_about = "\
Initialize Engram in the current directory.

Creates the .engram/ directory structure with:
  • .engram/AGENTS.md     - Protocol instructions for AI agents
  • .engram/draft.md      - Agent workspace (mutable, reset after commit)
  • .engram/history/      - Hash-linked entry storage
  • .engram/history/SUMMARY.md - Quick-reference index

Optionally creates root-level AI agent instruction files with the Engram 
protocol directive. Use flags to specify which files to create/update.",
        after_help = "\
EXAMPLES:
    Basic initialization:
        $ engram init

    Initialize with Warp AI support:
        $ engram init --warp

    Initialize with Junie AI support:
        $ engram init --junie

    Initialize with all AI agent directives:
        $ engram init --all"
    )]
    Init {
        /// Create/append WARP.md with Engram directive for Warp AI
        #[arg(long, help = "Create or append to WARP.md with Engram protocol directive")]
        warp: bool,
        
        /// Create/append .junie/guidelines.md with Engram directive for Junie AI
        #[arg(long, help = "Create or append to .junie/guidelines.md with Engram protocol directive")]
        junie: bool,
        
        /// Create/append AGENTS.md with Engram directive in project root
        #[arg(long, help = "Create or append to AGENTS.md with Engram protocol directive")]
        agents: bool,
        
        /// Apply all directive options (--warp, --junie, --agents)
        #[arg(long, help = "Create/append all AI agent directive files (WARP.md, .junie/guidelines.md, AGENTS.md)")]
        all: bool,
    },
    
    /// Commit the current draft to the hash-linked history
    #[command(
        long_about = "\
Commit the current draft to the hash-linked history.

Reads .engram/draft.md, extracts the summary and body content, then creates 
a new entry in .engram/history/ with:
  • The summary from the <summary> tag
  • A SHA256 hash link to the previous entry
  • An ISO-8601 timestamp
  • The full body content

The entry filename includes a sequence number and content hash (e.g., 002_e5f6a7b8.md).
After committing, the draft is reset to an empty template.

Requirements:
  • The <summary> tag must contain non-empty text
  • The draft body must have content beyond template comments",
        after_help = "\
EXAMPLES:
    After updating .engram/draft.md with your work report:
        $ engram commit

OUTPUT:
    Committed: 002_e5f6a7b8.md
    Summary: Added JWT authentication to the login endpoint
    Previous: a1b2c3d4..."
    )]
    Commit,
    
    /// Verify the integrity of the hash chain
    #[command(
        long_about = "\
Verify the integrity of the hash chain.

Checks that each entry in .engram/history/ correctly links to the previous 
entry via SHA256 hash. Also verifies that each filename's hash matches the 
file's actual content hash.

This detects:
  • Tampered or modified historical entries
  • Corrupted files
  • Missing entries in the chain
  • Incorrect hash values",
        after_help = "\
EXAMPLES:
    Verify the chain integrity:
        $ engram verify

OUTPUT (success):
    ✓ Chain verified: 47 entries
      First: 001_a1b2c3d4.md (2025-01-15)
      Latest: 047_f9e8d7c6.md (2025-06-12)

OUTPUT (failure):
    ✗ Chain broken at entry 023_bad12345.md
    Expected Previous: 8a7b6c5d4e3f2a1b...
    Found Previous:    0000000000000000..."
    )]
    Verify,
    
    /// Display current Engram state and status
    #[command(
        long_about = "\
Display current Engram state and status.

Shows a summary of the current Engram state including:
  • Number of entries in history
  • Latest entry filename, date, and summary
  • Draft status (empty or has uncommitted content)
  • Chain verification status",
        after_help = "\
EXAMPLES:
    Check current status:
        $ engram status

OUTPUT:
    Engram Status
    ─────────────
    History: 47 entries
    Latest:  047_f9e8d7c6.md (2025-06-12T14:32:07Z)
             \"Fixed token expiration validation bug\"

    Draft:   Has content (uncommitted work)
             Summary: \"Added rate limiting to API endpoints\"

    Chain:   ✓ Verified"
    )]
    Status,
}

fn main() {
    let cli = Cli::parse();
    
    let result = match cli.command {
        Commands::Init { warp, junie, agents, all } => {
            let options = commands::init::InitOptions {
                warp: warp || all,
                junie: junie || all,
                agents: agents || all,
                all,
            };
            commands::init::run(options)
        }
        Commands::Commit => {
            commands::commit::run()
        }
        Commands::Verify => {
            commands::verify::run()
        }
        Commands::Status => {
            commands::status::run()
        }
    };
    
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
