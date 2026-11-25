import 'package:flutter/material.dart';

import 'device.dart';
import 'health.dart';
import 'ui.dart';
import 'vault.dart';

class MonitoringPage extends StatefulWidget {
  final LocalVault vault;
  final HealthMonitor health;
  final Future<void> Function([Device? device]) edit;
  final Future<void> Function(Device, String) action;
  const MonitoringPage({
    super.key,
    required this.vault,
    required this.health,
    required this.edit,
    required this.action,
  });
  @override
  State<MonitoringPage> createState() => _MonitoringPageState();
}

class _MonitoringPageState extends State<MonitoringPage> {
  String? _selected;
  @override
  Widget build(BuildContext context) {
    final devices = widget.vault.devices, health = widget.health;
    if (devices.isEmpty) {
      return const Center(child: Text('Add a device to start monitoring.'));
    }
    final device =
        devices.where((d) => d.ip == _selected).firstOrNull ?? devices.first;
    final sample = health.latest[device.ip],
        history = health.history[device.ip] ?? [];
    final online = devices
            .where((d) => health.latest[d.ip]?.online == true)
            .length,
        offline = devices
            .where((d) => health.latest[d.ip]?.online == false)
            .length;
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                'Monitoring',
                style: Theme.of(context).textTheme.headlineSmall,
              ),
            ),
            Switch(
              value: !health.paused,
              onChanged: (value) => health.setPaused(!value),
              activeTrackColor: green,
            ),
          ],
        ),
        const Text(
          'Device health and service availability while the app is open.',
          style: TextStyle(color: Colors.grey),
        ),
        const SizedBox(height: 16),
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            for (final value in [
              ('Online', online, green),
              ('Offline', offline, const Color(0xffe66e6e)),
              ('Not checked', devices.length - online - offline, Colors.grey),
            ])
              SizedBox(
                width: 150,
                child: Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(value.$1, style: TextStyle(color: value.$3)),
                        const SizedBox(height: 8),
                        Text(
                          '${value.$2}',
                          style: Theme.of(context).textTheme.headlineMedium,
                        ),
                      ],
                    ),
                  ),
                ),
              ),
          ],
        ),
        const SizedBox(height: 20),
        Opacity(
          opacity: health.paused ? 0.45 : 1.0,
          child: IgnorePointer(
            ignoring: health.paused,
            child: Card(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    DropdownButtonFormField<String>(
                      key: ValueKey(device.ip),
                      initialValue: device.ip,
                      isExpanded: true,
                      decoration: const InputDecoration(labelText: 'Device'),
                      items: devices
                          .map(
                            (d) => DropdownMenuItem(
                              value: d.ip,
                              child: Row(
                                children: [
                                  StatusDot(health.latest[d.ip]?.online),
                                  const SizedBox(width: 10),
                                  DeviceIcon(
                                    d,
                                    size: 22,
                                    icons: widget.vault.icons,
                                  ),
                                  const SizedBox(width: 10),
                                  Expanded(
                                    child: Text(
                                      '${d.name}\n${d.ip}',
                                      style: const TextStyle(fontSize: 12),
                                    ),
                                  ),
                                ],
                              ),
                            ),
                          )
                          .toList(),
                      onChanged: (ip) => setState(() => _selected = ip),
                    ),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.end,
                      children: [
                        IconButton(
                          tooltip: 'Refresh checks',
                          onPressed: health.busy ? null : health.refresh,
                          icon: const Glyph('refresh-cw'),
                        ),
                        IconButton(
                          tooltip: 'SSH shell',
                          onPressed: () => widget.action(device, 'ssh'),
                          icon: const Glyph('terminal'),
                        ),
                        IconButton(
                          tooltip: 'Device settings',
                          onPressed: () => widget.edit(device),
                          icon: const Glyph('settings'),
                        ),
                      ],
                    ),
                    Text(
                      sample == null
                          ? 'Not checked yet'
                          : 'Device is ${sample.online ? 'online' : 'offline'} — last checked ${TimeOfDay.fromDateTime(sample.checked).format(context)}',
                      style: const TextStyle(color: Colors.grey, fontSize: 12),
                    ),
                    const SizedBox(height: 12),
                    Wrap(
                      spacing: 8,
                      runSpacing: 8,
                      children: [
                        Chip(
                          label: Text(
                            'Web ${device.webPort}: ${sample == null
                                ? '—'
                                : sample.web
                                ? 'Available'
                                : 'Unavailable'}',
                          ),
                        ),
                        Chip(
                          label: Text(
                            'SSH ${device.sshPort}: ${sample == null
                                ? '—'
                                : sample.ssh
                                ? 'Available'
                                : 'Unavailable'}',
                          ),
                        ),
                        Chip(
                          label: Text(
                            'Availability ${history.isEmpty ? '—' : '${(100 * history.where((v) => v.online).length / history.length).toStringAsFixed(1)}%'}',
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 18),
                    const Text('Check duration history'),
                    const SizedBox(height: 12),
                    SizedBox(
                      height: 160,
                      child: CustomPaint(
                        painter: HistoryPainter(List.of(history)),
                        child: const SizedBox.expand(),
                      ),
                    ),
                    const SizedBox(height: 10),
                    Text(
                      '${history.length} foreground samples · ${sample?.latency ?? 0} ms for last web/SSH checks',
                      style: const TextStyle(color: Colors.grey, fontSize: 12),
                    ),
                    const SizedBox(height: 20),
                    const Text('Host metrics and hardware diagnostics'),
                    const SizedBox(height: 8),
                    const Text(
                      'Use SSH for authenticated diagnostics. This first mobile build does not run unattended SSH collectors or record iDRAC sensor history.',
                      style: TextStyle(color: Colors.grey, fontSize: 12),
                    ),
                    OutlinedButton.icon(
                      onPressed: sample?.ssh == true
                          ? () => widget.action(device, 'ssh')
                          : null,
                      icon: const Glyph('terminal'),
                      label: const Text('Open SSH diagnostics'),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
        const SizedBox(height: 16),
        const Text(
          'Checks pause when this app is backgrounded. Recorded sample availability is not continuous uptime.',
          style: TextStyle(color: Colors.grey, fontSize: 12),
        ),
      ],
    );
  }
}

class HistoryPainter extends CustomPainter {
  final List<HealthSample> samples;
  HistoryPainter(this.samples);
  @override
  void paint(Canvas canvas, Size size) {
    final grid = Paint()
      ..color = line
      ..strokeWidth = 1;
    for (var i = 0; i < 4; i++) {
      final y = size.height * i / 3;
      canvas.drawLine(Offset(0, y), Offset(size.width, y), grid);
    }
    if (samples.length < 2) return;
    final maximum = samples
            .map((s) => s.latency)
            .reduce((a, b) => a > b ? a : b)
            .clamp(1, 100000),
        path = Path();
    for (var i = 0; i < samples.length; i++) {
      final x = i * size.width / (samples.length - 1),
          y =
              size.height -
              (samples[i].latency / maximum) * (size.height - 12) -
              6;
      if (i == 0) {
        path.moveTo(x, y);
      } else {
        path.lineTo(x, y);
      }
    }
    canvas.drawPath(
      path,
      Paint()
        ..color = green
        ..style = PaintingStyle.stroke
        ..strokeWidth = 2,
    );
  }

  @override
  bool shouldRepaint(HistoryPainter old) =>
      samples.length != old.samples.length ||
      (samples.isNotEmpty &&
          old.samples.isNotEmpty &&
          !identical(samples.last, old.samples.last));
}
