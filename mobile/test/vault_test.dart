import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:lab_manager_mobile/device.dart';
import 'package:lab_manager_mobile/vault.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test(
    'Encrypted accounts isolate devices and serialize concurrent saves',
    () async {
      FlutterSecureStorage.setMockInitialValues({});
      final root = await Directory.systemTemp.createTemp('lab-mobile-vault-');
      final vault = LocalVault(directory: () async => root);
      try {
        await vault.load();
        await vault.register(
          'alice',
          'alice correct password',
          [0, 1],
          ['school name', 'first nickname'],
          true,
          2,
        );
        final device = Device.create('192.0.2.9');
        await Future.wait([
          vault.saveDevices([device]),
          vault.saveCredential(device.key, {
            'password': 'disposable ssh password',
          }),
          vault.trust('192.0.2.9:22', 'SHA256:test'),
        ]);
        expect(vault.devices.single.ip, '192.0.2.9');
        expect(
          vault.credential(device.key)!['password'],
          'disposable ssh password',
        );
        expect(vault.trusted('192.0.2.9:22'), 'SHA256:test');
        final bytes = await File('${root.path}/mobile-vault.enc').readAsBytes();
        expect(
          String.fromCharCodes(bytes).contains('disposable ssh password'),
          false,
        );
        expect(String.fromCharCodes(bytes).contains('192.0.2.9'), false);
        await vault.signOut();
        await vault.register(
          'bob',
          'bob correct password',
          [2, 3],
          ['teacher name', 'pet name'],
          false,
          0,
        );
        expect(vault.devices, isEmpty);
        expect(vault.trusted('192.0.2.9:22'), isNull);
        await vault.signOut();
        await vault.signIn('alice', 'alice correct password', false);
        expect(vault.devices.single.ip, '192.0.2.9');
        final reopened = LocalVault(directory: () async => root);
        await reopened.load();
        expect(reopened.username, isNull);
        await reopened.signIn('alice', 'alice correct password', false);
        expect(reopened.devices.single.ip, '192.0.2.9');
        await reopened.removeAccount();
        expect(reopened.usernames, ['bob']);
        reopened.dispose();
      } finally {
        vault.dispose();
        await root.delete(recursive: true);
      }
    },
  );
  test(
    'Missing device key never overwrites an existing encrypted vault',
    () async {
      FlutterSecureStorage.setMockInitialValues({});
      final root = await Directory.systemTemp.createTemp(
        'lab-mobile-missing-key-',
      );
      final file = File('${root.path}/mobile-vault.enc');
      await file.writeAsBytes([1, 2, 3, 4]);
      final vault = LocalVault(directory: () async => root);
      try {
        await expectLater(vault.load(), throwsStateError);
        expect(await file.readAsBytes(), [1, 2, 3, 4]);
      } finally {
        vault.dispose();
        await root.delete(recursive: true);
      }
    },
  );
}
