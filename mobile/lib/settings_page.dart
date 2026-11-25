import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import 'device.dart';
import 'health.dart';
import 'transfer.dart';
import 'ui.dart';
import 'vault.dart';

class SettingsPage extends StatelessWidget {
  final LocalVault vault;
  final HealthMonitor health;
  final Future<void> Function() importQr;
  final void Function(String) notice;
  const SettingsPage({
    super.key,
    required this.vault,
    required this.health,
    required this.importQr,
    required this.notice,
  });
  Widget panel(List<Widget> children) => Padding(
    padding: const EdgeInsets.only(bottom: 16),
    child: SizedBox(
      width: double.infinity,
      child: Card(
        child: Padding(
          padding: const EdgeInsets.all(20),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: children,
          ),
        ),
      ),
    ),
  );
  Future<void> export(BuildContext context) async {
    try {
      if (!await confirm(
        context,
        'Export readable backup?',
        'The file contains device addresses, notes, icons and monitoring configuration. Keep it private. Passwords and private keys are excluded.',
        action: 'Export backup',
      )) {
        return;
      }
      final bytes = Uint8List.fromList(
        utf8.encode(jsonEncode(vault.backup().toJson())),
      );
      await FilePicker.saveFile(
        dialogTitle: 'Export Lab Manager backup',
        fileName: 'lab-manager-backup.json',
        mimeType: 'application/json',
        bytes: bytes,
      );
    } catch (error) {
      if (context.mounted) message(context, error);
    }
  }

  Future<void> restore(BuildContext context) async {
    try {
      final picked = await FilePicker.pickFile(
        type: FileType.custom,
        allowedExtensions: ['json'],
      );
      if (picked == null) return;
      final bytes = BytesBuilder(copy: false);
      await for (final chunk in picked.readAsByteStream()) {
        if (bytes.length + chunk.length > maxTransferBytes) {
          throw const FormatException('Backup exceeds 32 MiB.');
        }
        bytes.add(chunk);
      }
      final backup = BackupData.parse(
        jsonDecode(utf8.decode(bytes.takeBytes())),
      );
      if (context.mounted &&
          await confirm(
            context,
            'Import ${backup.devices.length} device(s)?',
            'Matching IP addresses will be updated. Desktop passwords and account records are never imported.',
            action: 'Import backup',
          )) {
        await vault.importBackup(backup);
        notice('Restored ${backup.devices.length} device(s).');
        unawaited(health.refresh());
      }
    } catch (error) {
      if (context.mounted) message(context, error);
    }
  }

  @override
  Widget build(BuildContext context) => ListView(
    padding: const EdgeInsets.all(16),
    children: [
      Text('Settings', style: Theme.of(context).textTheme.headlineSmall),
      const SizedBox(height: 20),
      panel([
        const Text(
          'Mobile app',
          style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
        ),
        const SizedBox(height: 10),
        const Text('Lab Manager 0.1.0 · First mobile alpha'),
        const SizedBox(height: 8),
        const Text(
          'Mobile builds are installed manually. App-store publication and background self-updates are not configured.',
          style: TextStyle(color: Colors.grey, fontSize: 12),
        ),
        const SizedBox(height: 12),
        OutlinedButton(
          onPressed: () => launchUrl(
            Uri.parse('https://github.com/exenreco/Lab-Manager'),
            mode: LaunchMode.externalApplication,
          ),
          child: const Text('View project on GitHub'),
        ),
      ]),
      panel([
        const Text(
          'Backup & restore',
          style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
        ),
        const SizedBox(height: 10),
        const Text(
          'Backups contain readable addresses, notes, icons and monitoring settings. Passwords, keys and local accounts are excluded.',
          style: TextStyle(color: Colors.grey, fontSize: 12),
        ),
        const SizedBox(height: 16),
        Wrap(
          spacing: 10,
          runSpacing: 10,
          children: [
            OutlinedButton.icon(
              onPressed: () => export(context),
              icon: const Glyph('download'),
              label: const Text('Export backup'),
            ),
            OutlinedButton.icon(
              onPressed: () => restore(context),
              icon: const Glyph('upload'),
              label: const Text('Restore backup'),
            ),
          ],
        ),
        const SizedBox(height: 22),
        const Text('Transfer from desktop'),
        const SizedBox(height: 8),
        const Text(
          'Scan every page from Desktop Settings → Transfer to mobile. This imports a snapshot; it does not continuously synchronize.',
          style: TextStyle(color: Colors.grey, fontSize: 12),
        ),
        const SizedBox(height: 12),
        FilledButton.icon(
          onPressed: importQr,
          icon: const Glyph('radar', color: navigation),
          label: const Text('Scan desktop QR'),
        ),
      ]),
      panel([
        const Text(
          'Security',
          style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
        ),
        const SizedBox(height: 10),
        const Text(
          'Data uses authenticated encryption. The vault key is held in Android secure storage or iOS Keychain. Host fingerprints are verified before SSH authentication.',
          style: TextStyle(color: Colors.grey, fontSize: 12),
        ),
        const SizedBox(height: 12),
        OutlinedButton(
          onPressed: () async {
            if (await confirm(
              context,
              'Forget saved SSH access?',
              'This forgets this mobile account’s passwords, keys and trusted fingerprints. Verify fingerprints before trusting devices again.',
              action: 'Forget SSH access',
            )) {
              await vault.clearSsh();
              if (context.mounted) {
                message(context, 'Saved SSH access removed.');
              }
            }
          },
          child: const Text('Forget saved SSH access'),
        ),
      ]),
      panel([
        const Text(
          'About & licenses',
          style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
        ),
        const SizedBox(height: 10),
        const Text(
          'Apache License 2.0. This alpha supports devices, foreground reachability checks, QR transfer and SSH. Mobile system shells, unattended host collectors and desktop command history are not included.',
          style: TextStyle(color: Colors.grey, fontSize: 12),
        ),
        const SizedBox(height: 12),
        TextButton(
          onPressed: () => showLicensePage(
            context: context,
            applicationName: 'Lab Manager',
            applicationVersion: '0.1.0 alpha',
            applicationLegalese:
                'Copyright 2026 Exenreco Bell · Apache License 2.0',
          ),
          child: const Text('Open source licenses'),
        ),
      ]),
    ],
  );
}

