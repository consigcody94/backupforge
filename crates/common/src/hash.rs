use blake3::Hasher;

/// Hash data using BLAKE3 (fast, secure hashing)
pub fn hash_data(data: &[u8]) -> Vec<u8> {
    Hasher::new().update(data).finalize().as_bytes().to_vec()
}

/// Hash data and return as hex string
pub fn hash_data_hex(data: &[u8]) -> String {
    hex::encode(hash_data(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_consistency() {
        let data = b"test data";
        let hash1 = hash_data(data);
        let hash2 = hash_data(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_different_data() {
        let hash1 = hash_data(b"data1");
        let hash2 = hash_data(b"data2");
        assert_ne!(hash1, hash2);
    }
}
