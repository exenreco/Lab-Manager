//! Reuse host identities already trusted by PuTTY; never accept a key from the network.
#[cfg(any(windows, test))]
use base64::{Engine, engine::general_purpose::STANDARD};
use std::path::Path;

#[cfg(any(windows, test))]
fn rsa_key(value: &str) -> Option<String> {
    fn field(out: &mut Vec<u8>, bytes: &[u8]) {
        out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        out.extend_from_slice(bytes);
    }
    fn integer(value: &str) -> Option<Vec<u8>> {
        let text = value.trim().strip_prefix("0x")?;
        if text.is_empty() || text.len() > 16384 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let text = if text.len() % 2 == 1 {
            format!("0{text}")
        } else {
            text.into()
        };
        let mut bytes: Vec<u8> = (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
            .collect::<Option<_>>()?;
        while bytes.len() > 1 && bytes[0] == 0 {
            bytes.remove(0);
        }
        if bytes[0] & 128 != 0 {
            bytes.insert(0, 0);
        }
        Some(bytes)
    }
    let (exponent, modulus) = value.split_once(',')?;
    let mut blob = Vec::new();
    field(&mut blob, b"ssh-rsa");
    field(&mut blob, &integer(exponent)?);
    field(&mut blob, &integer(modulus)?);
    Some(format!("ssh-rsa {}", STANDARD.encode(blob)))
}

#[cfg(windows)]
fn putty_rsa(ip: &str, port: u16) -> Option<String> {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW};
    let wide = |text: &str| text.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let key = wide("Software\\SimonTatham\\PuTTY\\SshHostKeys");
    let name = wide(&format!("rsa2@{port}:{ip}"));
    let mut bytes = 0;
    // Read one public host-key value, never saved credentials or private key material.
    unsafe {
        if RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut bytes,
        ) != 0
            || !(2..=65536).contains(&bytes)
        {
            return None;
        }
        let mut value = vec![0u16; bytes as usize / 2];
        if RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            value.as_mut_ptr().cast(),
            &mut bytes,
        ) != 0
        {
            return None;
        }
        value.truncate(bytes as usize / 2);
        let length = value.iter().position(|c| *c == 0).unwrap_or(value.len());
        rsa_key(&String::from_utf16(&value[..length]).ok()?)
    }
}
#[cfg(not(windows))]
fn putty_rsa(_ip: &str, _port: u16) -> Option<String> {
    None
}

pub fn known_hosts_option(
    server: &crate::model::Server,
    root: Option<&Path>,
) -> Result<Option<String>, String> {
    let Some(root) = root else {
        return Ok(None);
    };
    let Some(key) = putty_rsa(&server.ip.to_string(), server.ssh_port) else {
        return Ok(None);
    };
    let root = if root.is_absolute() {
        root.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(root)
    };
    let folder = root.join("trusted-hosts");
    std::fs::create_dir_all(&folder)
        .map_err(|e| format!("Cannot prepare PuTTY host trust: {e}"))?;
    let file = folder.join(format!(
        "{}-{}.known_hosts",
        server.ip.to_string().replace(':', "_"),
        server.ssh_port
    ));
    let host = if server.ssh_port == 22 {
        server.ip.to_string()
    } else {
        format!("[{}]:{}", server.ip, server.ssh_port)
    };
    let content = format!("{host} {key}\n");
    if std::fs::read_to_string(&file).ok().as_deref() != Some(&content) {
        std::fs::write(&file, content)
            .map_err(|e| format!("Cannot store PuTTY host trust: {e}"))?;
    }
    let home = directories::BaseDirs::new().ok_or("Home directory unavailable")?;
    let quote = |p: &Path| format!("\"{}\"", p.to_string_lossy().replace('\\', "/"));
    // Existing OpenSSH host keys remain in force; changed keys still fail strict checking.
    Ok(Some(format!(
        "UserKnownHostsFile={} {}",
        quote(&home.home_dir().join(".ssh/known_hosts")),
        quote(&file)
    )))
}

#[cfg(test)]
mod tests {
    #[test]
    fn putty_rsa_encoding_handles_unsigned_mpints_and_invalid_data() {
        let key = super::rsa_key("0x10001,0x80ff").unwrap();
        let blob = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            key.split_once(' ').unwrap().1,
        )
        .unwrap();
        assert_eq!(
            &blob,
            b"\0\0\0\x07ssh-rsa\0\0\0\x03\x01\0\x01\0\0\0\x03\0\x80\xff"
        );
        for invalid in ["", "0x1", "0xzz,0xff", "0x1,0x"] {
            assert!(super::rsa_key(invalid).is_none());
        }
    }
}
