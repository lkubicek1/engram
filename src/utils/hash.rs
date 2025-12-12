use sha2::{Digest, Sha256};

/// Compute SHA256 hash of content and return as lowercase hex string
pub fn sha256_hex(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// Compute SHA256 hash and return the first 8 characters (short hash)
pub fn sha256_short(content: &str) -> String {
    sha256_hex(content)[..8].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hex() {
        let hash = sha256_hex("hello world");
        assert_eq!(hash.len(), 64);
        // Known SHA256 hash for "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_sha256_short() {
        let short = sha256_short("hello world");
        assert_eq!(short.len(), 8);
        assert_eq!(short, "b94d27b9");
    }

    #[test]
    fn test_sha256_empty() {
        let hash = sha256_hex("");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
