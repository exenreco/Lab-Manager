import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:url_launcher/url_launcher.dart';

import 'auth_page.dart';
import 'device.dart';
import 'device_editor.dart';
import 'devices_page.dart';
import 'health.dart';
import 'monitoring_page.dart';
import 'scan_page.dart';
import 'settings_page.dart';
import 'terminal_page.dart';
import 'transfer.dart';
import 'ui.dart';
import 'vault.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  LicenseRegistry.addLicense(() async* {
    yield LicenseEntryWithLineBreaks([
      'Lab Manager',
    ], await rootBundle.loadString('assets/LICENSE.txt'));
    for (final name in [
      'LUCIDE-LICENSE',
      'SIMPLE-ICONS-LICENSE',
      'DASHBOARD-ICONS-LICENSE',
    ]) {
      yield LicenseEntryWithLineBreaks([
        'Lab Manager device icons · $name',
      ], await rootBundle.loadString('assets/icons/$name'));
    }
  });
  runApp(const LabManagerMobile());
}

class LabManagerMobile extends StatefulWidget {
  final LocalVault? vault;
  const LabManagerMobile({super.key, this.vault});
  @override
  State<LabManagerMobile> createState() => _LabManagerMobileState();
}

class _LabManagerMobileState extends State<LabManagerMobile> {
  late final LocalVault vault = widget.vault ?? LocalVault();
  String? _error;
  @override
  void initState() {
    super.initState();
    vault.addListener(_changed);
    _load();
  }

  void _changed() {
    if (mounted) setState(() {});
  }

  Future<void> _load() async {
    try {
      await vault.load();
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
    }
  }

  @override
  void dispose() {
    vault.removeListener(_changed);
    if (widget.vault == null) vault.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'Lab Manager',
    debugShowCheckedModeBanner: false,
    theme: appTheme(),
    scrollBehavior: const MaterialScrollBehavior().copyWith(
      overscroll: false,
      physics: const ClampingScrollPhysics(),
    ),
    home: _error != null
        ? Scaffold(
            body: Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Glyph('shield', size: 44),
                    const SizedBox(height: 20),
                    const Text('Your encrypted data was preserved.'),
                    const SizedBox(height: 10),
                    Text(_error!),
                    const SizedBox(height: 20),
                    FilledButton(
                      onPressed: () {
                        setState(() => _error = null);
                        _load();
                      },
                      child: const Text('Retry'),
                    ),
                  ],
                ),
              ),
            ),
          )
        : !vault.ready
        ? const Scaffold(body: Center(child: CircularProgressIndicator()))
        : vault.username == null
        ? AuthPage(vault)
        : Workspace(vault, key: ValueKey(vault.username)),
  );
}

class Notice {
  final String text;
  final DateTime time = DateTime.now();
  Notice(this.text);
}

class Workspace extends StatefulWidget {
  final LocalVault vault;
  const Workspace(this.vault, {super.key});
  @override
  State<Workspace> createState() => _WorkspaceState();
}

