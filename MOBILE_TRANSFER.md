# Lab Manager mobile transfer v1

The Saved inventory page displays a QR snapshot beside Backup & restore.
The planned mobile app must implement this protocol; ordinary camera apps cannot import it.

Each QR holds ASCII LMQR1:<transfer-id>:<part>:<count>:<chunk>.
- Transfer ID: 16 random bytes, base64url without padding.
- Part/count: one-based decimal integers. Collect every unique part of the same ID/count.
- Concatenate chunks in part order, decode base64url without padding, then decompress zlib.
- Parse JSON: format = labmanager.mobile, version = 1, username, avatar (0-4), backup.
- backup is the existing version-1 backup: devices, monitoring settings and imported icons.
- Show a device/settings preview and require import confirmation in the mobile app.
- Validate before importing; use Rust mobile_transfer::decode as a reference.
- Limits: 188 QR pages; 800 characters per page; 32 MiB decompressed JSON. Reject duplicates/mixed IDs, unknown versions and corrupt data.

Password records, private keys, local identity-file paths, login secrets and profile photos are excluded.
The QR is readable by anyone with camera access. No server, cloud upload, firewall change or mobile pairing service is used.
Refresh after desktop changes. Import is a snapshot, not ongoing synchronization.
