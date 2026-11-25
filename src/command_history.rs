use crate::model::Server;
use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub endpoint: String,
    pub device: String,
    pub timestamp: u64,
    pub command: String,
    pub result: String,
}
pub fn endpoint(server: &Server) -> String {
    format!("{}@{}:{}", server.username, server.ip, server.ssh_port)
}
pub fn safe_command(command: &str) -> &str {
    match command.trim() {
        "sudo apt update && sudo apt upgrade" => "sudo apt update && sudo apt upgrade",
        "sudo dnf upgrade" => "sudo dnf upgrade",
        "sudo pacman -Syu" => "sudo pacman -Syu",
        "apt update && apt upgrade" => "apt update && apt upgrade",
        "dnf upgrade" => "dnf upgrade",
        "pacman -Syu" => "pacman -Syu",
        "softwareupdate --list" => "softwareupdate --list",
        "racadm getsysinfo" => "racadm getsysinfo",
        _ => "Custom command (content omitted for privacy)",
    }
}
pub fn load(path: &Path) -> Result<Vec<Entry>, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e.to_string()),
    }
}
pub fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    use std::io::Write;
    let parent = path.parent().ok_or("Missing data directory")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = path.with_extension("tmp");
    let mut file = std::fs::File::create(&temp).map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(temp, path).map_err(|e| e.to_string())
}
pub fn start(path: &Path, server: &Server, command: &str) -> Result<String, String> {
    use rand_core::RngCore;
    let id = format!(
        "{:016x}{:016x}",
        rand_core::OsRng.next_u64(),
        rand_core::OsRng.next_u64()
    );
    let mut entries = load(path)?;
    entries.push(Entry {
        id: id.clone(),
        endpoint: endpoint(server),
        device: server.name.clone(),
        timestamp: crate::notifications::now(),
        command: safe_command(command).into(),
        result: "Running · result available while this session is open".into(),
    });
    if entries.len() > 500 {
        entries.drain(..entries.len() - 500);
    }
    write_json(path, &entries)?;
    Ok(id)
}
pub fn finish(path: &Path, id: &str, status: &str) -> Result<(), String> {
    let mut entries = load(path)?;
    if let Some(entry) = entries.iter_mut().find(|entry| entry.id == id) {
        entry.result = status.chars().take(160).collect();
    }
    write_json(path, &entries)
}
pub fn recover_interrupted(path: &Path) -> Result<(), String> {
    let mut entries = load(path)?;
    let mut changed = false;
    for entry in &mut entries {
        if entry.result.starts_with("Running") {
            entry.result = "Previous client session ended · remote completion unknown".into();
            changed = true;
        }
    }
    if changed {
        write_json(path, &entries)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_command_secrets_are_not_recorded() {
        assert!(!safe_command("curl -H 'Authorization: secret' example.com").contains("secret"));
        assert_eq!(safe_command("sudo dnf upgrade"), "sudo dnf upgrade");
    }
}
