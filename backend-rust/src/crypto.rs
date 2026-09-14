use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};

pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> anyhow::Result<(String, String)> {
    let cipher = Aes256Gcm::new_from_slice(key)?;
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let encrypted = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|_| anyhow::anyhow!("encryption failed"))?;
    Ok((STANDARD.encode(encrypted), STANDARD.encode(nonce)))
}
pub fn decrypt(key: &[u8; 32], ciphertext: &str, nonce: &str) -> anyhow::Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)?;
    let nonce = STANDARD.decode(nonce)?;
    let ciphertext = STANDARD.decode(ciphertext)?;
    cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| anyhow::anyhow!("decryption failed"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn credential_round_trip() {
        let k = [7u8; 32];
        let (c, n) = super::encrypt(&k, b"secret").unwrap();
        assert_eq!(super::decrypt(&k, &c, &n).unwrap(), b"secret");
        assert!(!c.contains("secret"));
    }
}
