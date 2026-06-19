use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use crate::kek::Kek;

#[derive(Clone)]
pub struct Dek(pub(crate) Vec<u8>);

pub struct CryptoManager {
    kek: Kek,
}

impl Clone for CryptoManager {
    fn clone(&self) -> Self {
        Self { kek: self.kek.clone() }
    }
}

impl CryptoManager {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self { kek: Kek::from_env()? })
    }

    pub fn from_hex(hex: &str) -> anyhow::Result<Self> {
        Ok(Self { kek: Kek::from_hex(hex)? })
    }

    pub fn generate_dek(&self) -> Dek {
        let key = Aes256Gcm::generate_key(OsRng);
        Dek(key.to_vec())
    }

    pub fn seal_dek(&self, dek: &Dek) -> anyhow::Result<Vec<u8>> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ct = self.kek.cipher()
            .encrypt(&nonce, dek.0.as_slice())
            .map_err(|_| anyhow::anyhow!("DEK seal failed"))?;
        let mut out = nonce.to_vec();
        out.extend(ct);
        Ok(out)
    }

    pub fn open_dek(&self, sealed: &[u8]) -> anyhow::Result<Dek> {
        anyhow::ensure!(sealed.len() > 12, "Sealed DEK too short");
        let (nonce_bytes, ct) = sealed.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = self.kek.cipher()
            .decrypt(nonce, ct)
            .map_err(|_| anyhow::anyhow!("DEK open failed, wrong key or corrupted data"))?;
        anyhow::ensure!(plaintext.len() == 32, "Decrypted DEK wrong length");
        Ok(Dek(plaintext))
    }

    fn encrypt_bytes(&self, dek: &Dek, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let key = Key::<Aes256Gcm>::from_slice(&dek.0);
        let cipher = Aes256Gcm::new(key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ct = cipher.encrypt(&nonce, data)
            .map_err(|_| anyhow::anyhow!("Encryption failed"))?;
        let mut out = nonce.to_vec();
        out.extend(ct);
        Ok(out)
    }

    fn decrypt_bytes(&self, dek: &Dek, ciphertext: &[u8]) -> anyhow::Result<Vec<u8>> {
        anyhow::ensure!(ciphertext.len() > 12, "Ciphertext too short");
        let key = Key::<Aes256Gcm>::from_slice(&dek.0);
        let cipher = Aes256Gcm::new(key);
        let (nonce_bytes, ct) = ciphertext.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        cipher.decrypt(nonce, ct)
            .map_err(|_| anyhow::anyhow!("Decryption failed, wrong key or corrupted data"))
    }

    pub fn encrypt_message(&self, dek: &Dek, content: &str) -> anyhow::Result<Vec<u8>> {
        self.encrypt_bytes(dek, content.as_bytes())
    }

    pub fn decrypt_message(&self, dek: &Dek, ciphertext: &[u8]) -> anyhow::Result<String> {
        let bytes = self.decrypt_bytes(dek, ciphertext)?;
        String::from_utf8(bytes)
            .map_err(|e| anyhow::anyhow!("Decrypted message not valid UTF-8: {}", e))
    }

    pub fn encrypt_file(&self, dek: &Dek, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.encrypt_bytes(dek, data)
    }

    pub fn decrypt_file(&self, dek: &Dek, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.decrypt_bytes(dek, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager() -> CryptoManager {
        CryptoManager::from_hex(&"ab".repeat(32)).unwrap()
    }

    #[test]
    fn dek_seal_open_roundtrip() {
        let mgr = manager();
        let dek = mgr.generate_dek();
        let sealed = mgr.seal_dek(&dek).unwrap();
        let opened = mgr.open_dek(&sealed).unwrap();
        assert_eq!(dek.0, opened.0);
    }

    #[test]
    fn message_encrypt_decrypt_roundtrip() {
        let mgr = manager();
        let dek = mgr.generate_dek();
        let plaintext = "Salut comment tu vas ?";
        let ct = mgr.encrypt_message(&dek, plaintext).unwrap();
        let result = mgr.decrypt_message(&dek, &ct).unwrap();
        assert_eq!(result, plaintext);
    }

    #[test]
    fn different_nonce_each_encrypt() {
        let mgr = manager();
        let dek = mgr.generate_dek();
        let ct1 = mgr.encrypt_message(&dek, "hello").unwrap();
        let ct2 = mgr.encrypt_message(&dek, "hello").unwrap();
        assert_ne!(ct1, ct2, "Each encryption must use a fresh nonce");
    }

    #[test]
    fn wrong_key_fails_decryption() {
        let mgr1 = manager();
        let mgr2 = CryptoManager::from_hex(&"cd".repeat(32)).unwrap();
        let dek1 = mgr1.generate_dek();
        let dek2 = mgr2.generate_dek();
        let ct = mgr1.encrypt_message(&dek1, "secret").unwrap();
        assert!(mgr1.decrypt_message(&dek2, &ct).is_err());
    }

    #[test]
    fn file_encrypt_decrypt_roundtrip() {
        let mgr = manager();
        let dek = mgr.generate_dek();
        let data = b"binary file content \x00\x01\x02";
        let ct = mgr.encrypt_file(&dek, data).unwrap();
        let result = mgr.decrypt_file(&dek, &ct).unwrap();
        assert_eq!(result, data);
    }
}
