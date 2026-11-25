use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

pub const QUESTIONS: [&str; 6] = [
    "What was the name of your first school?",
    "What was your childhood nickname?",
    "What is the name of a memorable teacher?",
    "What was the name of your first pet?",
    "What city did your parents meet in?",
    "What is a memorable place from your childhood?",
];

#[derive(Clone, Default, Serialize, Deserialize)]
struct Attempts {
    failures: u32,
    until: u64,
}
impl Attempts {
    fn allowed(&self) -> Result<(), String> {
        if self.until > now() {
            Err(format!(
                "Too many attempts. Try again in {} seconds.",
                self.until - now()
            ))
        } else {
            Ok(())
        }
    }
    fn fail(&mut self) {
        if self.until != 0 && self.until <= now() {
            self.failures = 0;
            self.until = 0;
        }
        self.failures += 1;
        if self.failures >= 5 {
            self.until = now() + 60;
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Account {
    version: u32,
    #[serde(default)]
    profile: crate::profile::Profile,
    #[serde(default)]
    legacy_inventory_owner: Option<bool>,
    username: String,
    password_hash: String,
    questions: [String; 2],
    answer_hashes: [String; 2],
    login: Attempts,
    recovery: Attempts,
    #[serde(default)]
    remembered_hash: Option<String>,
    #[serde(default)]
    remembered_until: u64,
}

#[derive(Serialize, Deserialize)]
struct RememberedSession {
    #[serde(default)]
    username: Option<String>,
    token: String,
}
impl Drop for RememberedSession {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.token.zeroize();
    }
}

pub struct AccountStore {
    pub path: PathBuf,
}
impl AccountStore {
    fn load_all(&self) -> Result<Vec<Account>, String> {
        let bytes = match std::fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(format!("Cannot read local accounts: {e}")),
        };
        let plaintext = Zeroizing::new(unprotect(&bytes)?);
        // The earlier single-owner object remains readable and migrates on save.
        let accounts: Vec<Account> = serde_json::from_slice(&plaintext)
            .or_else(|_| serde_json::from_slice::<Account>(&plaintext).map(|a| vec![a]))
            .map_err(|_| "The encrypted account record is invalid.")?;
        if accounts.iter().any(|a| a.version != 1) {
            return Err("Unsupported account format.".into());
        }
        Ok(accounts)
    }
    fn load(&self) -> Result<Option<Account>, String> {
        Ok(self.load_all()?.into_iter().next())
    }
    fn load_user(&self, username: &str) -> Result<Option<Account>, String> {
        Ok(self
            .load_all()?
            .into_iter()
            .find(|a| a.username.eq_ignore_ascii_case(username.trim())))
    }
    fn save(&self, account: &Account) -> Result<(), String> {
        let mut accounts = self.load_all()?;
        if let Some(existing) = accounts
            .iter_mut()
            .find(|a| a.username.eq_ignore_ascii_case(&account.username))
        {
            *existing = account.clone();
        } else {
            accounts.push(account.clone());
        }
        self.save_all(&accounts)
    }
    fn save_all(&self, accounts: &[Account]) -> Result<(), String> {
        let plaintext = Zeroizing::new(serde_json::to_vec(accounts).map_err(|e| e.to_string())?);
        let encrypted = protect(&plaintext)?;
        std::fs::create_dir_all(self.path.parent().ok_or("Missing account directory")?)
            .map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("enc.tmp");
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        file.write_all(&encrypted)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(tmp, &self.path).map_err(|e| format!("Cannot save encrypted account: {e}"))
    }
    pub fn username(&self) -> Result<Option<String>, String> {
        Ok(self.load()?.map(|a| a.username))
    }
    pub fn questions(&self, username: &str) -> Result<[String; 2], String> {
        Ok(self
            .load_user(username)?
            .ok_or("Enter the username of the account to recover.")?
            .questions)
    }
    pub fn owns_legacy_inventory(&self, username: &str) -> Result<bool, String> {
        Ok(self.load_all()?.iter().enumerate().any(|(index, account)| {
            account.username.eq_ignore_ascii_case(username)
                && account.legacy_inventory_owner.unwrap_or(index == 0)
        }))
    }

    pub fn register(
        &self,
        username: &str,
        password: &str,
        questions: [usize; 2],
        answers: [&str; 2],
    ) -> Result<(), String> {
        self.register_with_profile(
            username,
            password,
            questions,
            answers,
            crate::profile::Profile::default(),
        )
    }

    pub fn profile(&self, username: &str) -> Result<crate::profile::Profile, String> {
        let profile = self
            .load_user(username)?
            .ok_or("Account not found.")?
            .profile;
        profile.validate()?;
        Ok(profile)
    }

    pub fn register_with_profile(
        &self,
        username: &str,
        password: &str,
        questions: [usize; 2],
        answers: [&str; 2],
        profile: crate::profile::Profile,
    ) -> Result<(), String> {
        profile.validate()?;
        if self.load_user(username)?.is_some() {
            return Err("This username is already registered. Sign in or recover it.".into());
        }
        let username = username.trim();
        if username.len() < 3
            || username.len() > 64
            || !username
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err(
                "Use a username of 3–64 letters, numbers, dots, underscores or hyphens.".into(),
            );
        }
        validate_password(password)?;
        if questions[0] == questions[1] || questions.iter().any(|i| *i >= QUESTIONS.len()) {
            return Err("Choose two different security questions.".into());
        }
        let answers = [normalize(answers[0]), normalize(answers[1])];
        if answers
            .iter()
            .any(|a| a.chars().count() < 3 || a.len() > 256)
        {
            return Err("Each security answer must be 3–256 characters.".into());
        }
        if answers[0] == answers[1] {
            return Err("Use different answers for the two questions.".into());
        }
        let account = Account {
            version: 1,
            profile,
            legacy_inventory_owner: Some(!self.path.exists()),
            username: username.into(),
            password_hash: hash(password)?,
            questions: questions.map(|i| QUESTIONS[i].into()),
            answer_hashes: [hash(&answers[0])?, hash(&answers[1])?],
            login: Attempts::default(),
            recovery: Attempts::default(),
            remembered_hash: None,
            remembered_until: 0,
        };
        self.save(&account)
    }

