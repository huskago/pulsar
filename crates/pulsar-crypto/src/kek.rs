use aes_gcm::{aead::KeyInit, Aes256Gcm, Key};
use std::fmt;

#[derive(Clone)]
pub struct Kek {
    cipher: Aes256Gcm,
}

impl fmt::Debug for Kek {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Kek").finish_non_exhaustive()
    }
}

impl Kek {
    pub fn from_hex(hex_str: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| anyhow::anyhow!("KEK hex decode failed: {}", e))?;
        anyhow::ensure!(
            bytes.len() == 32,
            "KEK must be exactly 32 bytes (64 hex chars), got {}",
            bytes.len()
        );
        let key = Key::<Aes256Gcm>::from_slice(&bytes);
        Ok(Self { cipher: Aes256Gcm::new(key) })
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let hex = std::env::var("KEK_SECRET")
            .map_err(|_| anyhow::anyhow!("KEK_SECRET environment variable is not set"))?;
        Self::from_hex(&hex)
    }

    pub(crate) fn cipher(&self) -> &Aes256Gcm {
        &self.cipher
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_key() {
        let result = Kek::from_hex("deadbeef");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("32 bytes"));
    }

    #[test]
    fn rejects_invalid_hex() {
        let result = Kek::from_hex("zzzz");
        assert!(result.is_err());
    }

    #[test]
    fn accepts_valid_32_byte_key() {
        let hex = "0".repeat(64);
        assert!(Kek::from_hex(&hex).is_ok());
    }
}