class CommandPanel extends StatefulWidget {
  final Device device;
  const CommandPanel(this.device, {super.key});
  @override
  State<CommandPanel> createState() => _CommandPanelState();
}

class _CommandPanelState extends State<CommandPanel> {
  late final TextEditingController command = TextEditingController(
    text: widget.device.updateCommand,
  );
  @override
  void dispose() {
    command.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => SingleChildScrollView(
    child: Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                'Update / command',
                style: Theme.of(context).textTheme.titleLarge,
              ),
            ),
            IconButton(
              onPressed: () => Navigator.pop(context),
              tooltip: 'Close',
              icon: const Glyph('x'),
            ),
          ],
        ),
        Text(widget.device.name),
        Align(
          alignment: Alignment.centerLeft,
          child: Chip(
            label: Text(osLabels[widget.device.os] ?? widget.device.os),
          ),
        ),
        const Text(
          'Review this command. Updates change the remote server. Nothing runs until you choose Run.',
          style: TextStyle(color: Colors.grey),
        ),
        const SizedBox(height: 16),
        TextField(
          controller: command,
          minLines: 4,
          maxLines: 8,
          autocorrect: false,
          onChanged: (_) => setState(() {}),
          decoration: const InputDecoration(labelText: 'Remote command'),
        ),
        const SizedBox(height: 16),
        FilledButton(
          onPressed: command.text.trim().isEmpty
              ? null
              : () => Navigator.pop(context, command.text),
          child: const Text('Run in SSH terminal'),
        ),
      ],
    ),
  );
}

class DeviceSearch extends StatefulWidget {
  final List<Device> devices;
  final HealthMonitor health;
  const DeviceSearch(this.devices, this.health, {super.key});
  @override
  State<DeviceSearch> createState() => _DeviceSearchState();
}

