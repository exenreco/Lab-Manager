# Lab Manager

A native Rust desktop server manager for Windows, organized around `Server`, `ServerStore`, `SshService`, and `LabManager` structs and methods.

## Windows prerelease

Download the Windows x64 installer or portable ZIP from [GitHub Releases](https://github.com/exenreco/Lab-Manager/releases). The installer displays prerelease terms and offers an optional Windows startup checkbox. Installed data is preserved on uninstall. The first alpha is self-signed; Windows does not trust its publisher certificate by default. The public certificate and SHA-256 checksums are included with the release.

Settings → **Updates**, above Backup & restore, checks releases from this repository. **Automatic updates** is off by default; when enabled it checks at launch and every six hours while open and downloads newer installers. SHA-256 hashes from GitHub release metadata are required. You explicitly choose when to install and close the app. Prerelease builds receive newer prereleases; stable builds skip them. Updates never pull source code or run Git commands on users’ PCs.

To package a release on Windows, install Inno Setup 6 and Windows SDK signing tools, then run `./scripts/release.ps1`. The script creates or reuses a non-exportable self-signed certificate in the current user's certificate store, timestamps the signed app and installer, bundles third-party license notices, and writes checksums to `dist`. No private key is included in the repository or release. See [release notes](packaging/RELEASE_NOTES.md) for prerequisites and limitations.

## License

Android and iOS companion source is in [`mobile`](mobile/README.md). The **Android and iOS builds** workflow produces a test-signed Android APK, an unsigned iPhone app, and an iOS simulator app. The companion supports encrypted local accounts, desktop QR import, device management, foreground reachability monitoring and embedded SSH. See the mobile README for platform limits and signing requirements.

Apache License 2.0. See [LICENSE](LICENSE), [NOTICE](NOTICE), and the third-party icon licenses in `assets/icons`. Dependency notices are bundled in release packages.

## Linux and macOS builds

The **Linux and macOS builds** GitHub Actions workflow builds and tests Ubuntu 24.04 x86_64, macOS Apple Silicon, and macOS Intel on native runners. It produces a Linux `.deb` and `.tar.gz`, plus architecture-specific macOS `.dmg` files. Build artifacts include checksums and third-party notices. Run `python3 scripts/package_unix.py` after a native release build to reproduce packaging. See [platform requirements](packaging/UNIX_README.txt).

On Linux/macOS, account files use ChaCha20-Poly1305 encryption with a random key stored in Linux Secret Service or macOS Keychain. An unlocked desktop credential vault is required; there is no plaintext fallback. Windows DPAPI accounts do not migrate across platforms. Register a local account and restore a device backup. Data lives in the platform's per-user application data directory rather than inside `/usr/bin` or an application bundle.

The macOS toolbar remains at the top right, with rounded right corners, traffic-light controls, and left-to-right ordering that places Profile nearest the window controls. The DMG applications are ad-hoc signed; Apple Developer ID signing and notarization require an Apple signing identity and are not included.

## Run

Install Rust with the MSVC toolchain and Visual Studio C++ Build Tools, then run from this folder:

```powershell
cargo run
```

Build a desktop executable with `cargo build --release`. The executable is `target/release/lab-manager.exe`.

## Use

**Monitoring**, directly below Devices, checks saved web/SSH ports automatically while the app is open. Select a device to configure a 15–3600 second refresh interval, pause checks, enable notifications, or collect host metrics over SSH. History and settings live in each account's `monitoring.json`, retain the latest 1,440 checks per endpoint, and are cleared from memory on sign-out. Charts show the latest 120 checks; availability is the percentage of recorded checks with a reachable configured port, not continuous uptime.

Host metrics use read-only Linux/Proxmox, Windows PowerShell, or macOS collectors. Enable them and choose the platform on the Monitoring page. Open **SSH Shell** first to verify the host key. Monitoring defaults to keys/SSH agent (`BatchMode=yes`). For password login, enter the SSH password in Monitoring settings and choose **Save password & retry**. Password records use Windows DPAPI encryption in the account's `monitoring-credentials` folder, are bound to the saved IP/SSH port/username, and can be removed using **Remove saved password**. A hidden askpass helper supplies the password through a pipe; it is never placed in command-line arguments or environment variables. `StrictHostKeyChecking=yes` remains enabled in both modes. Monitoring never installs a server agent or changes server configuration. CPU, memory, root/system disk usage, host uptime, and network rates appear only when a collector returns them. Network rates need two successful samples; appliance/iDRAC checks provide service availability unless a compatible shell is available. SSH host collectors are covered by parser tests; actual host compatibility depends on the OS tools and access permissions.

HTTP/HTTPS checks request `/` using HEAD. HTTPS inspection uses Windows SslStream with normal certificate/hostname validation; self-signed, untrusted, expired, or IP/hostname-mismatched certificates report a validation failure. Trusted certificates warn at 30 days. Alerts cover service outages, HTTP errors, SSH collector errors, resource thresholds, and recovery, and are deduplicated until the condition changes. Collectors run on bounded background workers; SSH/HTTPS subprocesses have a 12-second timeout. Closing Lab Manager stops monitoring; no background service is installed.

The interface follows the latest dashboard reference with a compact icon rail, centered content column, rounded connection cards, and expandable management details. It retains the dark gray workspace and darker navigation. Expand the rail to reveal grouped links and the independent active/hover dotted connectors. Home and Servers provide search, browser links, SSH access, and saved connection metadata. Open Manage connection on a card for updates, settings, and removal. Ctrl+1 opens Servers, Ctrl+2 opens Local shell, Ctrl+3 opens Settings, and Ctrl+B toggles the labeled navigation. Shortcuts are paused while typing.

1. Select **+ Add servers** on Devices to open the right sidebar. The shared form and keep your draft when switching modes or closing the sidebar. **One** accepts a name, built-in icon, IP, browser port and protocol, SSH username, and SSH port. **Many** accepts a batch of IPv4/IPv6 addresses, an optional name prefix, and shared connection settings. Invalid batches are rejected; existing addresses are skipped. Saving closes the sidebar and returns to Servers.
2. **Devices** shows each server with an illustrated device image, name, IP, and a browser link with an external-link icon. Filter by name or IP.
3. Open **Settings** to set the device name, icon, SSH account/port, browser port, and HTTP/HTTPS. The default SSH username is `root`; change it to your server's actual account.
4. **SSH shell** opens an embedded terminal in the reusable right sidebar, including the SSH icon on Monitoring. Standard OpenSSH configuration, keys and agent authentication apply. Verify the server fingerprint when connecting for the first time. Saved encrypted monitoring passwords are reused when configured; otherwise authentication prompts appear in the terminal.
5. **Update / command** lets you select a Linux package-manager preset or enter a custom command, review it, then launch it on that server. Commands run through the remote account's shell. Output, passwords, sudo prompts, and package confirmations appear in the terminal. No update runs until you click **Run command in SSH terminal**.
6. **Local shell** automatically opens embedded PowerShell directly on its page and retains the session when switching pages. Signing out disconnects it. SSH terminals use the reusable right sidebar; drag its edge to resize. In either terminal, use the mouse wheel for scrollback, drag to select text, Ctrl+Shift+C to copy, and Ctrl+V to paste. Ctrl+C interrupts a command when no text is selected. Reconnect restarts the session. Disconnect or closing a terminal tab ends its client; hiding/replacing the sidebar retains SSH tabs.

Install the Windows **OpenSSH Client** optional feature if `ssh.exe` is unavailable. The managed server must have SSH enabled and be reachable, directly or through your configured network/VPN. This application does not scan the network or infer an operating system.

## Saved inventory

The exact `servers.json` path is displayed at the bottom of the window. Each account inventory is stored at `data/users/<encoded-username>/servers.json` beside the executable (`target/debug/data` for `cargo run`). Keep the executable in a writable folder. Entries and connection settings are saved immediately. Passwords and private keys are not stored. Back up the JSON file to transfer your inventory, and copy the `data` folder when moving to a release build. Readable inventories from the earlier AppData location are migrated automatically when no portable inventory exists; the original is retained. If loading fails, saving is disabled to protect the existing file; repair it and restart. Run one instance at a time when editing inventory.

The app currently targets Windows. Embedded terminals use ConPTY with ANSI color/cursor rendering and 2,000 lines of scrollback. Device artwork is drawn locally and requires no internet access.

## Verify

```powershell
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Tests cover IP validation and deduplication, IPv6 links, connection input validation, inventory round trips/corruption, and SSH command arguments. Real SSH authentication and remote updates require your own reachable server.

The toolbar includes Home, a pipe divider, and custom minimize, maximize/restore, and close controls. Drag the empty toolbar area to move the window; double-click it to maximize or restore.

The server icon picker includes Server, Database, Computer, Cloud, Gateway, Dell iDRAC, Proxmox, Ubuntu, Windows, macOS, Linux, Debian, Docker, NAS, Router, and Network switch. Icons use local vector artwork and are remembered in the inventory.

## Local account and recovery

The first launch shows local account registration. Additional users can choose Create another local account on the sign-in screen. Usernames are unique without regard to letter case, and each account has separate passwords, security questions, and server inventory. Choose a username, a password of at least 12 characters, and two different security questions with different answers. Subsequent launches require sign-in unless automatic sign-in was selected. The top toolbar shows the signed-in username and sign-out icon beside Home. Signing out clears the in-app session and drafts; separately launched SSH terminals continue running.

Account metadata is stored beside the inventory as `data/account.enc`, encrypted with Windows DPAPI for the current Windows user. Passwords and recovery answers are independently salted and hashed with Argon2id before encryption. Plaintext passwords and answers are never written to disk. The file cannot normally be decrypted under another Windows user or on a different computer. Back it up along with the inventory; Windows profile/key backups are also needed to preserve DPAPI access after reinstalling Windows.

Select **Forgot your password?** to answer both registration questions and set a new password. Answers ignore letter case and extra whitespace. Five failed sign-ins trigger a one-minute delay persisted in the encrypted account record. Recovery has a separate limit, allowing password reset even during a sign-in delay. A successful recovery clears the delays. If the encrypted file cannot be read, the app blocks account creation rather than silently replacing it.

This is a local application lock. Server inventory remains in `servers.json`; this request encrypts account information, not server inventory. It does not protect against an administrator or a person/process with full access to the same Windows account and application files. Recovery questions reset the application password; they cannot recover lost Windows DPAPI keys. All account operations stay offline.

Registration and sign-in offer an optional **Automatically sign in on this computer** checkbox, off by default. It stores a random token in DPAPI-encrypted `session.enc` and an Argon2id token hash in the encrypted account record, never a saved password. The token expires after 30 days. Signing out, resetting the password, or signing in without selecting this option revokes automatic access. A damaged or expired token falls back to the sign-in screen.

## Account isolation and migration

Inventories are loaded only after authentication and are selected from the authenticated account identity, never from a user-supplied folder. Accounts cannot view, edit, remove, connect to, or run commands against another account's entries through the application. Sign-out clears the loaded inventory, dialogs, drafts, searches, and discovery results, and cancels an active scan. The original single-account encrypted record remains readable and upgrades when saved. The pre-existing shared inventory is copied only to the original owner's account inventory on first sign-in; its original file is retained as a backup. New accounts start empty. Only the most recently signed-in account can be remembered for automatic sign-in on this installation.

## Network discovery

Select Scan network beside Add servers to open the right sidebar. Confirm the suggested IPv4 /24 range or enter your own private, link-local, or loopback CIDR from /22 to /32 (at most 1024 addresses). Scanning checks TCP ports 22, 80, 443, 8006, 8080, and 8443 with bounded concurrency and timeouts. It sends no application payloads, attempts no logins, and makes no changes to devices. Stop scan or close the sidebar to cancel.

Review the results and select devices before choosing Configure selected devices. This opens an unsaved One/Many form; check names, ports, protocol, and SSH settings, then explicitly Save to add the devices to your own inventory. Already-saved IPs are marked and cannot be reselected. Multiple devices use shared connection settings, so review them before saving. Discovery is service-based, not a complete network inventory: firewalls, sleeping devices, IPv6-only devices, or devices without an open checked port may not appear. No scan runs at startup.

## Notifications and dashboard controls

Home and Servers place search, Scan network, and Add servers together in one action row. Saved-server and HTTPS-endpoint counts are shown in the bottom toolbar. The notification bell beside Home opens a dropdown with newest activity first, a full local timestamp and relative time for each item, an unread indicator, and Clear all. Opening the panel marks existing items as read. Clearing affects only that account's notification history, not its servers.

Notifications are saved as `notifications.json` beside each account's own inventory and restored after restart. Repeated actions remain separate events. Server additions/removals, settings changes, terminal launches/errors, and discovery activity are recorded; terminal output, passwords, and recovery answers are not logged. Signing out closes and clears the in-memory panel; the next account loads its own history.

## Responsive views and connection status

Home and Servers align to the left and use the available workspace width. List/Grid beside Add servers switches between a compact grouped table and a responsive grid (one to four columns, depending on space). Search and all connection/management actions work in both views; narrow action rows wrap to fit.

Hover over the compact left rail to reveal labeled navigation. Moving into the expanded sidebar keeps labels visible; moving outside collapses it. The sidebar toggle/Ctrl+B can still pin or unpin the expanded menu.

A small circle beside each server name shows a background TCP reachability check: green if either the configured browser port or SSH port accepts a connection, red if neither responds, and gray before the first result. Hover the dot for details and last-check age. Checks run roughly every 30 seconds, use bounded worker threads and timeouts, send no application payloads, and are scoped to the signed-in account's configured servers. A red result means unreachable on those ports, not proof that the device is powered off. Firewalls and network policy can affect results. Signing out cancels pending checks and clears their cached results.

## Compact table view

List view follows the supplied table reference: a dark framed table, compact rows, collapsible connection-status groups with counts, small IP badges, and right-aligned metadata. All devices/Online/Offline tabs filter the table alongside the name/IP search. Browser and SSH ports appear as `browser / SSH` with a tooltip. The leading vector thumbnail uses the server's selected device icon. Click a name to open Device settings in the right sidebar, click its IP to open the browser, or use the row's three-dot menu for browser, SSH, updates, settings, and removal. Columns resize with the window. Device icons are the first column. Secondary columns fold away at narrower widths; when the IP column no longer fits, the IP appears below the server name. Full connection details remain in name tooltips and row actions. Pagination offers Previous/Next, page numbers, result ranges, and 5/10/25/50/100 rows per page. Changing search or status filters resets the page; shrinking results clamps the page to a valid range. Grid view is unchanged.

Network discovery, Device settings, and Add servers use the same exclusive right sidebar. Closing settings discards that edit draft; saving persists changes and closes the panel. The Actions column always keeps its label and reserved width, including when the sidebar narrows the workspace. Pagination aligns to the right; Previous/Next use flat chevron controls, and Rows per page is vertically centered with its selector. On narrow tables the pagination controls use two right-aligned rows.

The table has a dedicated Status column after the leading device icon. All row cells use vertical centering; compact rows retain the IP below the name when needed. Selecting any row updates the shared Device settings sidebar immediately, while reselecting the same device preserves its current edit draft. More buttons use a compact dark background, a thin border, and minimal padding. The action button and device right-click menus share the same icon-labeled actions; right-click is available in both list and grid views. The table footer places Rows per page, its selector, and the result range on the left, with Previous/page numbers/Next on the right; footer labels share a fixed-height vertical center.

Device actions use the same vertical three-dot menu in list and grid views. Right-click and action menus keep the selected device outlined in green while open. Grid cards show the browser URL below the IP and use the shared menu for browser, SSH, commands, settings, and removal. Command review and removal confirmation open in the reusable right sidebar; removing a device still requires explicit confirmation there. The Remove server action uses a drawn trash-can icon.

Grid cards are capped at 360 logical pixels and the grid at four columns, while still shrinking to fit smaller windows and sidebars. The top toolbar centers the workspace breadcrumb, keeps the navigation toggle in the left rail, and provides Undo/Redo before notifications. Inventory history supports the last 50 saved changes for the current signed-in session (add, edit, remove). Undo/Redo saves the restored inventory; save failures leave history intact. A new change clears redo, and switching accounts or signing out clears history. Use Ctrl+Z to undo, Ctrl+Y or Ctrl+Shift+Z to redo when no text field has focus. Remote shell commands are not part of inventory history.

Click a grid card to open Device settings; the selected card stays outlined in green while its settings sidebar is open. Browser-IP links and action menus retain their own clicks. The centered workspace breadcrumb is painted behind the title-bar drag surface, without a selectable label or hit target. Toolbar order is sign-out, divider, notifications, divider, Undo, Redo, then the window controls.

The title-bar breadcrumb is left aligned and remains behind the drag surface. Grid and list views share collapsible Online, Offline / unreachable, and Checking groups. Drag only a device's name in grid or list view to reorder it within its current status group; the name displays Grab on hover and Grabbing while dragging, and a green line marks the insertion position. IP links and action buttons keep their own interactions. Dropping saves the inventory order shared by both views, and Undo/Redo can restore it. Filtering does not discard or reorder other hidden devices.

The shared device icon picker shows a preview and Choose icon button in Add server and Device settings forms. Its frosted-glass modal contains a searchable Icons tab with all 16 built-in device icons and an Import tab for your own PNG images. Imports use the native Windows file chooser, preserve transparency, accept files up to 5 MB and 4096 by 4096 pixels, and store a normalized preview (up to 256 by 256) inside the current user's icons directory beside their inventory. Imported icons remain available after restart and display in both grid and list views. Choosing an icon updates the draft; Save server or Save settings commits the device change. The blurred backdrop stays in memory and is released when the modal closes.

Dashboard Scan/Add/view actions now share a dark capsule with compact icon controls, active and hover states, and responsive icon-only actions in narrow layouts. A single Grid/List button morphs its icon and slides its label over 450 ms while switching views immediately. The title-bar actions and minimize/maximize/close controls share a second capsule with rounded outer end caps; breadcrumb dragging and the notification dropdown remain functional. Design references: https://collectui.com/designs/navigation-ui-design-inspiration/4fae0173-2eac-4018-ba12-c48de1f8676c and https://codepen.io/aaroniker/pen/dyoKeMP.

The top toolbar now places a round profile portrait and Signed in as text beside the Redo divider. Hovering opens the account dropdown; clicking pins it open. Its Log out action has a rounded bordered icon and revokes automatic sign-in through the existing logout flow. Registration offers five local illustrated avatars (Avatar 1 by default) and a native PNG/JPEG photo chooser. Photos are limited to 5 MB and 4096 by 4096 pixels, center-cropped to a 256 by 256 PNG thumbnail, and saved inside the Windows-encrypted account record rather than as a photo path. Existing accounts without profile data automatically use Avatar 1. Profile data remains separate for each local account, including after automatic sign-in or password recovery.

Grid and list now render through one device collection container with shared All devices/Online/Offline filters, connection groups, page size, range, and pagination. The profile trigger shows only the portrait; Signed in as appears inside its dropdown, followed by Log out and a trash-can Remove account action. Account removal opens the same reusable frosted modal as the icon picker. Both modals blur and block only the area below the 44-pixel title bar, leaving window controls and dragging usable. Removing an account requires the final confirmation button: its sign-in/profile are removed, remembered tokens revoked, and per-user device data moved into a random folder under data/removed-accounts. Data is rolled back if saving account changes fails; re-registering a removed username starts with a fresh inventory, and legacy inventory ownership never transfers to another account.

The title bar matches the 44-pixel window-control capsule, with the breadcrumb centered. Search and device actions sit at opposite edges and wrap on narrow windows. The icon picker uses a borderless gallery beside a dedicated import-settings gutter, with system and imported icon tabs.

UniFi SSH metrics: select **UniFi OS / AP / Switch** under Monitoring settings after enabling **Collect host metrics over SSH**. The read-only collector reads CPU, memory, root disk, uptime, and aggregate interface counters from Linux proc files, with a memory fallback for older BusyBox firmware. Use the console SSH credentials for UniFi OS consoles or the configured device SSH credentials for APs/switches. SSH must already be enabled; unsupported metrics remain unavailable rather than displaying zero. UniFi firmware collectors are parser-tested; device compatibility still requires a successful SSH login and access to the proc files.

## Device organization and maintenance

Device settings and Add servers → **Organization & notes** provides a group, comma-separated tags, notes, HTTP/HTTPS links and a reminder due in one hour, tomorrow or seven days. Save with Done or Save server(s); Many applies organization fields to every new device. Search includes group, tags and notes; the collection filters by group/tag and can group both list and grid views by status or named group. Reminders fire once while Lab Manager is open, including overdue reminders after restarting.

Monitoring settings → **Maintenance & alerts** can silence a device's alerts for 1, 4 or 24 hours while checks/history continue. Alerts resume when the window expires; End maintenance resumes immediately. CPU/memory/disk thresholds and outage/recovery alerts remain per-device. Optional Windows desktop notifications use the Windows tray notification API through hidden PowerShell; Windows notification policy/Focus Assist may suppress them. Alerts are deduplicated; desktop delivery is bounded and may drop a burst rather than block the application.

**Service checks** supports up to eight per device: custom TCP ports (including Proxmox 8006), DNS resolution through the device (Pi-hole 53), `systemctl is-active` services and Docker running state over SSH. DNS queries default to example.com and can be edited. Linux service/container names are validated before entering a fixed read-only command. SSH checks use the selected monitoring authentication and require corresponding read permissions. No agents are installed and no services are modified. Availability badges and failures appear in Monitoring. The existing offline policy skips probes for devices marked unreachable on both configured web/SSH ports; the lightweight connection monitor detects recovery.

## Terminal tabs and reviewed-command history

SSH opens a device tab in the shared terminal sidebar. The terminal **+** button opens device search in the shared Choose Icon frosted modal, constrained to the right sidebar, with a search field and All devices/Online/Offline results. Search by device name, IP, group, tags or notes; use arrows and Enter, or click a result, to open an additional SSH session. Escape, outside click or the close icon dismisses the search; up to eight tabs can remain connected. Switching sidebar content or closing the sidebar hides terminals without disconnecting them. **Terminals (n)** in Devices restores the tabs. Disconnect stops the selected client; its × closes the tab. Sign-out, restore and app exit end sessions. Local PowerShell remains embedded on its own page.

Update / command → **Command history** shows up to 30 recent reviewed runs per endpoint; the account retains 500 entries in commands.json. Known update presets are recorded by name/command; custom command text is omitted. Keystrokes, shell output, passwords and interactive shell history are not captured by Lab Manager. SSH client exit status is recorded when available; disconnects/restarts are explicitly marked as remote completion unknown. An interrupted previous session is marked unknown at next sign-in. Remote background jobs may outlive an SSH client.

## Account backup and restore

Saved inventory → **Backup & restore** exports a versioned JSON bundle containing devices, metadata/reminders, imported icons and monitoring settings. Authentication records, security answers, SSH passwords/credential references, profile photos, command history and monitoring samples are excluded. Backups are readable and may contain private device notes; store them privately.

Restore validates version, limits, endpoints, service names and icon data, then shows a preview. Default merge adds new IPs and replaces settings for matching IPs. Replace inventory is an explicit option in the preview. A recovery bundle is written to the signed-in account's backups directory before any inventory change. Restores are confined to that account; imported monitoring configurations use key authentication until a password is configured again. Imported endpoints start a fresh monitoring history. Icon-name collisions are assigned new identifiers. Inventory writes roll back if saving monitoring settings fails.

### SSH reliability

Lab Manager reuses RSA host identities already trusted in the current Windows user's PuTTY host-key cache. The cached public key is converted into an account-local OpenSSH known-hosts file; unknown keys are never fetched or accepted automatically, and strict checking stays enabled for saved-password/background connections. Standard OpenSSH known_hosts records remain in use.

Password mode skips public-key and GSSAPI attempts. Interactive terminals offer **Log in manually** to verify a new host or enter a password without deleting saved credentials. Passphrase-protected private keys still need an unlocked agent for unattended metrics.

Opening an existing failed SSH tab retries it using the current device/authentication settings. Healthy tabs remain connected. Remote update commands are never automatically replayed. ConPTY output is drained before process startup to prevent startup backpressure; startup can be cancelled from Disconnect.

### Dell iDRAC monitoring

Select Dell iDRAC / RACADM in Monitoring settings and enable Collect over SSH. Existing devices using the Dell iDRAC icon are detected once; later platform choices are preserved. The collector reuses the encrypted SSH password or key configuration and strict host verification. It only runs read-only getsensorinfo, getsysinfo and storage get queries. Collection runs at least 60 seconds apart.

The dashboard shows reported hardware health, server power state, temperature/fan/power history, supply readings, controllers, physical disks and virtual disks. Unknown or unavailable readings stay unknown, including sensors unavailable while the server is off. Optional query failures are displayed as partial-data notes. Hardware warnings and failures feed the existing notification and maintenance rules.

Only the latest check retains detailed sensor/storage inventories; older checks retain summary readings and hardware issues for charts and alerts. This is RACADM integration; a Redfish client is not required or included.

### Device OS and reviewed commands

Device settings and Add Device include an OS selector. Automatic follows the selected device icon; a manual OS choice survives later icon changes. Update / command displays the resolved OS as a badge and starts with its appropriate preset. Linux distribution presets omit sudo for the root account. Unknown distributions, Windows and appliances require an explicit custom command. iDRAC starts with read-only racadm getsysinfo; firmware packages and update commands must be reviewed separately. macOS starts with softwareupdate --list. No command runs until Run command in SSH terminal is clicked.

Command sessions use the same tab strip, terminal controls and embedded terminal as SSH shell sessions, with an OS badge and no Add Terminal button while the command tab is selected. Ordinary SSH tabs retain device search and Add Terminal.