    pub fn sign_in(&self, username: &str, password: &str) -> Result<String, String> {
        self.sign_in_with_options(username, password, false)
    }

    pub fn sign_in_with_options(
        &self,
        username: &str,
        password: &str,
        remember: bool,
    ) -> Result<String, String> {
        let mut account = self
            .load_user(username)?
            .ok_or("Incorrect username or password.")?;
        account.login.allowed()?;
        let password_ok = verify(password, &account.password_hash);
        if !password_ok || !account.username.eq_ignore_ascii_case(username.trim()) {
            account.login.fail();
            self.save(&account)?;
            return Err("Incorrect username or password.".into());
        }
        account.login = Attempts::default();
        // Only the account most recently signed in may be remembered on this installation.
        let mut accounts = self.load_all()?;
        for other in &mut accounts {
            other.remembered_hash = None;
            other.remembered_until = 0;
        }
        self.save_all(&accounts)?;
        account.remembered_hash = None;
        account.remembered_until = 0;
        if remember {
            let mut bytes = Zeroizing::new([0_u8; 32]);
            OsRng
                .try_fill_bytes(bytes.as_mut())
                .map_err(|_| "Could not generate a secure sign-in token.")?;
            let session = RememberedSession {
                username: Some(account.username.clone()),
                token: bytes.iter().map(|b| format!("{b:02x}")).collect(),
            };
            let plaintext =
                Zeroizing::new(serde_json::to_vec(&session).map_err(|e| e.to_string())?);
            let encrypted = protect(&plaintext)?;
            account.remembered_hash = Some(hash(&session.token)?);
            account.remembered_until = now() + 30 * 24 * 60 * 60;
            let path = self.path.with_file_name("session.enc");
            let temp = path.with_extension("enc.tmp");
            use std::io::Write;
            let mut file = std::fs::File::create(&temp).map_err(|e| e.to_string())?;
            file.write_all(&encrypted)
                .and_then(|_| file.sync_all())
                .map_err(|e| e.to_string())?;
            drop(file);
            std::fs::rename(temp, path).map_err(|e| e.to_string())?;
        }
        self.save(&account)?;
        Ok(account.username)
    }

