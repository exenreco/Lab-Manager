import 'dart:async';

import 'package:flutter/material.dart';

import 'device.dart';
import 'health.dart';
import 'ui.dart';
import 'vault.dart';

class DevicesPage extends StatefulWidget {
  final LocalVault vault;
  final HealthMonitor health;
  final Future<void> Function([Device? device]) edit;
  final Future<void> Function(Device, String) action;
  final Future<void> Function() importQr;
  final void Function(String) notice;
  const DevicesPage({
    super.key,
    required this.vault,
    required this.health,
    required this.edit,
    required this.action,
    required this.importQr,
    required this.notice,
  });
  @override
  State<DevicesPage> createState() => _DevicesPageState();
}

class _DevicesPageState extends State<DevicesPage> {
  String _query = '';
  int _filter = 0, _page = 0;
  bool _grid = false;
  Widget actions(Device device) => PopupMenuButton<String>(
    tooltip: 'Device actions',
    icon: const Glyph('ellipsis-vertical'),
    onSelected: (value) => widget.action(device, value),
    itemBuilder: (_) => [
      for (final item in [
        ('browser', 'external-link', 'Open browser'),
        ('ssh', 'terminal', 'SSH shell'),
        ('command', 'refresh-cw', 'Update / command'),
        ('settings', 'settings', 'Settings'),
        ('remove', 'trash-2', 'Remove device'),
      ])
        PopupMenuItem(
          value: item.$1,
          child: Row(
            children: [
              Container(
                padding: const EdgeInsets.all(6),
                decoration: BoxDecoration(
                  border: Border.all(color: line),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Glyph(item.$2, size: 16),
              ),
              const SizedBox(width: 12),
              Expanded(child: Text(item.$3)),
            ],
          ),
        ),
    ],
  );
  Widget tile(Device device) {
    final sample = widget.health.latest[device.ip];
    return Container(
      decoration: BoxDecoration(
        color: navigation,
        border: Border.all(color: line),
        borderRadius: BorderRadius.circular(12),
      ),
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: () => widget.edit(device),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Row(
                children: [
                  DeviceIcon(device, icons: widget.vault.icons, size: 32),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            StatusDot(sample?.online),
                            const SizedBox(width: 7),
                            Expanded(
                              child: Text(
                                device.name,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: const TextStyle(
                                  fontWeight: FontWeight.w600,
                                ),
                              ),
                            ),
                          ],
                        ),
                        const SizedBox(height: 5),
                        InkWell(
                          onTap: () => widget.action(device, 'browser'),
                          child: Text(
                            device.ip,
                            style: const TextStyle(color: green),
                          ),
                        ),
                      ],
                    ),
                  ),
                  actions(device),
                ],
              ),
              if (_grid) ...[
                const SizedBox(height: 16),
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        '${device.data['https'] == true ? 'HTTPS' : 'HTTP'} ${device.webPort}',
                        style: const TextStyle(
                          color: Colors.grey,
                          fontSize: 12,
                        ),
                      ),
                    ),
                    Text(
                      'SSH ${device.sshPort}',
                      style: const TextStyle(color: Colors.grey, fontSize: 12),
                    ),
                  ],
                ),
                if (device.tags.isNotEmpty)
                  Padding(
                    padding: const EdgeInsets.only(top: 10),
                    child: Align(
                      alignment: Alignment.centerLeft,
                      child: Text(
                        device.tags,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(
                          fontSize: 11,
                          color: Colors.grey,
                        ),
                      ),
                    ),
                  ),
              ],
            ],
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final all = widget.vault.devices;
    final filtered =
        all
            .where(
              (d) =>
                  '${d.name} ${d.ip} ${d.group} ${d.tags}'
                      .toLowerCase()
                      .contains(_query.toLowerCase()) &&
                  (_filter == 0 ||
                      (_filter == 1
                          ? widget.health.latest[d.ip]?.online == true
                          : widget.health.latest[d.ip]?.online == false)),
            )
            .toList()
          ..sort((a, b) => a.group.compareTo(b.group));
    final pages = (filtered.length / 10).ceil().clamp(1, 1000);
    _page = _page.clamp(0, pages - 1);
    final groups = <String, List<Device>>{};
    for (final device in filtered.skip(_page * 10).take(10)) {
      groups
          .putIfAbsent(
            device.group.isEmpty ? 'Ungrouped' : device.group,
            () => [],
          )
          .add(device);
    }
    return RefreshIndicator(
      onRefresh: widget.health.refresh,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Text(
            'Your workspace',
            style: Theme.of(context).textTheme.headlineSmall,
          ),
          const SizedBox(height: 6),
          const Text(
            'Devices, connections, and a little peace of mind.',
            style: TextStyle(color: Colors.grey),
          ),
          const SizedBox(height: 20),
          TextField(
            onChanged: (value) => setState(() {
              _query = value;
              _page = 0;
            }),
            decoration: const InputDecoration(
              hintText: 'Search name, IP, group, or tag',
              prefixIcon: Padding(
                padding: EdgeInsets.all(14),
                child: Glyph('search'),
              ),
            ),
          ),
          const SizedBox(height: 12),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              IconButton.outlined(
                tooltip: 'Scan QR',
                onPressed: widget.importQr,
                icon: const Glyph('radar'),
              ),
              IconButton.outlined(
                tooltip: 'Scan network',
                onPressed: () => showPanel<void>(
                  context,
                  DiscoveryPanel(
                    widget.vault,
                    onSaved: (count) {
                      widget.notice('Added $count discovered device(s).');
                      unawaited(widget.health.refresh());
                    },
                  ),
                ),
                icon: const Glyph('network'),
              ),
              IconButton.filled(
                tooltip: 'Add Device',
                onPressed: () => widget.edit(),
                icon: const Glyph('plus', color: navigation),
              ),
              IconButton.outlined(
                tooltip: _grid ? 'Switch to list view' : 'Switch to grid view',
                icon: Glyph(_grid ? 'list' : 'layout-grid'),
                onPressed: () => setState(() => _grid = !_grid),
              ),
            ],
          ),
          const SizedBox(height: 20),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(14),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Wrap(
                    spacing: 8,
                    children: List.generate(
                      3,
                      (i) => ChoiceChip(
                        label: Text(
                          [
                            'All (${all.length})',
                            'Online (${all.where((d) => widget.health.latest[d.ip]?.online == true).length})',
                            'Offline (${all.where((d) => widget.health.latest[d.ip]?.online == false).length})',
                          ][i],
                        ),
                        selected: _filter == i,
                        onSelected: (_) => setState(() {
                          _filter = i;
                          _page = 0;
                        }),
                      ),
                    ),
                  ),
                  if (all.isEmpty)
                    Padding(
                      padding: const EdgeInsets.symmetric(vertical: 40),
                      child: Column(
                        children: [
                          const Glyph('server', size: 48, color: green),
                          const SizedBox(height: 16),
                          const Text('Start with your first device'),
                          const SizedBox(height: 8),
                          const Text(
                            'Add an IP address or scan your desktop transfer QR.',
                            textAlign: TextAlign.center,
                            style: TextStyle(color: Colors.grey),
                          ),
                          const SizedBox(height: 16),
                          FilledButton(
                            onPressed: () => widget.edit(),
                            child: const Text('Add Device'),
                          ),
                        ],
                      ),
                    )
                  else if (filtered.isEmpty)
                    const Padding(
                      padding: EdgeInsets.all(32),
                      child: Text('No devices match your search.'),
                    ),
                  for (final group in groups.entries) ...[
                    Padding(
                      padding: const EdgeInsets.only(top: 20, bottom: 12),
                      child: Text(
                        group.key,
                        style: const TextStyle(
                          color: Colors.grey,
                          fontSize: 12,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                    if (_grid)
                      LayoutBuilder(
                        builder: (context, constraints) {
                          final columns = (constraints.maxWidth / 260)
                              .floor()
                              .clamp(1, 4);
                          return Wrap(
                            spacing: 10,
                            runSpacing: 10,
                            children: group.value
                                .map(
                                  (device) => SizedBox(
                                    width:
                                        ((constraints.maxWidth -
                                                    (columns - 1) * 10) /
                                                columns)
                                            .clamp(0, 380),
                                    child: tile(device),
                                  ),
                                )
                                .toList(),
                          );
                        },
                      )
                    else
                      for (final device in group.value)
                        Padding(
                          padding: const EdgeInsets.only(bottom: 8),
                          child: tile(device),
                        ),
                  ],
                  const SizedBox(height: 16),
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          filtered.isEmpty
                              ? '0 devices'
                              : '${_page * 10 + 1}–${(_page * 10 + 10).clamp(0, filtered.length)} of ${filtered.length}',
                          style: const TextStyle(
                            color: Colors.grey,
                            fontSize: 12,
                          ),
                        ),
                      ),
                      IconButton(
                        tooltip: 'Previous page',
                        onPressed: _page > 0
                            ? () => setState(() => _page--)
                            : null,
                        icon: const Glyph('chevron-left'),
                      ),
                      Text('${_page + 1} / $pages'),
                      IconButton(
                        tooltip: 'Next page',
                        onPressed: _page + 1 < pages
                            ? () => setState(() => _page++)
                            : null,
                        icon: const Glyph('chevron-right'),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 10),
          Row(
            children: [
              Expanded(
                child: Text(
                  '${all.length} saved devices',
                  style: const TextStyle(color: Colors.grey, fontSize: 12),
                ),
              ),
              if (widget.health.busy)
                const SizedBox(
                  width: 12,
                  height: 12,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              const SizedBox(width: 8),
              const Text(
                'Foreground checks',
                style: TextStyle(color: Colors.grey, fontSize: 12),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class DiscoveryPanel extends StatefulWidget {
  final LocalVault vault;
  final void Function(int) onSaved;
  const DiscoveryPanel(this.vault, {super.key, required this.onSaved});
  @override
  State<DiscoveryPanel> createState() => _DiscoveryPanelState();
}

class _DiscoveryPanelState extends State<DiscoveryPanel> {
  final _prefix = TextEditingController(text: '192.168.1'),
      _selected = <String>{};
  List<Device> _found = [];
  bool _running = false, _cancelled = false;
  int _checked = 0;
  String? _error;
  @override
  void dispose() {
    _cancelled = true;
    _prefix.dispose();
    super.dispose();
  }

  Future<void> scan() async {
    setState(() {
      _running = true;
      _cancelled = false;
      _checked = 0;
      _error = null;
      _found = [];
      _selected.clear();
    });
    try {
      final devices = await discover(_prefix.text.trim(), () => _cancelled, (
        value,
      ) {
        if (mounted) setState(() => _checked = value);
      });
      if (mounted) setState(() => _found = devices);
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
    } finally {
      if (mounted) setState(() => _running = false);
    }
  }

  Future<void> save() async {
    try {
      final all = widget.vault.devices, ips = all.map((d) => d.ip).toSet();
      final additions = _found
          .where((d) => _selected.contains(d.ip) && !ips.contains(d.ip))
          .toList();
      await widget.vault.saveDevices([...all, ...additions]);
      widget.onSaved(additions.length);
      if (mounted) Navigator.pop(context);
    } catch (error) {
      if (mounted) message(context, error);
    }
  }

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      Row(
        children: [
          const Expanded(
            child: Text('Scan network', style: TextStyle(fontSize: 20)),
          ),
          IconButton(
            onPressed: () => Navigator.pop(context),
            tooltip: 'Close',
            icon: const Glyph('x'),
          ),
        ],
      ),
      const Text(
        'Only scan a private network you are authorized to administer. Checks TCP ports 22, 80, 443 and 8006. Nothing is configured automatically.',
        style: TextStyle(color: Colors.grey, fontSize: 12),
      ),
      const SizedBox(height: 16),
      TextField(
        controller: _prefix,
        enabled: !_running,
        decoration: const InputDecoration(
          labelText: 'Private IPv4 subnet',
          hintText: '192.168.1',
        ),
      ),
      const SizedBox(height: 12),
      if (_running) ...[
        LinearProgressIndicator(value: _checked / 254),
        const SizedBox(height: 8),
        Text('$_checked / 254 addresses checked'),
        TextButton(
          onPressed: () => setState(() => _cancelled = true),
          child: const Text('Stop scan'),
        ),
      ] else
        OutlinedButton(
          onPressed: scan,
          child: const Text('Start authorized scan'),
        ),
      if (_error != null) Text(_error!),
      Flexible(
        child: ListView(
          shrinkWrap: true,
          children: _found
              .map(
                (d) => CheckboxListTile(
                  value: _selected.contains(d.ip),
                  onChanged: (value) => setState(() {
                    if (value == true) {
                      _selected.add(d.ip);
                    } else {
                      _selected.remove(d.ip);
                    }
                  }),
                  title: Text(d.ip),
                  subtitle: Text(
                    'Web ${d.webPort} · ${d.data['https'] == true ? 'HTTPS' : 'HTTP'}',
                  ),
                  secondary: DeviceIcon(d),
                ),
              )
              .toList(),
        ),
      ),
      if (_found.isNotEmpty)
        FilledButton(
          onPressed: _selected.isEmpty ? null : save,
          child: Text('Add ${_selected.length} selected device(s)'),
        ),
    ],
  );
}