class _DeviceSearchState extends State<DeviceSearch> {
  String query = '';
  @override
  Widget build(BuildContext context) {
    final devices = widget.devices
        .where(
          (d) =>
              '${d.name} ${d.ip}'.toLowerCase().contains(query.toLowerCase()),
        )
        .toList();
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Row(
          children: [
            const Expanded(
              child: Text(
                'Connect to a device',
                style: TextStyle(fontSize: 20),
              ),
            ),
            IconButton(
              onPressed: () => Navigator.pop(context),
              tooltip: 'Close',
              icon: const Glyph('x'),
            ),
          ],
        ),
        TextField(
          onChanged: (value) => setState(() => query = value),
          decoration: const InputDecoration(hintText: 'Search name or IP'),
        ),
        const SizedBox(height: 12),
        Flexible(
          child: ListView.builder(
            shrinkWrap: true,
            itemCount: devices.length,
            itemBuilder: (context, index) {
              final device = devices[index];
              return ListTile(
                leading: DeviceIcon(device),
                title: Text(device.name),
                subtitle: Text(device.ip),
                trailing: StatusDot(widget.health.latest[device.ip]?.online),
                onTap: () => Navigator.pop(context, device),
              );
            },
          ),
        ),
      ],
    );
  }
}

class HelpPanel extends StatefulWidget {
  const HelpPanel({super.key});
  @override
  State<HelpPanel> createState() => _HelpPanelState();
}

class _HelpPanelState extends State<HelpPanel> {
  String query = '';
  int page = 0;
  static const entries = [
    (
      'Device settings',
      'Tap a device to edit its connection, icon, OS, group and notes.',
    ),
    (
      'Desktop transfer',
      'Show the mobile QR in desktop Settings, then scan all its pages here.',
    ),
    (
      'Monitoring',
      'Checks run in the foreground. Online means a configured TCP port answered.',
    ),
    (
      'SSH host keys',
      'Compare the first fingerprint with the administrator. Changed keys are blocked.',
    ),
    (
      'Terminal controls',
      'The bottom row has Escape, Tab, arrows, Ctrl+C and Ctrl+X.',
    ),
    (
      'Update / command',
      'Review OS commands before running. iDRAC defaults to read-only information.',
    ),
    (
      'Account recovery',
      'Use your two security answers to reset a forgotten password.',
    ),
    (
      'Backup privacy',
      'Readable JSON backups exclude account passwords and SSH keys.',
    ),
  ];
  @override
  Widget build(BuildContext context) {
    final items = entries
        .where(
          (v) => '${v.$1} ${v.$2}'.toLowerCase().contains(query.toLowerCase()),
        )
        .toList();
    final pages = (items.length / 4).ceil().clamp(1, 10);
    page = page.clamp(0, pages - 1);
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Row(
          children: [
            const Expanded(child: Text('Help', style: TextStyle(fontSize: 20))),
            IconButton(
              onPressed: () => Navigator.pop(context),
              tooltip: 'Close',
              icon: const Glyph('x'),
            ),
          ],
        ),
        TextField(
          onChanged: (value) => setState(() {
            query = value;
            page = 0;
          }),
          decoration: const InputDecoration(hintText: 'Search help'),
        ),
        Flexible(
          child: ListView(
            shrinkWrap: true,
            children: items
                .skip(page * 4)
                .take(4)
                .map(
                  (v) => ListTile(
                    contentPadding: EdgeInsets.zero,
                    title: Text(v.$1),
                    subtitle: Text(v.$2),
                  ),
                )
                .toList(),
          ),
        ),
        Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            IconButton(
              tooltip: 'Previous page',
              onPressed: page > 0 ? () => setState(() => page--) : null,
              icon: const Glyph('chevron-left'),
            ),
            SizedBox(
              width: 64,
              child: Text('${page + 1} / $pages', textAlign: TextAlign.center),
            ),
            IconButton(
              tooltip: 'Next page',
              onPressed: page + 1 < pages ? () => setState(() => page++) : null,
              icon: const Glyph('chevron-right'),
            ),
          ],
        ),
      ],
    );
  }
}