    pub fn automatic_sign_in(&self) -> Result<Option<String>, String> {
        let encrypted = match std::fs::read(self.path.with_file_name("session.enc")) {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let plaintext = Zeroizing::new(unprotect(&encrypted)?);
        let session: RememberedSession =
            serde_json::from_slice(&plaintext).map_err(|_| "Invalid automatic sign-in token.")?;
        let account = match &session.username {
            Some(username) => self.load_user(username)?,
            None => self.load()?,
        };
        let Some(account) = account else {
            return Ok(None);
        };
        let Some(hash) = account.remembered_hash else {
            return Ok(None);
        };
        if account.remembered_until <= now() || account.login.until > now() {
            return Ok(None);
        }
        Ok(verify(&session.token, &hash).then_some(account.username))
    }

    pub fn sign_out(&self) -> Result<(), String> {
        let mut accounts = self.load_all()?;
        for account in &mut accounts {
            account.remembered_hash = None;
            account.remembered_until = 0;
        }
        if !accounts.is_empty() {
            self.save_all(&accounts)?;
        }
        // Revocation is enforced by the account record, even if cleanup is denied.
        let _ = std::fs::remove_file(self.path.with_file_name("session.enc"));
        Ok(())
    }

    /// Remove a local sign-in and archive its inventory; never reuse old device data.
    pub fn remove_account(&self, username: &str) -> Result<(), String> {
        let mut accounts = self.load_all()?;
        let index = accounts
            .iter()
            .position(|account| account.username.eq_ignore_ascii_case(username))
            .ok_or("Account not found.")?;
        for (index, account) in accounts.iter_mut().enumerate() {
            if account.legacy_inventory_owner.is_none() {
                account.legacy_inventory_owner = Some(index == 0);
            }
            account.remembered_hash = None;
            account.remembered_until = 0;
        }
        let root = self.path.parent().ok_or("Missing account directory")?;
        let store = crate::model::ServerStore::for_user(root, &accounts[index].username)?;
        let directory = store.path.parent().ok_or("Missing inventory directory")?;
        let archive = if directory.exists() {
            let archive_root = root.join("removed-accounts");
            std::fs::create_dir_all(&archive_root)
                .map_err(|error| format!("Cannot archive device data: {error}"))?;
            let mut random = [0u8; 12];
            OsRng.fill_bytes(&mut random);
            let name = random
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            let target = archive_root.join(name);
            std::fs::rename(directory, &target)
                .map_err(|error| format!("Cannot archive device data: {error}"))?;
            Some(target)
        } else {
            None
        };
        accounts.remove(index);
        if let Err(error) = self.save_all(&accounts) {
            if let Some(archive) = archive
                && let Err(restore) = std::fs::rename(&archive, directory)
            {
                return Err(format!(
                    "{error}. Device data remains in {} because restoring it failed: {restore}",
                    archive.display()
                ));
            }
            return Err(error);
        }
        let _ = std::fs::remove_file(self.path.with_file_name("session.enc"));
        Ok(())
    }

    pub fn reset(&self, username: &str, answers: [&str; 2], password: &str) -> Result<(), String> {
        let mut account = self
            .load_user(username)?
            .ok_or("Username or security answers did not match.")?;
        account.recovery.allowed()?;
        validate_password(password)?;
        let a = verify(&normalize(answers[0]), &account.answer_hashes[0]);
        let b = verify(&normalize(answers[1]), &account.answer_hashes[1]);
        if !a || !b || !account.username.eq_ignore_ascii_case(username.trim()) {
            account.recovery.fail();
            self.save(&account)?;
            return Err("Username or security answers did not match.".into());
        }
        account.password_hash = hash(password)?;
        account.login = Attempts::default();
        account.recovery = Attempts::default();
        account.remembered_hash = None;
        account.remembered_until = 0;
        self.save(&account)
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn normalize(value: &str) -> Zeroizing<String> {
    Zeroizing::new(
        value
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase(),
    )
}
fn validate_password(value: &str) -> Result<(), String> {
    if value.chars().count() < 12 || value.len() > 1024 {
        Err("Use a password of at least 12 characters (maximum 1024 bytes).".into())
    } else {
        Ok(())
    }
}
fn hash(value: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(value.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| "Could not protect the account credentials.".into())
}
fn verify(value: &str, encoded: &str) -> bool {
    PasswordHash::new(encoded).is_ok_and(|hash| {
        Argon2::default()
            .verify_password(value.as_bytes(), &hash)
            .is_ok()
    })
}

#[cfg(windows)]
fn crypt(input: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
        },
    };
    let input_len = u32::try_from(input.len()).map_err(|_| "Account record is too large")?;
    let source = CRYPT_INTEGER_BLOB {
        cbData: input_len,
        pbData: input.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // SAFETY: input remains alive for the call; DPAPI owns the output buffer.
    // UI is forbidden and no machine-wide flag is used, binding to the current user.
    let success = unsafe {
        if encrypt {
            CryptProtectData(
                &source,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &source,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    if success == 0 {
        return Err(format!(
            "Windows could not {} the account for this user: {}",
            if encrypt { "encrypt" } else { "decrypt" },
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: DPAPI returned cbData bytes, released by LocalFree after copying.
    let result = unsafe {
        let buffer = std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize);
        let result = buffer.to_vec();
        use zeroize::Zeroize;
        buffer.zeroize();
        LocalFree(output.pbData.cast());
        result
    };
    Ok(result)
}
pub(crate) fn protect(input: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(windows)]
    {
        crypt(input, true)
    }
    #[cfg(not(windows))]
    {
        let _ = input;
        Err("Encrypted local accounts require Windows DPAPI.".into())
    }
}
pub(crate) fn unprotect(input: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(windows)]
    {
        crypt(input, false)
    }
    #[cfg(not(windows))]
    {
        let _ = input;
        Err("Encrypted local accounts require Windows DPAPI.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(windows)]
    fn account_removal_rolls_back_on_save_failure_and_does_not_transfer_inventory() {
        use crate::model::{Server, ServerStore};
        let directory =
            std::env::temp_dir().join(format!("lab-remove-account-{}", std::process::id()));
        let store = AccountStore {
            path: directory.join("account.enc"),
        };
        store
            .register(
                "alice",
                "alice secure password",
                [0, 1],
                ["school name", "nickname"],
            )
            .unwrap();
        store
            .register(
                "bob",
                "bob secure password",
                [2, 3],
                ["teacher name", "first pet"],
            )
            .unwrap();
        let alice = ServerStore::for_user(&directory, "alice").unwrap();
        let bob = ServerStore::for_user(&directory, "bob").unwrap();
        let alice_devices = vec![Server::new("192.168.1.1".parse().unwrap())];
        let bob_devices = vec![Server::new("192.168.1.2".parse().unwrap())];
        alice.save(&alice_devices).unwrap();
        bob.save(&bob_devices).unwrap();
        std::fs::write(
            directory.join("servers.json"),
            serde_json::to_vec(&alice_devices).unwrap(),
        )
        .unwrap();
        store
            .sign_in_with_options("alice", "alice secure password", true)
            .unwrap();
        let token = std::fs::read(directory.join("session.enc")).unwrap();
        let blocked_temp = store.path.with_extension("enc.tmp");
        std::fs::create_dir(&blocked_temp).unwrap();
        assert!(store.remove_account("alice").is_err());
        assert_eq!(alice.load().unwrap(), alice_devices);
        assert!(store.load_user("alice").unwrap().is_some());
        std::fs::remove_dir(blocked_temp).unwrap();
        store.remove_account("alice").unwrap();
        assert!(store.load_user("alice").unwrap().is_none());
        assert_eq!(bob.load().unwrap(), bob_devices);
        assert!(!alice.path.exists());
        assert!(!store.owns_legacy_inventory("bob").unwrap());
        std::fs::write(directory.join("session.enc"), token).unwrap();
        assert!(store.automatic_sign_in().unwrap().is_none());
        assert!(store.sign_in("alice", "alice secure password").is_err());
        assert!(store.sign_in("bob", "bob secure password").is_ok());
        store
            .register(
                "alice",
                "another secure password",
                [0, 1],
                ["new school", "new nickname"],
            )
            .unwrap();
        assert!(alice.load().unwrap().is_empty());
        assert!(!store.owns_legacy_inventory("alice").unwrap());
        store.remove_account("alice").unwrap();
        store.remove_account("bob").unwrap();
        store
            .register(
                "charlie",
                "charlie secure password",
                [0, 1],
                ["school name", "nickname"],
            )
            .unwrap();
        assert!(!store.owns_legacy_inventory("charlie").unwrap());
        let archive_root = directory.join("removed-accounts");
        let archives: Vec<_> = std::fs::read_dir(&archive_root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(archives.len(), 2);
        for archive in archives {
            std::fs::remove_file(archive.join("servers.json")).unwrap();
            std::fs::remove_dir(archive).unwrap();
        }
        std::fs::remove_dir(archive_root).unwrap();
        std::fs::remove_dir(directory.join("users")).unwrap();
        std::fs::remove_file(directory.join("servers.json")).unwrap();
        std::fs::remove_file(&store.path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
    #[test]
    #[cfg(windows)]
    fn profiles_are_encrypted_isolated_and_default_for_legacy_accounts() {
        let directory =
            std::env::temp_dir().join(format!("lab-profile-auth-{}", std::process::id()));
        let store = AccountStore {
            path: directory.join("account.enc"),
        };
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            8,
            8,
            image::Rgba([50, 120, 180, 255]),
        ))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
        let photo = crate::profile::Profile {
            avatar: 3,
            photo: Some(png.into_inner()),
        };
        store
            .register_with_profile(
                "alice",
                "alice secure password",
                [0, 1],
                ["first school", "nickname"],
                photo.clone(),
            )
            .unwrap();
        store
            .register(
                "bob",
                "bob secure password",
                [2, 3],
                ["teacher name", "first pet"],
            )
            .unwrap();
        let reopened = AccountStore {
            path: store.path.clone(),
        };
        assert_eq!(reopened.profile("alice").unwrap(), photo);
        assert_eq!(
            reopened.profile("bob").unwrap(),
            crate::profile::Profile::default()
        );
        let encrypted = std::fs::read(&store.path).unwrap();
        assert!(!String::from_utf8_lossy(&encrypted).contains("alice"));
        assert!(
            !encrypted
                .windows(8)
                .any(|bytes| bytes == b"\x89PNG\r\n\x1a\n")
        );
        let mut accounts = store.load_all().unwrap();
        accounts[1].profile.avatar = 4;
        let mut legacy = serde_json::to_value(&accounts).unwrap();
        legacy[1].as_object_mut().unwrap().remove("profile");
        std::fs::write(
            &store.path,
            protect(&serde_json::to_vec(&legacy).unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            reopened.profile("bob").unwrap(),
            crate::profile::Profile::default()
        );
        assert_eq!(reopened.profile("alice").unwrap(), photo);
        std::fs::remove_file(&store.path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
    #[test]
    #[cfg(windows)]
    fn accounts_keep_passwords_and_recovery_separate() {
        let directory = std::env::temp_dir().join(format!("lab-multi-auth-{}", std::process::id()));
        let store = AccountStore {
            path: directory.join("account.enc"),
        };
        store
            .register(
                "alice",
                "alice secure password",
                [0, 1],
                ["alice school", "alice nickname"],
            )
            .unwrap();
        store
            .register(
                "bob",
                "bob secure password",
                [2, 3],
                ["bob teacher", "bob pet"],
            )
            .unwrap();
        assert!(
            store
                .register(
                    "ALICE",
                    "another secure password",
                    [0, 1],
                    ["school", "nickname"]
                )
                .is_err()
        );
        assert!(store.owns_legacy_inventory("alice").unwrap());
        assert!(!store.owns_legacy_inventory("bob").unwrap());
        assert!(store.sign_in("bob", "alice secure password").is_err());
        assert!(
            store
                .reset(
                    "bob",
                    ["alice school", "alice nickname"],
                    "a changed password"
                )
                .is_err()
        );
        assert_eq!(
            store.questions("bob").unwrap(),
            [QUESTIONS[2].to_owned(), QUESTIONS[3].to_owned()]
        );
        store
            .sign_in_with_options("alice", "alice secure password", true)
            .unwrap();
        assert_eq!(store.automatic_sign_in().unwrap().as_deref(), Some("alice"));
        store.sign_in("bob", "bob secure password").unwrap();
        assert!(store.automatic_sign_in().unwrap().is_none());
        store
            .sign_in_with_options("bob", "bob secure password", true)
            .unwrap();
        assert_eq!(store.automatic_sign_in().unwrap().as_deref(), Some("bob"));
        assert!(store.sign_in("alice", "alice secure password").is_ok());
        store.sign_out().unwrap();
        std::fs::remove_file(&store.path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
    #[test]
    #[cfg(windows)]
    fn remembered_signin_expires_and_is_revoked_by_logout_and_reset() {
        let directory = std::env::temp_dir().join(format!("lab-auto-auth-{}", std::process::id()));
        let store = AccountStore {
            path: directory.join("account.enc"),
        };
        store
            .register(
                "owner",
                "long secure password",
                [0, 1],
                ["first school", "nickname"],
            )
            .unwrap();
        assert!(store.automatic_sign_in().unwrap().is_none());
        store
            .sign_in_with_options("owner", "long secure password", true)
            .unwrap();
        assert_eq!(
            AccountStore {
                path: store.path.clone()
            }
            .automatic_sign_in()
            .unwrap()
            .as_deref(),
            Some("owner")
        );
        let stale = std::fs::read(directory.join("session.enc")).unwrap();
        store.sign_out().unwrap();
        std::fs::write(directory.join("session.enc"), stale).unwrap();
        assert!(store.automatic_sign_in().unwrap().is_none());
        store
            .sign_in_with_options("owner", "long secure password", true)
            .unwrap();
        let mut account = store.load().unwrap().unwrap();
        account.remembered_until = now() - 1;
        store.save(&account).unwrap();
        assert!(store.automatic_sign_in().unwrap().is_none());
        store
            .sign_in_with_options("owner", "long secure password", true)
            .unwrap();
        store
            .reset("owner", ["first school", "nickname"], "new secure password")
            .unwrap();
        assert!(store.automatic_sign_in().unwrap().is_none());
        store
            .sign_in_with_options("owner", "new secure password", true)
            .unwrap();
        store.sign_in("owner", "new secure password").unwrap();
        assert!(store.automatic_sign_in().unwrap().is_none());
        store.sign_out().unwrap();
        std::fs::remove_file(&store.path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
    #[test]
    fn attempt_delay_expires_without_permanent_lockout() {
        let mut attempts = Attempts {
            failures: 5,
            until: now().saturating_sub(1),
        };
        assert!(attempts.allowed().is_ok());
        attempts.fail();
        assert_eq!(attempts.failures, 1);
        assert_eq!(attempts.until, 0);
    }
    #[test]
    fn hashes_are_salted_and_answers_are_normalized() {
        let a = hash("correct long password").unwrap();
        let b = hash("correct long password").unwrap();
        assert_ne!(a, b);
        assert!(verify("correct long password", &a));
        assert!(!verify("incorrect", &a));
        assert_eq!(*normalize("  My   School "), "my school");
        assert!(validate_password("short").is_err());
    }
    #[test]
    #[cfg(windows)]
    fn encrypted_registration_signin_and_recovery() {
        let directory = std::env::temp_dir().join(format!("lab-auth-{}", std::process::id()));
        let store = AccountStore {
            path: directory.join("account.enc"),
        };
        assert!(store.username().unwrap().is_none());
        assert!(
            store
                .register(
                    "admin",
                    "long secure password",
                    [0, 0],
                    ["school", "nickname"]
                )
                .is_err()
        );
        store
            .register(
                "admin",
                "long secure password",
                [0, 1],
                ["School Name", "Nickname"],
            )
            .unwrap();
        let bytes = std::fs::read(&store.path).unwrap();
        assert!(!bytes.windows(5).any(|s| s == b"admin"));
        assert!(
            store
                .register(
                    "ADMIN",
                    "long secure password",
                    [0, 1],
                    ["school", "nickname"]
                )
                .is_err()
        );
        assert!(store.sign_in("admin", "wrong").is_err());
        assert_eq!(
            store.sign_in("ADMIN", "long secure password").unwrap(),
            "admin"
        );
        assert!(
            store
                .reset("admin", ["wrong", "nickname"], "new secure password")
                .is_err()
        );
        store
            .reset(
                "admin",
                [" school  name ", "NICKNAME"],
                "new secure password",
            )
            .unwrap();
        assert!(store.sign_in("admin", "long secure password").is_err());
        assert!(store.sign_in("admin", "new secure password").is_ok());
        for _ in 0..5 {
            assert!(store.sign_in("admin", "wrong").is_err());
        }
        assert!(
            store
                .sign_in("admin", "new secure password")
                .unwrap_err()
                .contains("Too many")
        );
        assert!(
            AccountStore {
                path: store.path.clone()
            }
            .sign_in("admin", "new secure password")
            .is_err()
        );
        store
            .reset("admin", ["school name", "nickname"], "recovered password")
            .unwrap();
        assert!(store.sign_in("admin", "recovered password").is_ok());
        let mut corrupt = std::fs::read(&store.path).unwrap();
        let last = corrupt.len() - 1;
        corrupt[last] ^= 1;
        std::fs::write(&store.path, corrupt).unwrap();
        assert!(store.username().is_err());
        std::fs::remove_file(&store.path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
