use std::{
    net::{Ipv4Addr, SocketAddr, TcpStream, UdpSocket},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver},
    },
    time::Duration,
};

pub const PORTS: [u16; 6] = [22, 80, 443, 8006, 8080, 8443];
#[derive(Clone, Debug)]
pub struct Device {
    pub ip: Ipv4Addr,
    pub ports: Vec<u16>,
}
pub enum Event {
    Found(Device),
    Progress,
    Done,
}
pub struct Scan {
    pub receiver: Receiver<Event>,
    cancel: Arc<AtomicBool>,
    pub total: usize,
}
impl Scan {
    pub fn start(cidr: &str) -> Result<Self, String> {
        let addresses = Arc::new(parse_range(cidr)?);
        let total = addresses.len();
        let (sender, receiver) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        std::thread::spawn(move || {
            let next = AtomicUsize::new(0);
            std::thread::scope(|scope| {
                for _ in 0..24.min(total) {
                    let (sender, addresses, cancel, next) = (
                        sender.clone(),
                        addresses.clone(),
                        worker_cancel.clone(),
                        &next,
                    );
                    scope.spawn(move || {
                        loop {
                            if cancel.load(Ordering::Relaxed) {
                                break;
                            }
                            let index = next.fetch_add(1, Ordering::Relaxed);
                            let Some(ip) = addresses.get(index).copied() else {
                                break;
                            };
                            let ports = PORTS
                                .into_iter()
                                .take_while(|_| !cancel.load(Ordering::Relaxed))
                                .filter(|port| probe(ip, *port, Duration::from_millis(250)))
                                .collect::<Vec<_>>();
                            if !ports.is_empty()
                                && sender.send(Event::Found(Device { ip, ports })).is_err()
                            {
                                break;
                            }
                            if sender.send(Event::Progress).is_err() {
                                break;
                            }
                        }
                    });
                }
            });
            let _ = sender.send(Event::Done);
        });
        Ok(Self {
            receiver,
            cancel,
            total,
        })
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
impl Drop for Scan {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub fn suggested_range() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|socket| {
            socket.connect("192.0.2.1:9")?;
            socket.local_addr()
        })
        .ok()
        .and_then(|addr| match addr.ip() {
            std::net::IpAddr::V4(ip) if ip.is_private() => Some(ip),
            _ => None,
        })
        .map(|ip| {
            let [a, b, c, _] = ip.octets();
            format!("{a}.{b}.{c}.0/24")
        })
        .unwrap_or_default()
}
pub fn parse_range(value: &str) -> Result<Vec<Ipv4Addr>, String> {
    let (address, prefix) = value
        .trim()
        .split_once('/')
        .ok_or("Enter an IPv4 CIDR, such as 192.168.1.0/24.")?;
    let ip: Ipv4Addr = address.parse().map_err(|_| "Invalid IPv4 address.")?;
    let prefix: u32 = prefix.parse().map_err(|_| "Invalid prefix length.")?;
    if !(22..=32).contains(&prefix) {
        return Err("Use /22 through /32 (at most 1024 addresses).".into());
    }
    let network = u32::from(ip) & (u32::MAX << (32 - prefix));
    let count = 1_u32 << (32 - prefix);
    let local = |ip: Ipv4Addr| ip.is_private() || ip.is_loopback() || ip.is_link_local();
    if !local(Ipv4Addr::from(network)) || !local(Ipv4Addr::from(network + count - 1)) {
        return Err("Discovery is limited to private, link-local, or loopback networks.".into());
    }
    let hosts = if prefix < 31 {
        network + 1..network + count - 1
    } else {
        network..network + count
    };
    Ok(hosts.map(Ipv4Addr::from).collect())
}
fn probe(ip: Ipv4Addr, port: u16, timeout: Duration) -> bool {
    TcpStream::connect_timeout(&SocketAddr::new(ip.into(), port), timeout).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges_are_bounded_and_do_not_target_public_networks() {
        assert_eq!(parse_range("192.168.2.18/24").unwrap().len(), 254);
        assert_eq!(
            parse_range("10.0.0.1/32").unwrap(),
            vec![Ipv4Addr::new(10, 0, 0, 1)]
        );
        for invalid in ["0.0.0.0/0", "8.8.8.0/24", "10.0.0.0/16", "bad/24"] {
            assert!(parse_range(invalid).is_err());
        }
    }
    #[test]
    fn detects_a_listening_service_without_sending_payloads() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        assert!(probe(
            Ipv4Addr::LOCALHOST,
            listener.local_addr().unwrap().port(),
            Duration::from_millis(100)
        ));
    }
}
