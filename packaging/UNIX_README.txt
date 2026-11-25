Lab Manager - Linux and macOS prerelease builds

Linux: Ubuntu 24.04+ x86_64. Install the .deb with your package manager,
or extract the tar.gz and run ./lab-manager. A graphical desktop, OpenGL,
OpenSSH client, curl, a desktop file portal, and an unlocked Secret Service
keyring (such as GNOME Keyring) are required. No plaintext credential
fallback is used when the keyring is unavailable.

macOS: use the DMG matching Apple Silicon (aarch64) or Intel (x86_64).
Drag Lab Manager.app into Applications. These builds are ad-hoc signed,
not Apple Developer ID signed or notarized. macOS may block them under
its security policy. They are experimental builds for testing.

Account files are encrypted with ChaCha20-Poly1305; the encryption key is
kept in macOS Keychain or Linux Secret Service. Keep that keyring when
moving or backing up your account. Windows DPAPI account files cannot be
copied to these platforms. Register a local account and restore a device
backup instead. Accounts and SSH passwords are not included in backups.

App data is stored in the platform's per-user application-data folder,
separate from the executable or .app bundle. Uninstalling leaves it intact.

The macOS toolbar stays at the top right, with left-to-right controls,
traffic-light window buttons, Profile nearest the window buttons, and
rounded right corners. Windows/Linux keep the existing toolbar order.

Updates select the current OS and architecture. Linux archives must be
extracted/replaced manually; macOS DMGs are opened for manual installation.
Automatic checks and verified downloads do not silently replace an app.

Apache License 2.0. See LICENSE, NOTICE, and THIRD-PARTY-NOTICES.txt.
