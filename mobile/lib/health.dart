import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';

import 'device.dart';

Future<bool> reachable(String ip, int port) async {
  try {
    final socket = await Socket.connect(
      ip,
      port,
      timeout: const Duration(seconds: 3),
    );
    socket.destroy();
    return true;
  } catch (_) {
    return false;
  }
}

class HealthSample {
  final bool web, ssh;
  final int latency;
  final DateTime checked;
  bool get online => web || ssh;
  HealthSample(this.web, this.ssh, this.latency) : checked = DateTime.now();
}

class HealthMonitor extends ChangeNotifier {
  final List<Device> Function() devices;
  final latest = <String, HealthSample>{},
      history = <String, List<HealthSample>>{};
  Timer? _timer;
  bool paused = false, busy = false, _disposed = false;
  int _generation = 0;
  HealthMonitor(this.devices);
  void start() {
    _timer?.cancel();
    _timer = Timer.periodic(const Duration(seconds: 30), (_) => refresh());
    unawaited(refresh());
  }

  void setPaused(bool value) {
    paused = value;
    _generation++;
    if (!value) unawaited(refresh());
    if (!_disposed) notifyListeners();
  }

  Future<void> refresh() async {
    if (busy || paused || _disposed) return;
    busy = true;
    notifyListeners();
    final generation = _generation;
    try {
      final snapshot = devices();
      for (var start = 0; start < snapshot.length; start += 4) {
        if (paused || _disposed || generation != _generation) break;
        await Future.wait(
          snapshot.skip(start).take(4).map((device) async {
            final watch = Stopwatch()..start();
            final status = await Future.wait([
              reachable(device.ip, device.webPort),
              reachable(device.ip, device.sshPort),
            ]);
            if (paused || _disposed || generation != _generation) return;
            final sample = HealthSample(
              status[0],
              status[1],
              watch.elapsedMilliseconds,
            );
            latest[device.ip] = sample;
            final samples = history.putIfAbsent(device.ip, () => [])
              ..add(sample);
            if (samples.length > 120) {
              samples.removeRange(0, samples.length - 120);
            }
          }),
        );
      }
      final addresses = devices().map((d) => d.ip).toSet();
      latest.removeWhere((key, _) => !addresses.contains(key));
      history.removeWhere((key, _) => !addresses.contains(key));
    } finally {
      busy = false;
      if (!_disposed) notifyListeners();
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _generation++;
    _timer?.cancel();
    super.dispose();
  }
}

bool privateSubnet(String prefix) {
  final parts = prefix.split('.').map(int.tryParse).toList();
  if (parts.length != 3 || parts.any((v) => v == null || v < 0 || v > 255)) {
    return false;
  }
  return parts[0] == 10 ||
      (parts[0] == 172 && parts[1]! >= 16 && parts[1]! <= 31) ||
      (parts[0] == 192 && parts[1] == 168);
}

Future<List<Device>> discover(
  String prefix,
  bool Function() cancelled,
  void Function(int) progress,
) async {
  if (!privateSubnet(prefix)) {
    throw const FormatException(
      'Scan a private IPv4 subnet, such as 192.168.1.',
    );
  }
  final found = <Device>[];
  var checked = 0;
  for (var start = 1; start <= 254 && !cancelled(); start += 12) {
    await Future.wait(
      List.generate((255 - start).clamp(0, 12), (offset) async {
        final ip = '$prefix.${start + offset}';
        final results = await Future.wait(
          [80, 443, 22, 8006].map((port) => reachable(ip, port)),
        );
        if (cancelled()) return;
        if (results.any((value) => value)) {
          final port = results[0]
              ? 80
              : results[1]
              ? 443
              : results[3]
              ? 8006
              : 80;
          found.add(
            Device.create(ip).copy({
              'web_port': port,
              'https': port == 443 || port == 8006,
              'icon': port == 8006 ? 'Proxmox' : 'Server',
            }),
          );
        }
        progress(++checked);
      }),
    );
  }
  return found..sort((a, b) => a.ip.compareTo(b.ip));
}
