First Windows x64 prerelease of Lab Manager.

Includes device inventory and organization, network discovery, availability and SSH monitoring, iDRAC hardware monitoring, embedded SSH and local terminals, encrypted local accounts, backup/restore and mobile transfer codes.

### Installation
- Download the `windows-x64-setup.exe` installer. Review and accept the prerelease terms, and optionally enable **Run Lab Manager when I sign in to Windows**.
- Windows 10/11 x64. SSH features require the Windows OpenSSH client.
- The portable ZIP is also available. Keep its `data` folder with the executable when moving it.
- Installed app data stays in `%LOCALAPPDATA%\Programs\Lab Manager\data`. Uninstalling preserves that data. Existing development/portable accounts are not automatically moved to an installed copy; use Backup & restore for devices and register a local account there.

### Signing and verification
This prerelease is Authenticode-signed using a **self-signed** Lab Manager prerelease certificate. It is **not a publicly trusted publisher certificate**. Windows/SmartScreen will show warnings. The public `.cer` certificate and SHA-256 checksums are provided for inspection; no private signing key is distributed. Do not install the certificate into a trusted root store merely to suppress warnings.

### Updates
Settings → Updates checks this repository’s GitHub releases. Alpha builds include newer prereleases; stable builds skip prereleases. Automatic updates are opt-in, check at launch and every six hours, and download verified installers. Installation requires confirmation and closes the app. Keep backups and finish terminal work before installing.

### License and known limits
Apache License 2.0; third-party notices are included. This is an experimental prerelease: back up settings and review administrative commands. iDRAC firmware updates require a package-specific command. The planned mobile scanner app is not included.
