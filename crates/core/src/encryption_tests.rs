use super::*;

fn fast_params() -> KdfParams {
    KdfParams {
        memory_kib: 8,
        iterations: 1,
        parallelism: 1,
        salt: vec![7u8; 16],
    }
}

#[test]
fn test_key_generation() {
    let key1 = EncryptionKey::generate();
    let key2 = EncryptionKey::generate();

    assert_ne!(key1.as_bytes(), key2.as_bytes());
}

#[test]
fn test_key_derivation_is_deterministic_and_salted() {
    let params = fast_params();
    let key = EncryptionKey::derive("my_password", &params).unwrap();
    let key2 = EncryptionKey::derive("my_password", &params).unwrap();
    assert_eq!(key.as_bytes(), key2.as_bytes());

    let other_pw = EncryptionKey::derive("other", &params).unwrap();
    assert_ne!(key.as_bytes(), other_pw.as_bytes());

    let mut other_salt = fast_params();
    other_salt.salt = vec![8u8; 16];
    let key3 = EncryptionKey::derive("my_password", &other_salt).unwrap();
    assert_ne!(key.as_bytes(), key3.as_bytes());
}

#[test]
fn test_short_salt_rejected() {
    let mut params = fast_params();
    params.salt = vec![1u8; 8];
    assert!(EncryptionKey::derive("pw", &params).is_err());
}

#[test]
fn test_subkeys_differ_by_context() {
    let key = EncryptionKey::generate();
    assert_ne!(key.subkey("aead"), key.subkey("chunk-id"));
    assert_ne!(key.subkey("aead"), key.as_bytes());
}

#[test]
fn test_encryption_decryption() {
    let encryptor = Encryptor::new(&EncryptionKey::generate());

    let plaintext = b"Hello, World! This is secret data.";
    let encrypted = encryptor.encrypt(plaintext, b"ctx").unwrap();

    assert_ne!(plaintext.to_vec(), encrypted);
    assert_eq!(encrypted.len(), NONCE_SIZE + plaintext.len() + 16);

    let decrypted = encryptor.decrypt(&encrypted, b"ctx").unwrap();
    assert_eq!(plaintext.to_vec(), decrypted);
}

#[test]
fn test_encryption_different_each_time() {
    let encryptor = Encryptor::new(&EncryptionKey::generate());

    let encrypted1 = encryptor.encrypt(b"test data", b"").unwrap();
    let encrypted2 = encryptor.encrypt(b"test data", b"").unwrap();
    assert_ne!(encrypted1, encrypted2);
}

#[test]
fn test_wrong_aad_rejected() {
    let encryptor = Encryptor::new(&EncryptionKey::generate());
    let encrypted = encryptor.encrypt(b"payload", b"chunk-a").unwrap();
    assert!(encryptor.decrypt(&encrypted, b"chunk-b").is_err());
}

#[test]
fn test_wrong_key_rejected() {
    let a = Encryptor::new(&EncryptionKey::generate());
    let b = Encryptor::new(&EncryptionKey::generate());
    let encrypted = a.encrypt(b"payload", b"x").unwrap();
    assert!(b.decrypt(&encrypted, b"x").is_err());
}

#[test]
fn test_decrypt_invalid_data() {
    let encryptor = Encryptor::new(&EncryptionKey::generate());

    assert!(encryptor.decrypt(b"short", b"").is_err());

    // Flip a bit in the nonce, the ciphertext, and the tag in turn
    let encrypted = encryptor.encrypt(b"test", b"").unwrap();
    for i in [0, NONCE_SIZE, encrypted.len() - 1] {
        let mut tampered = encrypted.clone();
        tampered[i] ^= 0x01;
        assert!(encryptor.decrypt(&tampered, b"").is_err(), "byte {i}");
    }

    // Truncation
    assert!(encryptor
        .decrypt(&encrypted[..encrypted.len() - 1], b"")
        .is_err());
}
