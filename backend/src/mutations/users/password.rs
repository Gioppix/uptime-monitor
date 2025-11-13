use anyhow::Result;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

/// Hashes a password using Argon2id
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
        .to_string();

    Ok(password_hash)
}

/// Verifies a password against a hash
pub fn verify_password(password: &str, password_hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|e| anyhow::anyhow!("Failed to parse password hash: {}", e))?;

    let argon2 = Argon2::default();

    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password() {
        let password = "test_password_123";
        hash_password(password).unwrap();
    }

    #[test]
    fn test_verify_password_success() {
        let password = "my_secret_password";
        let hash = hash_password(password).unwrap();

        let is_valid = verify_password(password, &hash).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_verify_password_failure() {
        let password = "correct_password";
        let hash = hash_password(password).unwrap();

        let is_valid = verify_password("wrong_password", &hash).unwrap();
        assert!(!is_valid);
    }

    #[test]
    fn test_different_hashes_for_same_password() {
        let password = "same_password";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        // Due to random salt, same password should produce different hashes
        assert_ne!(hash1, hash2);

        // But both should verify successfully
        assert!(verify_password(password, &hash1).unwrap());
        assert!(verify_password(password, &hash2).unwrap());
    }
}
