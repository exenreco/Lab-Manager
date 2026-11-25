Implement within the existing dark workspace, shared collection, right sidebar and frosted dialogs.

1. Per-device groups, tags, notes, links and reminder; searchable/filterable inventory.
2. Monitoring maintenance windows, custom TCP/DNS/systemd/container checks and desktop alerts.
3. Multiple persistent terminal tabs and private per-device reviewed-command history.
4. Validated account backup/restore with preview and an automatic pre-restore backup.
5. Run targeted and full checks; build, restart and inspect the final UI.

Account boundaries remain unchanged. Backups exclude authentication records and SSH passwords.
Command history excludes terminal keystrokes, output and arbitrary custom-command text.

Completed: all five slices. Existing styling and sidebar/modal components retained.
Validation: full suite 80 tests passed (57 library, 22 UI/binary, 1 credential-helper integration); Clippy all targets clean. Responsive filter adjustment rechecked with all 22 UI tests.
Live UI verified: device organization fields, service/maintenance panels, backup controls, successful read-only reviewed SSH command, second SSH tab, hide/resume without disconnect, and explicit tab closure. Server configuration was not changed for these checks.
Desktop notifications are opt-in; their display depends on Windows notification policy. Linux service/container probes require matching SSH permissions and have not been enabled on the user's devices by these checks.
