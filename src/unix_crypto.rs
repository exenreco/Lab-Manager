//! Account files are AEAD encrypted; their key stays in the desktop credential vault.
use chacha20poly1305::{
    ChaCha20Poly1305, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use rand_core::{OsRng, RngCore};
use std::sync::{Mutex, OnceLock};
use zeroize::Zeroizing;

type SecretKey = Zeroizing<Vec<u8>>;
static KEY: OnceLock<Mutex<Option<SecretKey>>> = OnceLock::new();
const HEADER: &[u8] = b"LMU1";
const DOMAIN: &[u8] = b"Lab Manager local account encryption v1";

fn key(create: bool) -> Result<Zeroizing<Vec<u8>>, String> {
    let mut cached = KEY
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Credential vault lock failed")?;
    if let Some(key) = cached.as_ref() {
        return Ok(key.clone());
    }
    #[cfg(not(test))]
    let value = {
        let entry = keyring::Entry::new("com.exenreco.lab-manager", "local-encryption-key-v1")
            .map_err(|e| format!("Cannot open desktop credential vault: {e}"))?;
        match entry.get_secret() {
            Ok(value) => Zeroizing::new(value),
            Err(keyring::Error::NoEntry) if create => {
                let mut value = Zeroizing::new(vec![0u8; 32]);
                OsRng.fill_bytes(&mut value);
                entry
                    .set_secret(&value)
                    .map_err(|e| format!("Cannot save encryption key in desktop vault: {e}"))?;
                value
            }
            Err(e) => {
                return Err(format!(
                    "Desktop credential vault unavailable. Unlock Keychain or a Linux Secret Service keyring: {e}"
                ));
            }
        }
    };
    // Unit tests exercise encryption without modifying a developer's credential vault.
    #[cfg(test)]
    let value = {
        let _ = create;
        let mut value = Zeroizing::new(vec![0u8; 32]);
        OsRng.fill_bytes(&mut value);
        value
    };
    if value.len() != 32 {
        return Err("Invalid desktop encryption key. Existing accounts were not changed.".into());
    }
    *cached = Some(value.clone());
    Ok(value)
}

pub fn protect(input: &[u8]) -> Result<Vec<u8>, String> {
    let key = key(true)?;
    let cipher = ChaCha20Poly1305::new_from_slice(&key).map_err(|_| "Invalid encryption key")?;
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let encrypted = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: input,
                aad: DOMAIN,
            },
        )
        .map_err(|_| "Account encryption failed")?;
    Ok([HEADER, nonce.as_slice(), encrypted.as_slice()].concat())
}

pub fn unprotect(input: &[u8]) -> Result<Vec<u8>, String> {
    if input.len() < 32 || &input[..4] != HEADER {
        return Err("Invalid encrypted account format. Windows DPAPI accounts cannot be copied to this platform; register a local account and restore a backup.".into());
    }
    let key = key(false)?;
    let cipher = ChaCha20Poly1305::new_from_slice(&key).map_err(|_| "Invalid encryption key")?;
    cipher
        .decrypt(
            Nonce::from_slice(&input[4..16]),
            Payload {
                msg: &input[16..],
                aad: DOMAIN,
            },
        )
        .map_err(|_| {
            "Cannot decrypt account. Its credential-vault key is missing or the file was changed."
                .into()
        })
}

#[cfg(test)]
mod tests {
    #[test]
    fn encryption_roundtrips_and_rejects_tampering() {
        let plain = b"disposable local credential";
        let mut encrypted = super::protect(plain).unwrap();
        assert_eq!(super::unprotect(&encrypted).unwrap(), plain);
        assert!(!encrypted.windows(plain.len()).any(|v| v == plain));
        encrypted[20] ^= 1;
        assert!(super::unprotect(&encrypted).is_err());
        assert!(super::unprotect(b"not an encrypted account").is_err());
    }
}
