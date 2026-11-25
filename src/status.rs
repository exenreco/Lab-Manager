use crate::model::Server;
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver},
    },
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
struct Endpoint {
    ip: IpAddr,
    web: u16,
    ssh: u16,
}
impl From<&Server> for Endpoint {
    fn from(s: &Server) -> Self {
        Self {
            ip: s.ip,
            web: s.web_port,
            ssh: s.ssh_port,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Connection {
    Checking,
    Online,
    Offline,
}
#[derive(Clone, Copy)]
pub struct Status {
    pub connection: Connection,
    pub checked: Option<Instant>,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            connection: Connection::Checking,
            checked: None,
        }
    }
}
pub struct Monitor {
    targets: Vec<Endpoint>,
    values: HashMap<Endpoint, Status>,
    receiver: Option<Receiver<(Endpoint, Connection)>>,
    cancel: Arc<AtomicBool>,
    last_start: Option<Instant>,
}
impl Default for Monitor {
    fn default() -> Self {
        Self {
            targets: Vec::new(),
            values: HashMap::new(),
            receiver: None,
            cancel: Arc::new(AtomicBool::new(false)),
            last_start: None,
        }
    }
}
impl Monitor {
    pub fn reset(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        *self = Self::default();
    }
    pub fn get(&self, server: &Server) -> Status {
        self.values
            .get(&Endpoint::from(server))
            .copied()
            .unwrap_or_default()
    }
    pub fn update(&mut self, servers: &[Server]) {
        let targets: Vec<_> = servers.iter().map(Endpoint::from).collect();
        if targets != self.targets {
            self.cancel.store(true, Ordering::Relaxed);
            self.receiver = None;
            self.last_start = None;
            self.values.retain(|key, _| targets.contains(key));
            self.targets = targets;
        }
        let mut finished = false;
        if let Some(receiver) = &self.receiver {
            loop {
                match receiver.try_recv() {
                    Ok((key, connection)) => {
                        self.values.insert(
                            key,
                            Status {
                                connection,
                                checked: Some(Instant::now()),
                            },
                        );
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        finished = true;
                        break;
                    }
                }
            }
        }
        if finished {
            self.receiver = None;
        }
        if self.targets.is_empty()
            || self.receiver.is_some()
            || self
                .last_start
                .is_some_and(|start| start.elapsed() < Duration::from_secs(30))
        {
            return;
        }
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.cancel.clone();
        let targets = Arc::new(self.targets.clone());
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.last_start = Some(Instant::now());
        std::thread::spawn(move || {
            let next = AtomicUsize::new(0);
            std::thread::scope(|scope| {
                for _ in 0..8.min(targets.len()) {
                    let (targets, cancel, sender, next) =
                        (targets.clone(), cancel.clone(), sender.clone(), &next);
                    scope.spawn(move || {
                        loop {
                            if cancel.load(Ordering::Relaxed) {
                                break;
                            }
                            let Some(target) =
                                targets.get(next.fetch_add(1, Ordering::Relaxed)).copied()
                            else {
                                break;
                            };
                            let connection = check(target);
                            if cancel.load(Ordering::Relaxed)
                                || sender.send((target, connection)).is_err()
                            {
                                break;
                            }
                        }
                    });
                }
            });
        });
    }
}
impl Drop for Monitor {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
fn check(target: Endpoint) -> Connection {
    for port in [target.web, target.ssh] {
        if TcpStream::connect_timeout(
            &SocketAddr::new(target.ip, port),
            Duration::from_millis(450),
        )
        .is_ok()
        {
            return Connection::Online;
        }
    }
    Connection::Offline
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn status_tracks_reachable_and_unreachable_configured_ports() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let target = Endpoint {
            ip: addr.ip(),
            web: addr.port(),
            ssh: addr.port(),
        };
        assert_eq!(check(target), Connection::Online);
        drop(listener);
        assert_eq!(check(target), Connection::Offline);
    }
    #[test]
    fn account_switch_discards_cached_status_and_pending_results() {
        let mut monitor = Monitor::default();
        let server = Server::new("127.0.0.1".parse().unwrap());
        monitor.values.insert(
            Endpoint::from(&server),
            Status {
                connection: Connection::Online,
                checked: Some(Instant::now()),
            },
        );
        monitor.reset();
        assert_eq!(monitor.get(&server).connection, Connection::Checking);
    }
}
