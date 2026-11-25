use crate::{model::Server, monitoring::Config};
use serde::{Deserialize, Serialize};
use std::{
    net::{SocketAddr, TcpStream, UdpSocket},
    path::Path,
    sync::atomic::AtomicBool,
    time::Duration,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    #[default]
    Tcp,
    Dns,
    Systemd,
    Docker,
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tcp => "TCP port",
            Self::Dns => "DNS lookup",
            Self::Systemd => "Linux service",
            Self::Docker => "Docker container",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceCheck {
    pub name: String,
    pub kind: Kind,
    pub port: u16,
    pub target: String,
}
impl Default for ServiceCheck {
    fn default() -> Self {
        Self {
            name: "Custom service".into(),
            kind: Kind::Tcp,
            port: 443,
            target: String::new(),
        }
    }
}
impl ServiceCheck {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() || self.name.len() > 80 {
            return Err("Service name must be 1–80 characters.".into());
        }
        if self.port == 0 {
            return Err("Service port must be 1–65535.".into());
        }
        match self.kind {
            Kind::Tcp => Ok(()),
            Kind::Dns => {
                if self.target.len() <= 253
                    && !self.target.is_empty()
                    && self.target.split('.').all(|label| {
                        !label.is_empty()
                            && label.len() <= 63
                            && label
                                .bytes()
                                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
                    })
                {
                    Ok(())
                } else {
                    Err("Enter a DNS name such as example.com.".into())
                }
            }
            Kind::Systemd | Kind::Docker => {
                if !self.target.is_empty()
                    && self.target.len() <= 128
                    && !self.target.starts_with('-')
                    && self
                        .target
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"._@-".contains(&c))
                {
                    Ok(())
                } else {
                    Err("Service/container name may contain letters, numbers, dot, underscore, @ and hyphen.".into())
                }
            }
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceResult {
    pub name: String,
    pub online: bool,
    pub detail: String,
}
pub fn probe(
    server: &Server,
    check: &ServiceCheck,
    config: &Config,
    cancel: &AtomicBool,
    root: Option<&Path>,
) -> ServiceResult {
    let result = check.validate().and_then(|()| match check.kind {
        Kind::Tcp => TcpStream::connect_timeout(
            &SocketAddr::new(server.ip, check.port),
            Duration::from_millis(800),
        )
        .map(|_| format!("Port {} open", check.port))
        .map_err(|_| format!("Port {} unavailable", check.port)),
        Kind::Dns => dns(SocketAddr::new(server.ip, check.port), &check.target),
        Kind::Systemd | Kind::Docker => {
            let command = if check.kind == Kind::Systemd {
                format!("systemctl is-active -- '{}'", check.target)
            } else {
                format!(
                    "docker inspect --format '{{{{.State.Running}}}}' -- '{}'",
                    check.target
                )
            };
            crate::monitoring_probe::ssh_output(server, config, cancel, root, &command).and_then(
                |output| {
                    let expected = if check.kind == Kind::Systemd {
                        "active"
                    } else {
                        "true"
                    };
                    if output.trim() == expected {
                        Ok("Running".into())
                    } else {
                        Err("Not running or not accessible".into())
                    }
                },
            )
        }
    });
    match result {
        Ok(detail) => ServiceResult {
            name: check.name.clone(),
            online: true,
            detail,
        },
        Err(detail) => ServiceResult {
            name: check.name.clone(),
            online: false,
            detail,
        },
    }
}
fn dns(address: SocketAddr, name: &str) -> Result<String, String> {
    let mut id = [0; 2];
    use rand_core::RngCore;
    rand_core::OsRng.fill_bytes(&mut id);
    let mut packet = vec![id[0], id[1], 1, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in name.split('.') {
        packet.push(label.len() as u8);
        packet.extend(label.as_bytes());
    }
    packet.extend([0, 0, 1, 0, 1]);
    let socket = UdpSocket::bind(if address.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    })
    .map_err(|e| e.to_string())?;
    socket
        .set_read_timeout(Some(Duration::from_millis(1200)))
        .map_err(|e| e.to_string())?;
    socket.connect(address).map_err(|e| e.to_string())?;
    socket.send(&packet).map_err(|e| e.to_string())?;
    let mut response = [0; 4096];
    let size = socket
        .recv(&mut response)
        .map_err(|_| "DNS query timed out or failed".to_owned())?;
    if size < 12
        || response[..2] != id
        || response[2] & 0x80 == 0
        || response[3] & 15 != 0
        || u16::from_be_bytes([response[6], response[7]]) == 0
    {
        return Err("DNS returned no successful answer".into());
    }
    Ok(format!("Resolved {name}"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_shell_injection_and_invalid_dns_targets() {
        for target in ["x;reboot", "$(id)", "a'b", "--help", "a\nb"] {
            assert!(
                ServiceCheck {
                    kind: Kind::Systemd,
                    target: target.into(),
                    ..Default::default()
                }
                .validate()
                .is_err()
            );
        }
        assert!(
            ServiceCheck {
                kind: Kind::Dns,
                target: "example.com".into(),
                ..Default::default()
            }
            .validate()
            .is_ok()
        );
        assert!(
            ServiceCheck {
                kind: Kind::Dns,
                target: "a..com".into(),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
    #[test]
    fn dns_requires_matching_successful_reply() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let address = socket.local_addr().unwrap();
        let worker = std::thread::spawn(move || {
            let mut bytes = [0; 512];
            let (size, peer) = socket.recv_from(&mut bytes).unwrap();
            bytes[2] = 0x81;
            bytes[3] = 0x80;
            bytes[7] = 1;
            socket.send_to(&bytes[..size], peer).unwrap();
        });
        assert!(dns(address, "example.com").is_ok());
        worker.join().unwrap();
    }
}
