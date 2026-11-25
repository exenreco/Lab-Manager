use crate::model::Server;
use std::net::IpAddr;

/// Insert a device before/after its target, preserving all other relative order.
pub fn reordered(
    servers: &[Server],
    source: IpAddr,
    target: IpAddr,
    after: bool,
) -> Option<Vec<Server>> {
    if source == target {
        return None;
    }
    let from = servers.iter().position(|s| s.ip == source)?;
    servers.iter().position(|s| s.ip == target)?;
    let mut result = servers.to_vec();
    let device = result.remove(from);
    let to = result.iter().position(|s| s.ip == target)? + usize::from(after);
    result.insert(to, device);
    (result != servers).then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moves_both_directions_without_losing_hidden_devices() {
        let servers: Vec<Server> = (1..=4)
            .map(|n| {
                serde_json::from_value(serde_json::json!({
                    "name": n.to_string(), "ip": format!("192.168.1.{n}"), "web_port": 80,
                    "ssh_port": 22, "https": false, "username": "root"
                }))
                .unwrap()
            })
            .collect();
        let names = |s: Vec<Server>| s.into_iter().map(|s| s.name).collect::<Vec<_>>();
        assert_eq!(
            names(reordered(&servers, servers[0].ip, servers[2].ip, true).unwrap()),
            ["2", "3", "1", "4"]
        );
        assert_eq!(
            names(reordered(&servers, servers[3].ip, servers[1].ip, false).unwrap()),
            ["1", "4", "2", "3"]
        );
        assert!(reordered(&servers, servers[0].ip, servers[1].ip, false).is_none());
        assert!(reordered(&servers, servers[0].ip, servers[0].ip, true).is_none());
        assert!(
            reordered(
                &servers,
                "192.168.1.99".parse().unwrap(),
                servers[0].ip,
                false
            )
            .is_none()
        );
    }
}
