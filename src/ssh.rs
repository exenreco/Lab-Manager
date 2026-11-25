use crate::model::Server;
#[cfg(windows)]
use std::process::Command;

pub struct SshService;

impl SshService {
    pub fn arguments(server: &Server, command: Option<&str>) -> Result<Vec<String>, String> {
        server.validate()?;
        let mut args = vec![
            "-t".into(),
            "-o".into(),
            "StrictHostKeyChecking=ask".into(),
            "-p".into(),
            server.ssh_port.to_string(),
            "-l".into(),
            server.username.clone(),
            server.ip.to_string(),
        ];
        if let Some(command) = command {
            if command.trim().is_empty() || command.contains('\0') {
                return Err("Enter a remote command.".into());
            }
            args.push(command.into());
        }
        Ok(args)
    }

    pub fn connect(server: &Server, command: Option<&str>) -> Result<(), String> {
        let args = Self::arguments(server, command)?;
        // Encoded PowerShell avoids local interpolation of the remote command.
        let quote = |s: &str| format!("'{}'", s.replace('\'', "''"));
        let arguments = args
            .iter()
            .map(|s| windows_quote(s))
            .collect::<Vec<_>>()
            .join(" ");
        let script = format!(
            "$si = New-Object System.Diagnostics.ProcessStartInfo; $si.FileName = 'ssh.exe'; $si.UseShellExecute = $false; $si.Arguments = {}; $p = [System.Diagnostics.Process]::Start($si); $p.WaitForExit(); Write-Host ('SSH exited with code ' + $p.ExitCode)",
            quote(&arguments)
        );
        Self::powershell(Some(&script))
    }

    pub fn local_shell() -> Result<(), String> {
        Self::powershell(None)
    }

    fn powershell(script: Option<&str>) -> Result<(), String> {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let mut process = Command::new("powershell.exe");
            process.args(["-NoLogo", "-NoProfile", "-NoExit"]);
            if let Some(script) = script {
                let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
                process.args(["-EncodedCommand", &base64(&bytes)]);
            }
            process
                .creation_flags(0x00000010)
                .spawn()
                .map_err(|e| format!("Cannot open terminal: {e}"))?;
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = script;
            Err("This terminal integration currently requires Windows and OpenSSH.".into())
        }
    }
}

// Windows argv escaping, before embedding as a literal in the encoded script.
fn windows_quote(value: &str) -> String {
    let mut result = String::from("\"");
    let mut slashes = 0;
    for ch in value.chars() {
        if ch == '\\' {
            slashes += 1;
            continue;
        }
        if ch == '"' {
            result.push_str(&"\\".repeat(slashes * 2 + 1));
        } else {
            result.push_str(&"\\".repeat(slashes));
        }
        result.push(ch);
        slashes = 0;
    }
    result.push_str(&"\\".repeat(slashes * 2));
    result.push('"');
    result
}

#[cfg(windows)]
fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    for chunk in bytes.chunks(3) {
        let n = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;
        output.push(TABLE[((n >> 18) & 63) as usize] as char);
        output.push(TABLE[((n >> 12) & 63) as usize] as char);
        output.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_remains_a_single_remote_argument() {
        let server = Server::new("::1".parse().unwrap());
        let command = "sudo apt update && echo '$HOME'";
        let args = SshService::arguments(&server, Some(command)).unwrap();
        assert_eq!(args.last().unwrap(), command);
        assert_eq!(args[7], "::1");
        assert!(SshService::arguments(&server, Some(" ")).is_err());
    }
    #[test]
    fn preserves_quotes_and_backslashes() {
        assert_eq!(windows_quote("echo \"hello\""), "\"echo \\\"hello\\\"\"");
        assert_eq!(windows_quote("end\\"), "\"end\\\\\"");
    }
    #[test]
    #[cfg(windows)]
    fn encodes_powershell_payload() {
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"foo"), "Zm9v");
    }
}
