use std::fs;
use std::io;
use std::path::Path;

/// Check if the .engram directory exists in the current directory
pub fn engram_exists() -> bool {
    Path::new(".engram").is_dir()
}

/// Check if the .engram/worklog directory exists
pub fn worklog_exists() -> bool {
    Path::new(".engram/worklog").is_dir()
}

/// Get the path to the draft file
pub fn draft_path() -> &'static Path {
    Path::new(".engram/draft.md")
}

/// Get the path to the worklog directory
pub fn worklog_path() -> &'static Path {
    Path::new(".engram/worklog")
}

/// Get the path to the SUMMARY.md file
pub fn summary_path() -> &'static Path {
    Path::new(".engram/worklog/SUMMARY.md")
}

/// Get the path to the AGENTS.md file
pub fn agents_path() -> &'static Path {
    Path::new(".engram/AGENTS.md")
}

/// List all worklog entry files sorted by sequence number
pub fn list_worklog_entries() -> io::Result<Vec<String>> {
    let worklog_dir = worklog_path();
    
    if !worklog_dir.is_dir() {
        return Ok(Vec::new());
    }
    
    let mut entries: Vec<String> = fs::read_dir(worklog_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|name| {
            // Match pattern NNN_*.md but not SUMMARY.md
            let re = regex::Regex::new(r"^\d{3}_[a-f0-9]{8}\.md$").unwrap();
            re.is_match(name)
        })
        .collect();
    
    // Sort by sequence number (first 3 digits)
    entries.sort();
    
    Ok(entries)
}

/// Get the next sequence number for a new worklog entry
pub fn next_sequence_number() -> io::Result<u32> {
    let entries = list_worklog_entries()?;
    
    if entries.is_empty() {
        return Ok(1);
    }
    
    // Parse highest sequence number from last entry
    let last = entries.last().unwrap();
    let seq: u32 = last[..3].parse().unwrap_or(0);
    
    Ok(seq + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths() {
        assert_eq!(draft_path().to_str().unwrap(), ".engram/draft.md");
        assert_eq!(worklog_path().to_str().unwrap(), ".engram/worklog");
        assert_eq!(summary_path().to_str().unwrap(), ".engram/worklog/SUMMARY.md");
        assert_eq!(agents_path().to_str().unwrap(), ".engram/AGENTS.md");
    }
}
