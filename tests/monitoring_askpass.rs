#[cfg(windows)]
#[test]
fn hidden_helper_returns_only_disposable_password_and_rejects_other_prompts() {
    use lab_manager::{model::Server, monitoring_credentials};
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    let root = std::env::temp_dir().join(format!("lab-askpass-test-{}", std::process::id()));
    let server = Server::new("192.0.2.55".parse().unwrap());
    let id = monitoring_credentials::save(&root, &server, "disposable-helper-test", None).unwrap();
    for (prompt, success) in [
        ("root@192.0.2.55's password:", true),
        ("Enter passphrase for key", false),
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_lab-manager"));
        monitoring_credentials::configure(&mut command, &root, &id, &server).unwrap();
        assert!(
            !command
                .get_args()
                .any(|v| v.to_string_lossy().contains("disposable-helper-test"))
        );
        assert!(!command.get_envs().any(|(_, v)| {
            v.is_some_and(|v| v.to_string_lossy().contains("disposable-helper-test"))
        }));
        let output = command
            .arg(prompt)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert_eq!(output.status.success(), success);
        if success {
            assert!(output.stdout == b"disposable-helper-test\n");
        } else {
            assert!(output.stdout.is_empty());
        }
    }
    monitoring_credentials::remove(&root, &id).unwrap();
    let _ = std::fs::remove_dir(root);
}
