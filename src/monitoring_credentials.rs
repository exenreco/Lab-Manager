use crate::model::Server;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use zeroize::{Zeroize, Zeroizing};

#[derive(Serialize, Deserialize)]
struct Credential {
    endpoint: String,
    password: String,
}
impl Drop for Credential {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}
fn endpoint(s: &Server) -> String {
    format!("{}|{}|{}", s.ip, s.ssh_port, s.username)
}
pub fn path(root: &Path, id: &str) -> Result<PathBuf, String> {
    if id.len() != 64 || !id.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid saved credential identifier".into());
    }
    Ok(root.join(format!("{id}.enc")))
}
pub fn save(
    root: &Path,
    server: &Server,
    password: &str,
    previous: Option<&str>,
) -> Result<String, String> {
    server.validate()?;
    if password.is_empty() || password.len() > 1024 || password.contains(['\r', '\n', '\0']) {
        return Err("Enter a password of 1–1024 bytes without line breaks.".into());
    }
    let id = if let Some(id) = previous {
        id.to_owned()
    } else {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    };
    let target = path(root, &id)?;
    let credential = Credential {
        endpoint: endpoint(server),
        password: password.into(),
    };
    let plaintext = Zeroizing::new(serde_json::to_vec(&credential).map_err(|e| e.to_string())?);
    let encrypted = crate::auth::protect(&plaintext)?;
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let temp = target.with_extension("enc.tmp");
    use std::io::Write;
    let mut file = std::fs::File::create(&temp).map_err(|e| e.to_string())?;
    file.write_all(&encrypted)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(temp, target).map_err(|e| format!("Cannot save SSH password: {e}"))?;
    Ok(id)
}
pub fn remove(root: &Path, id: &str) -> Result<(), String> {
    match std::fs::remove_file(path(root, id)?) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
fn read(target: &Path, expected: &str) -> Result<Zeroizing<String>, String> {
    if std::fs::metadata(target).map_err(|e| e.to_string())?.len() > 16384 {
        return Err("Invalid SSH credential size".into());
    }
    let bytes = std::fs::read(target).map_err(|e| e.to_string())?;
    let plain = Zeroizing::new(crate::auth::unprotect(&bytes)?);
    let credential: Credential =
        serde_json::from_slice(&plain).map_err(|_| "Invalid encrypted SSH credential")?;
    if credential.endpoint != expected {
        return Err("Saved SSH password belongs to a different endpoint".into());
    }
    Ok(Zeroizing::new(credential.password.clone()))
}
pub fn configure(
    command: &mut std::process::Command,
    root: &Path,
    id: &str,
    server: &Server,
) -> Result<(), String> {
    let target = path(root, id)?;
    let target = if target.is_absolute() {
        target
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(target)
    };
    // Verify the binding before starting SSH. No secret is placed in argv or env.
    let _password = read(&target, &endpoint(server))?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    command
        .env("SSH_ASKPASS", exe)
        .env("SSH_ASKPASS_REQUIRE", "force")
        .env("DISPLAY", "lab-manager")
        .env("LAB_MANAGER_ASKPASS", "1")
        .env("LAB_MANAGER_CREDENTIAL_FILE", target)
        .env("LAB_MANAGER_CREDENTIAL_ENDPOINT", endpoint(server));
    Ok(())
}
/// The same executable acts as a hidden OpenSSH askpass helper, before GUI startup.
pub fn askpass_mode() -> Option<i32> {
    if std::env::var("LAB_MANAGER_ASKPASS").as_deref() != Ok("1") {
        return None;
    }
    let result = (|| -> Result<(), String> {
        let prompt = std::env::args().nth(1).unwrap_or_default().to_lowercase();
        if !prompt.contains("password") || prompt.contains("passphrase") {
            return Err("Unexpected SSH prompt".into());
        }
        let file =
            std::env::var_os("LAB_MANAGER_CREDENTIAL_FILE").ok_or("Missing credential path")?;
        let expected = std::env::var("LAB_MANAGER_CREDENTIAL_ENDPOINT")
            .map_err(|_| "Missing credential binding")?;
        let password = read(Path::new(&file), &expected)?;
        use std::io::Write;
        let mut stdout = std::io::stdout().lock();
        stdout
            .write_all(password.as_bytes())
            .and_then(|_| stdout.write_all(b"\n"))
            .and_then(|_| stdout.flush())
            .map_err(|e| e.to_string())
    })();
    Some(if result.is_ok() { 0 } else { 1 })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn passwords_are_encrypted_bound_to_endpoint_and_removable() {
        let root = std::env::temp_dir().join(format!("lab-ssh-credentials-{}", std::process::id()));
        let s = Server::new("192.0.2.1".parse().unwrap());
        let id = save(&root, &s, "test-only-secret", None).unwrap();
        let bytes = std::fs::read(path(&root, &id).unwrap()).unwrap();
        assert!(!bytes.windows(16).any(|b| b == b"test-only-secret"));
        assert_eq!(
            &*read(&path(&root, &id).unwrap(), &endpoint(&s)).unwrap(),
            "test-only-secret"
        );
        assert!(read(&path(&root, &id).unwrap(), "different account endpoint").is_err());
        assert!(path(&root, "../../other").is_err());
        remove(&root, &id).unwrap();
        assert!(!path(&root, &id).unwrap().exists());
        let _ = std::fs::remove_dir(root);
    }
}
