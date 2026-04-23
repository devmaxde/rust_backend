use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

pub fn hash_password(password: &str) -> (String, String, String, String) {
    let salt = SaltString::generate(OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password");

    let hash_string = password_hash.to_string();
    let salt_string = salt.to_string();
    let algo = "argon2id".to_string();
    let params = "default".to_string();

    (hash_string, salt_string, algo, params)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};

    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}