class _WorkspaceState extends State<Workspace> with WidgetsBindingObserver {
  late final HealthMonitor health = HealthMonitor(() => widget.vault.devices);
  late final TerminalHub terminals = TerminalHub(widget.vault);
  final notices = ValueNotifier<List<Notice>>([]);
  int _page = 0;
  bool _foreground = true;
  static const titles = ['Devices', 'Monitoring', 'Terminals', 'Settings'];
  static const glyphs = ['server', 'activity', 'terminal', 'settings'];
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    health.start();
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    health.dispose();
    terminals.dispose();
    notices.dispose();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      if (!_foreground) {
        _foreground = true;
        health.setPaused(false);
      }
    } else if (state == AppLifecycleState.paused ||
        state == AppLifecycleState.hidden ||
        state == AppLifecycleState.detached) {
      _foreground = false;
      health.setPaused(true);
      for (final tab in terminals.tabs) {
        unawaited(tab.disconnect());
      }
    }
  }

  void notice(String value) {
    if (mounted) {
      notices.value = [Notice(value), ...notices.value].take(100).toList();
    }
  }

  Future<void> edit([Device? device]) async {
    final saved = await showPanel<bool>(
      context,
      DeviceEditor(vault: widget.vault, device: device),
    );
    if (saved == true) {
      notice(
        device == null ? 'Device/s saved.' : '${device.name} settings saved.',
      );
      unawaited(health.refresh());
    }
  }

  Future<void> importQr() async {
    final imported = await Navigator.push<MobileImport>(
      context,
      MaterialPageRoute(builder: (_) => const ScanPage()),
    );
    if (imported == null) return;
    try {
      await widget.vault.importBackup(imported.backup, avatar: imported.avatar);
      notice('Imported ${imported.backup.devices.length} device(s).');
      unawaited(health.refresh());
    } catch (error) {
      if (mounted) message(context, error);
    }
  }

  Future<void> action(Device device, String action) async {
    try {
      if (action == 'browser' &&
          !await launchUrl(device.url, mode: LaunchMode.externalApplication)) {
        throw StateError('Cannot open browser.');
      }
      if (action == 'settings') await edit(device);
      if (action == 'remove' &&
          mounted &&
          await confirm(
            context,
            'Remove ${device.name}?',
            'This removes the device from this mobile account. It does not change the server.',
            action: 'Remove device',
          )) {
        await widget.vault.saveDevices(
          widget.vault.devices.where((value) => value.ip != device.ip).toList(),
        );
        notice('Removed ${device.name}.');
      }
      if (action == 'ssh' && mounted) {
        setState(() => _page = 2);
        await terminals.open(context, device);
      }
      if (action == 'command' && mounted) {
        final command = await showPanel<String>(context, CommandPanel(device));
        if (command != null && mounted) {
          setState(() => _page = 2);
          await terminals.open(context, device, command: command);
        }
      }
    } catch (error) {
      if (mounted) message(context, error);
      notice(error.toString());
    }
  }

  Future<void> addTerminal() async {
    final device = await showPanel<Device>(
      context,
      DeviceSearch(widget.vault.devices, health),
    );
    if (device != null && mounted) await terminals.open(context, device);
  }

  void profile() {
    showPanel<void>(
      context,
      Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          ListTile(
            contentPadding: EdgeInsets.zero,
            leading: Avatar(widget.vault.avatar),
            title: Text(widget.vault.username ?? ''),
            subtitle: const Text('Signed in as'),
          ),
          ListTile(
            contentPadding: EdgeInsets.zero,
            leading: const Glyph('log-out'),
            title: const Text('Sign out'),
            onTap: () async {
              Navigator.pop(context);
              await widget.vault.signOut();
            },
          ),
          ListTile(
            contentPadding: EdgeInsets.zero,
            leading: const Glyph('trash-2'),
            title: const Text('Remove account'),
            onTap: () async {
              Navigator.pop(context);
              if (await confirm(
                context,
                'Remove this mobile account?',
                'This account’s devices, credentials and trusted host keys will be removed from this device. Remote devices are unchanged.',
                action: 'Remove account',
              )) {
                await widget.vault.removeAccount();
              }
            },
          ),
        ],
      ),
    );
  }

  void notifications() {
    showPanel<void>(
      context,
      ValueListenableBuilder(
        valueListenable: notices,
        builder: (context, values, _) => Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Row(
              children: [
                const Expanded(
                  child: Text('Notifications', style: TextStyle(fontSize: 20)),
                ),
                TextButton(
                  onPressed: () => notices.value = [],
                  child: const Text('Clear all'),
                ),
                IconButton(
                  onPressed: () => Navigator.pop(context),
                  tooltip: 'Close',
                  icon: const Glyph('x'),
                ),
              ],
            ),
            Flexible(
              child: values.isEmpty
                  ? const Padding(
                      padding: EdgeInsets.all(24),
                      child: Text('No notifications yet.'),
                    )
                  : ListView.builder(
                      shrinkWrap: true,
                      itemCount: values.length,
                      itemBuilder: (context, index) => ListTile(
                        leading: const StatusDot(true),
                        title: Text(values[index].text),
                        subtitle: Text(
                          TimeOfDay.fromDateTime(values[index].time)
                              .format(context),
                        ),
                      ),
                    ),
            ),
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
    animation: Listenable.merge([widget.vault, health]),
    builder: (context, _) {
      final wide = MediaQuery.sizeOf(context).width >= 760;
      return Scaffold(
        appBar: AppBar(
          title: Row(
            children: [
              const Glyph('server', color: green),
              const SizedBox(width: 10),
              Text(titles[_page]),
            ],
          ),
          actions: [
            IconButton(
              tooltip: 'Notifications',
              onPressed: notifications,
              icon: const Glyph('bell'),
            ),
            IconButton(
              tooltip: 'Help',
              onPressed: () => showPanel<void>(context, const HelpPanel()),
              icon: const Glyph('circle-help'),
            ),
            Padding(
              padding: const EdgeInsets.only(right: 8),
              child: InkWell(
                onTap: profile,
                borderRadius: BorderRadius.circular(24),
                child: Avatar(widget.vault.avatar),
              ),
            ),
          ],
        ),
        body: Row(
          children: [
            if (wide)
              NavigationRail(
                backgroundColor: navigation,
                selectedIndex: _page,
                onDestinationSelected: (page) => setState(() => _page = page),
                labelType: NavigationRailLabelType.all,
                destinations: List.generate(
                  titles.length,
                  (i) => NavigationRailDestination(
                    icon: Glyph(glyphs[i]),
                    selectedIcon: Glyph(glyphs[i], color: green),
                    label: Text(titles[i]),
                  ),
                ),
              ),
            Expanded(
              child: switch (_page) {
                0 => DevicesPage(
                  vault: widget.vault,
                  health: health,
                  edit: edit,
                  action: action,
                  importQr: importQr,
                  notice: notice,
                ),
                1 => MonitoringPage(
                  vault: widget.vault,
                  health: health,
                  edit: edit,
                  action: action,
                ),
                2 => TerminalPage(hub: terminals, add: addTerminal),
                _ => SettingsPage(
                  vault: widget.vault,
                  health: health,
                  importQr: importQr,
                  notice: notice,
                ),
              },
            ),
          ],
        ),
        bottomNavigationBar: wide
            ? null
            : NavigationBar(
                selectedIndex: _page,
                onDestinationSelected: (page) => setState(() => _page = page),
                destinations: List.generate(
                  titles.length,
                  (i) => NavigationDestination(
                    icon: Glyph(glyphs[i]),
                    selectedIcon: Glyph(glyphs[i], color: green),
                    label: titles[i],
                  ),
                ),
              ),
      );
    },
  );
}
