import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'device.dart';

const maxTransferBytes = 32 * 1024 * 1024;

class _BoundedSink implements Sink<List<int>> {
  final bytes = BytesBuilder(copy: false);
  int size = 0;
  @override
  void add(List<int> data) {
    size += data.length;
    if (size > maxTransferBytes) {
      throw const FormatException('Transfer is too large.');
    }
    bytes.add(data);
  }

  @override
  void close() {}
}

class TransferCollector {
  String? id;
  int count = 0;
  final Map<int, String> _parts = {};
  int get received => _parts.length;
  bool get complete => count > 0 && received == count;
  void reset() {
    id = null;
    count = 0;
    _parts.clear();
  }

  bool add(String page) {
    if (page.length > 800) throw const FormatException('QR page is too large.');
    final fields = page.split(':');
    if (fields.length != 5 ||
        fields[0] != 'LMQR1' ||
        !RegExp(r'^[A-Za-z0-9_-]{22}$').hasMatch(fields[1])) {
      throw const FormatException('This is not a Lab Manager transfer QR.');
    }
    final index = int.tryParse(fields[2]) ?? 0,
        total = int.tryParse(fields[3]) ?? 0;
    if (total < 1 ||
        total > 188 ||
        index < 1 ||
        index > total ||
        !RegExp(r'^[A-Za-z0-9_-]+$').hasMatch(fields[4])) {
      throw const FormatException('Invalid QR page.');
    }
    if (id != null && (id != fields[1] || count != total)) {
      throw const FormatException(
        'Mixed transfers. Reset before scanning a different transfer.',
      );
    }
    if (_parts.containsKey(index)) {
      if (_parts[index] != fields[4]) {
        throw const FormatException('Conflicting QR page.');
      }
      return false; // The camera may deliver the same unchanged frame repeatedly.
    }
    id = fields[1];
    count = total;
    _parts[index] = fields[4];
    return true;
  }

  MobileImport decode() {
    if (!complete) {
      throw const FormatException('Scan every QR page before importing.');
    }
    final encoded = List.generate(count, (i) => _parts[i + 1]!).join();
    if (encoded.length > 128 * 1024) {
      throw const FormatException('QR transfer is too large.');
    }
    final compressed = base64Url.decode(base64Url.normalize(encoded));
    final output = _BoundedSink();
    final decoder = ZLibDecoder().startChunkedConversion(output);
    for (var start = 0; start < compressed.length; start += 1024) {
      decoder.add(
        compressed.sublist(start, (start + 1024).clamp(0, compressed.length)),
      );
    }
    decoder.close();
    final json = jsonDecode(utf8.decode(output.bytes.takeBytes()));
    if (json is! Map<String, dynamic> ||
        json['format'] != 'labmanager.mobile' ||
        json['version'] != 1) {
      throw const FormatException('Unsupported mobile transfer.');
    }
    final backup = BackupData.parse(json['backup']);
    return MobileImport(
      json['username'] as String? ?? '',
      (json['avatar'] as int? ?? 0).clamp(0, 4),
      backup,
    );
  }
}

class MobileImport {
  final String username;
  final int avatar;
  final BackupData backup;
  MobileImport(this.username, this.avatar, this.backup);
}

class BackupData {
  final List<Device> devices;
  final Map<String, dynamic> monitoring, icons;
  BackupData(this.devices, this.monitoring, this.icons);
  factory BackupData.parse(dynamic value) {
    if (value is! Map<String, dynamic> ||
        value['version'] != 1 ||
        value['devices'] is! List) {
      throw const FormatException('Unsupported backup.');
    }
    final raw = value['devices'] as List;
    if (raw.length > 4096) throw const FormatException('Too many devices.');
    final devices = raw
        .map((v) => Device(Map<String, dynamic>.from(v as Map)))
        .toList();
    final addresses = <String>{};
    for (final device in devices) {
      device.validate();
      if (!addresses.add(InternetAddress(device.ip).address)) {
        throw const FormatException('Duplicate device address.');
      }
    }
    final monitoring = Map<String, dynamic>.from(
      value['monitoring'] as Map? ?? {},
    );
    final icons = Map<String, dynamic>.from(value['icons'] as Map? ?? {});
    if (icons.length > 512) throw const FormatException('Too many icons.');
    var total = 0;
    for (final entry in icons.entries) {
      if (entry.key.length > 160 ||
          entry.key.contains('..') ||
          !RegExp(r'^[A-Za-z0-9_.-]+\.png$').hasMatch(entry.key) ||
          entry.value is! List) {
        throw const FormatException('Invalid imported icon.');
      }
      final bytes = List<int>.from(entry.value as List);
      total += bytes.length;
      if (bytes.length < 24 ||
          bytes.length > 1024 * 1024 ||
          total > 8 * 1024 * 1024 ||
          !bytes.every((b) => b >= 0 && b <= 255) ||
          base64Encode(bytes.take(8).toList()) != 'iVBORw0KGgo=') {
        throw const FormatException('Invalid PNG icon.');
      }
      final header = ByteData.sublistView(Uint8List.fromList(bytes));
      if (header.getUint32(16) > 256 || header.getUint32(20) > 256) {
        throw const FormatException('Icon dimensions exceed 256 pixels.');
      }
    }
    for (final device in devices) {
      if (device.data['custom_icon'] != null &&
          !icons.containsKey(device.data['custom_icon'])) {
        throw const FormatException('Missing imported device icon.');
      }
    }
    final keys = devices.map((device) => device.monitorKey).toSet();
    for (final entry in monitoring.entries) {
      final value = entry.value;
      if (value is! Map ||
          !keys.contains(entry.key) ||
          value['password_id'] != null ||
          (value['identity_file'] ?? '') != '') {
        throw const FormatException(
          'Backups cannot contain SSH credential references.',
        );
      }
      final interval = value['interval'] ?? 60;
      if (interval is! int ||
          interval < 15 ||
          interval > 3600 ||
          (value['services'] is List &&
              (value['services'] as List).length > 8)) {
        throw const FormatException('Invalid monitoring settings.');
      }
      for (final field in ['cpu_limit', 'memory_limit', 'disk_limit']) {
        final limit = value[field] ?? 90;
        if (limit is! num || !limit.isFinite || limit < 1 || limit > 100) {
          throw const FormatException('Invalid monitoring threshold.');
        }
      }
    }
    return BackupData(devices, monitoring, icons);
  }
  Map<String, dynamic> toJson() => {
    'version': 1,
    'created': DateTime.now().millisecondsSinceEpoch ~/ 1000,
    'devices': devices.map((d) => d.data).toList(),
    'monitoring': monitoring,
    'icons': icons,
  };
}
