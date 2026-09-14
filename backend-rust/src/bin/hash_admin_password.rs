use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::io::{self, Write};

fn main() -> anyhow::Result<()> {
    eprint!("Admin password: ");
    io::stderr().flush()?;
    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    let password = password.trim_end_matches(['\r', '\n']);
    anyhow::ensure!(
        password.len() >= 12,
        "password must be at least 12 characters"
    );
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| anyhow::anyhow!("failed to hash password: {error}"))?;
    println!("{}", STANDARD.encode(hash.to_string()));
    Ok(())
}
