import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:lab_manager_mobile/device.dart';
import 'package:lab_manager_mobile/transfer.dart';

void main() {
  test('Desktop QR pages import out of order, with repeated camera frames', () {
    final device = Device.create('192.0.2.1')
        .copy({'name': 'Pi-hole', 'icon': 'PiHole', 'notes': 'DNS'});
    final payload = {
      'format': 'labmanager.mobile',
      'version': 1,
      'username': 'desktop',
      'avatar': 2,
      'backup': BackupData([device], {}, {}).toJson(),
    };
    final encoded = base64Url
        .encode(zlib.encode(utf8.encode(jsonEncode(payload))))
        .replaceAll('=', '');
    final middle = encoded.length ~/ 2;
    final collector = TransferCollector();
    final a =
        'LMQR1:AAAAAAAAAAAAAAAAAAAAAA:1:2:${encoded.substring(0, middle)}';
    final b = 'LMQR1:AAAAAAAAAAAAAAAAAAAAAA:2:2:${encoded.substring(middle)}';
    collector.add(b);
    expect(collector.add(b), false);
    expect(collector.complete, false);
    expect(collector.decode, throwsFormatException);
    collector.add(a);
    final imported = collector.decode();
    expect(imported.avatar, 2);
    expect(imported.backup.devices.single.notes, 'DNS');
    expect(imported.backup.devices.single.os, 'DebianUbuntu');
  });
  test('Mixed and malformed QR transfers cannot be imported', () {
    final collector = TransferCollector()
      ..add('LMQR1:AAAAAAAAAAAAAAAAAAAAAA:1:2:YQ');
    expect(
      () => collector.add('LMQR1:BBBBBBBBBBBBBBBBBBBBBB:2:2:Yg'),
      throwsFormatException,
    );
    expect(
      () => collector.add('LMQR1:AAAAAAAAAAAAAAAAAAAAAA:1:2:YWJj'),
      throwsFormatException,
    );
    expect(
      () => TransferCollector().add('LMQR1:AAAAAAAAAAAAAAAAAAAAAA:1:189:YQ'),
      throwsFormatException,
    );
    expect(
      () => BackupData.parse({
        'version': 1,
        'devices': [
          Device.create('192.0.2.1').data,
          Device.create('192.0.2.1').data,
        ],
      }),
      throwsFormatException,
    );
  });
  test('OS defaults and reviewed commands follow the selected device', () {
    expect(
      Device.create('192.0.2.1').copy({'icon': 'DellIdrac'}).updateCommand,
      'racadm getsysinfo',
    );
    expect(
      Device.create('192.0.2.1').copy({'icon': 'PiHole'}).updateCommand,
      'apt update && apt upgrade',
    );
    expect(
      Device.create('192.0.2.1')
          .copy({'icon': 'PiHole', 'os': 'Arch', 'username': 'admin'})
          .updateCommand,
      'sudo pacman -Syu',
    );
  });
}
