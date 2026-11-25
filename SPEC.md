# Lab Manager

Native Windows Rust desktop application using eframe. Rust structs with encapsulated methods separate Server, ServerStore, SshService, and LabManager UI responsibilities.

Build order: saved inventory → device gallery → interactive SSH and updates.

Accept one or many IPv4/IPv6 addresses separated by whitespace, commas, or semicolons. Validate the whole batch before saving and skip duplicates. Show a separate device page with a drawn server image, editable name and SSH settings, IP, browser link, and terminal actions. Remember entries in a portable data folder beside the executable. Never save passwords or private keys.

Use system OpenSSH with host verification in a separate interactive terminal launched by the application. Support an interactive remote shell, local PowerShell, and explicit review of a remote update command before launch. Default update is for Debian/Ubuntu; users select or edit commands for other systems. No server actions run on startup.

Persist changes before reporting success. Invalid or unreadable saved data must not be silently overwritten. Tests cover batch validation, IPv6 URLs, persistence and argument construction. Build with `cargo build`, test with `cargo test`, lint with `cargo clippy --all-targets -- -D warnings`, format with `cargo fmt`.
