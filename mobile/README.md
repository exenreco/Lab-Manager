# Lab Manager mobile alpha

Android and iOS companion apps with the desktop's dark gray/green styling, responsive phone/tablet navigation, SVG device icons, grouped list/grid views, account-scoped devices and notes, OS-aware reviewed commands, and embedded SSH terminals.

## Included

- Encrypted local accounts and inventories. AES-256-GCM protects the vault; its random key stays in Android secure storage / iOS Keychain. Password and recovery-answer verifiers use salted PBKDF2-HMAC-SHA256 (210,000 iterations). Two security questions support local password reset. Automatic sign-in is optional.
- Devices: add one/many IPs, edit OS/icon/ports/SSH account, groups/tags/notes, search, pagination, browser links and removal confirmation.
- Foreground reachability monitoring: TCP web/SSH checks, online/offline badges, sampled availability and bounded check-duration history. Checks pause in the background. These are service reachability checks, not authenticated HTTP health or continuous uptime.
- Private IPv4 subnet discovery checks ports 22/80/443/8006; the user must select discovered devices before saving.
- Desktop `LMQR1` QR transfer: multipart collection, bounded decompression, validation and confirmation before import. JSON backups can also be imported/exported. Imports are snapshots, not live sync. Credentials and desktop account files never transfer.
- SSH password or private-key authentication, optional encrypted credential saving, first-use host fingerprint confirmation and changed-key blocking. ANSI/VT terminal rendering, multiple tabs, copy/paste, arrow/Tab/Escape/Ctrl+C/Ctrl+X touch controls. Tabs close on sign-out. Finish commands before backgrounding the app; mobile OSes may suspend network sessions.

## Platform limits

This first mobile alpha does not provide a local Android/iOS system shell, unattended desktop host-metric collectors, iDRAC hardware history, desktop command history/undo, or app-store/self-update distribution. Desktop monitoring settings and reminders remain in imported device records but do not schedule background mobile work. Current terminals disconnect when the app is backgrounded; reconnect when returning. Imported icons are supported; new icon/photo import and local reminder notifications are not included yet. Five built-in profile avatars are available.

## Build

Use Flutter **3.47.4** / Dart 3.13.3. `flutter pub get`, `flutter analyze`, `flutter test`.

- Android: `flutter build apk --release`. Minimum SDK 23, target SDK 36. The initial APK uses Flutter's development signing configuration and is a **test build**, not a production/store release. Its CI-generated development key is not a persistent release identity; a later test build may require uninstalling the old one. Export backups first. Configure a persistent private signing key before public distribution.
- iOS 15+: on a Mac, `flutter build ios --release --no-codesign`, then `flutter build ios --simulator --debug --no-codesign`. The first produces an **unsigned device app** requiring Apple Developer signing/provisioning before physical-device installation. The simulator app is a separate build; it cannot be installed on an iPhone. Open `ios/Runner.xcworkspace` to configure a team when signing credentials are available.

The **Android and iOS builds** GitHub Actions workflow creates a verified test-signed APK and unsigned/simulator app ZIPs, with SHA-256 checksums. It does not publish to Google Play, TestFlight or the App Store.

Android backup/restore of app-private data is disabled because encrypted files without their hardware-protected vault key are not portable. Back up devices using the in-app export. Key loss does not cause existing vault files to be overwritten.

Apache License 2.0. Source and icon license texts are bundled in assets. Flutter includes third-party package notices in its license registry.
