use crate::{model::Server, monitoring::Config};
use portable_pty::{CommandBuilder, MasterPty, PtySize};
use std::{
    io::{Read, Write},
    path::Path,
    sync::{
        Arc, Mutex,
        mpsc::{self, SyncSender},
    },
};

pub const SCROLLBACK: usize = 2000;
pub struct Session {
    master: Option<Box<dyn MasterPty + Send>>,
    child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    input: Option<SyncSender<Vec<u8>>>,
    pub parser: Arc<Mutex<vt100::Parser>>,
    pub status: String,
    pub running: bool,
    size: (u16, u16),
    repaint: Arc<Mutex<Option<eframe::egui::Context>>>,
}
pub fn ssh_command(
    server: &Server,
    config: Option<&Config>,
    root: Option<&Path>,
    remote: Option<&str>,
) -> Result<CommandBuilder, String> {
    server.validate()?;
    let mut command = CommandBuilder::new(if cfg!(windows) { "ssh.exe" } else { "ssh" });
    command.args([
        "-tt",
        "-o",
        "ConnectTimeout=8",
        "-o",
        "ConnectionAttempts=1",
        "-o",
        "GSSAPIAuthentication=no",
        "-o",
        "ServerAliveInterval=15",
        "-o",
        "ServerAliveCountMax=3",
    ]);
    if let Some(option) = crate::ssh_trust::known_hosts_option(server, root)? {
        command.args(["-o", &option]);
    }
    let password_mode = config.is_some_and(Config::uses_password);
    if password_mode {
        command.args([
            "-o",
            "PreferredAuthentications=password,keyboard-interactive",
            "-o",
            "PubkeyAuthentication=no",
            "-o",
            "NumberOfPasswordPrompts=1",
        ]);
    }
    if let Some(config) = config.filter(|c| !c.uses_password())
        && let Some(path) = config.ssh_identity()?
    {
        command.arg("-i");
        command.arg(path);
        command.args(["-o", "IdentitiesOnly=yes"]);
    }
    let saved = config
        .filter(|c| c.uses_password())
        .and_then(|c| c.password_id.as_deref());
    if let (Some(id), Some(root)) = (saved, root) {
        let mut env = std::process::Command::new(if cfg!(windows) { "ssh.exe" } else { "ssh" });
        crate::monitoring_credentials::configure(&mut env, root, id, server)?;
        for (key, value) in env.get_envs() {
            if let Some(value) = value {
                command.env(key, value);
            }
        }
        command.args([
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            "PreferredAuthentications=password,keyboard-interactive",
            "-o",
            "NumberOfPasswordPrompts=1",
        ]);
    } else {
        command.args(["-o", "StrictHostKeyChecking=ask"]);
    }
    command.args([
        "-p",
        &server.ssh_port.to_string(),
        "-l",
        &server.username,
        &server.ip.to_string(),
    ]);
    if let Some(remote) = remote {
        if remote.trim().is_empty() || remote.contains('\0') {
            return Err("Enter a remote command.".into());
        }
        command.arg(remote);
    }
    command.env("TERM", "xterm-256color");
    Ok(command)
}
pub fn local_command() -> CommandBuilder {
    #[cfg(windows)]
    {
        let mut command = CommandBuilder::new("powershell.exe");
        command.args(["-NoLogo", "-NoProfile"]);
        command.env("TERM", "xterm-256color");
        command
    }
    #[cfg(not(windows))]
    {
        CommandBuilder::new(std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into()))
    }
}
impl Session {
    pub fn spawn(command: CommandBuilder, rows: u16, cols: u16) -> Result<Self, String> {
        let rows = rows.clamp(2, 160);
        let cols = cols.clamp(10, 320);
        let pair = portable_pty::native_pty_system()
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Cannot create embedded terminal: {e}"))?;
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let mut writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(64);
        std::thread::spawn(move || {
            use zeroize::Zeroize;
            while let Ok(mut bytes) = rx.recv() {
                let result = writer.write_all(&bytes).and_then(|_| writer.flush());
                bytes.zeroize();
                if result.is_err() {
                    break;
                }
            }
        });
        let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, SCROLLBACK)));
        let screen = parser.clone();
        let replies = tx.clone();
        let repaint = Arc::new(Mutex::new(None::<eframe::egui::Context>));
        let output_repaint = repaint.clone();
        std::thread::spawn(move || {
            let mut bytes = [0u8; 4096];
            let mut tail = Vec::new();
            loop {
                let n = match reader.read(&mut bytes) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                if let Ok(mut parser) = screen.lock() {
                    parser.process(&bytes[..n]);
                    tail.extend_from_slice(&bytes[..n]);
                    if tail.windows(4).any(|w| w == b"\x1b[6n") {
                        let (r, c) = parser.screen().cursor_position();
                        let _ = replies.try_send(format!("\x1b[{};{}R", r + 1, c + 1).into_bytes());
                    }
                    if tail.windows(3).any(|w| w == b"\x1b[c") {
                        let _ = replies.try_send(b"\x1b[?1;2c".to_vec());
                    }
                    // Retain only an incomplete escape prefix across output chunks.
                    let keep = tail.iter().rposition(|b| *b == 0x1b).filter(|i| {
                        tail.len() - i < 4
                            && !tail[*i..].ends_with(b"n")
                            && !tail[*i..].ends_with(b"c")
                    });
                    tail = keep.map_or_else(Vec::new, |i| tail[i..].to_vec());
                } else {
                    break;
                }
                if let Ok(context) = output_repaint.lock()
                    && let Some(context) = &*context
                {
                    context.request_repaint_after(std::time::Duration::from_millis(33));
                }
            }
        });
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|e| format!("Cannot start terminal process: {e}"))?;
        drop(pair.slave);
        Ok(Self {
            master: Some(pair.master),
            child: Some(child),
            input: Some(tx),
            parser,
            status: "Session running".into(),
            running: true,
            size: (rows, cols),
            repaint,
        })
    }
    pub fn repaint_on_output(&self, ctx: &eframe::egui::Context) {
        if let Ok(mut context) = self.repaint.lock()
            && context.is_none()
        {
            *context = Some(ctx.clone());
        }
    }
    pub fn send(&self, bytes: Vec<u8>) -> Result<(), String> {
        if bytes.len() > 1024 * 1024 {
            return Err("Paste is limited to 1 MiB.".into());
        }
        self.input
            .as_ref()
            .ok_or("Terminal disconnected")?
            .try_send(bytes)
            .map_err(|_| "Terminal input is busy or disconnected; try again.".into())
    }
    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<(), String> {
        let size = (rows.clamp(2, 160), cols.clamp(10, 320));
        if self.size == size {
            return Ok(());
        }
        self.master
            .as_ref()
            .ok_or("Terminal closed")?
            .resize(PtySize {
                rows: size.0,
                cols: size.1,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;
        if let Ok(mut parser) = self.parser.lock() {
            parser.set_size(size.0, size.1);
        }
        self.size = size;
        Ok(())
    }
    pub fn poll(&mut self) {
        if !self.running {
            return;
        }
        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.running = false;
                    self.status = format!("Session ended: {status}");
                }
                Err(e) => {
                    self.running = false;
                    self.status = format!("Terminal error: {e}");
                }
                _ => {}
            }
        }
    }
    pub fn disconnect(&mut self) {
        self.running = false;
        self.status = "Disconnected".into();
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        self.input = None;
        // ConPTY shutdown can wait for its output drain; never block the UI thread.
        if let Some(master) = self.master.take() {
            std::thread::spawn(move || drop(master));
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn password_mode_skips_agent_keys_and_bounds_connection_attempts() {
        let server = Server::new("192.0.2.1".parse().unwrap());
        let config = Config {
            auth_mode: Some(crate::monitoring::AuthMode::Password),
            ..Default::default()
        };
        let command = ssh_command(&server, Some(&config), None, None).unwrap();
        let args: Vec<_> = command
            .get_argv()
            .iter()
            .map(|arg| arg.to_string_lossy())
            .collect();
        for expected in [
            "ConnectTimeout=8",
            "ConnectionAttempts=1",
            "GSSAPIAuthentication=no",
            "PubkeyAuthentication=no",
            "StrictHostKeyChecking=ask",
            "PreferredAuthentications=password,keyboard-interactive",
        ] {
            assert!(args.iter().any(|arg| arg == expected), "missing {expected}");
        }
        assert!(
            !args
                .iter()
                .any(|arg| arg.contains("accept-new") || arg.contains("StrictHostKeyChecking=no"))
        );
    }

    #[test]
    fn explicit_key_path_is_a_single_argument_and_ignored_in_password_mode() {
        let server = Server::new("192.0.2.1".parse().unwrap());
        let mut config = Config {
            identity_file: r"C:\My Keys\id_ed25519".into(),
            ..Default::default()
        };
        let command = ssh_command(&server, Some(&config), None, None).unwrap();
        let args: Vec<_> = command
            .get_argv()
            .iter()
            .map(|arg| arg.to_string_lossy())
            .collect();
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "-i" && pair[1] == r"C:\My Keys\id_ed25519")
        );
        config.auth_mode = Some(crate::monitoring::AuthMode::Password);
        let command = ssh_command(&server, Some(&config), None, None).unwrap();
        assert!(!command.get_argv().iter().any(|arg| arg == "-i"));
        config.auth_mode = Some(crate::monitoring::AuthMode::Key);
        config.identity_file = "bad\npath".into();
        assert!(ssh_command(&server, Some(&config), None, None).is_err());
    }

    #[test]
    fn parser_tracks_color_cursor_overwrites_and_alternate_screen() {
        let mut p = vt100::Parser::new(10, 40, 100);
        p.process(b"hello\r\x1b[31mworld\x1b[0m");
        assert_eq!(p.screen().cell(0, 0).unwrap().contents(), "w");
        assert_eq!(
            p.screen().cell(0, 0).unwrap().fgcolor(),
            vt100::Color::Idx(1)
        );
        p.process(b"\x1b[?1049h\x1b[2Jtop");
        assert!(p.screen().alternate_screen());
        p.process(b"\x1b[?1049l");
        assert!(p.screen().contents().contains("world"));
    }
    #[test]
    fn native_terminal_runs_reads_input_resizes_and_disconnects() {
        let mut s = Session::spawn(local_command(), 20, 80).unwrap();
        s.send(if cfg!(windows) {
            b"[Console]::WriteLine(('LAB_' + 'TERMINAL_READY'))\r".to_vec()
        } else {
            b"printf '%s%s\\n' 'LAB_' 'TERMINAL_READY'\r".to_vec()
        })
        .unwrap();
        let start = std::time::Instant::now();
        loop {
            let text = s.parser.lock().unwrap().screen().contents();
            if text.contains("LAB_TERMINAL_READY") {
                break;
            }
            assert!(
                start.elapsed() < std::time::Duration::from_secs(15),
                "No terminal output: {text}"
            );
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        s.resize(30, 100).unwrap();
        assert_eq!(s.parser.lock().unwrap().screen().size(), (30, 100));
        s.disconnect();
        assert!(!s.running);
        assert!(s.child.is_none());
    }
}
