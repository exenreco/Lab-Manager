import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:lab_manager_mobile/auth_page.dart';
import 'package:lab_manager_mobile/device.dart';
import 'package:lab_manager_mobile/devices_page.dart';
import 'package:lab_manager_mobile/health.dart';
import 'package:lab_manager_mobile/ui.dart';
import 'package:lab_manager_mobile/vault.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  testWidgets('Sign-in and registration fit a narrow phone', (tester) async {
    FlutterSecureStorage.setMockInitialValues({});
    final root = Directory.systemTemp.createTempSync('lab-mobile-auth-layout-');
    final vault = LocalVault(directory: () async => root);
    await tester.runAsync(vault.load);
    tester.view.physicalSize = const Size(360, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await tester.pumpWidget(
      MaterialApp(theme: appTheme(), home: AuthPage(vault)),
    );
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    await tester.tap(find.text('Create a local account'));
    await tester.pumpAndSettle();
    expect(find.text('Account recovery'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
    vault.dispose();
    await tester.runAsync(() => root.delete(recursive: true));
  });
  testWidgets(
    'Device actions and grouped list/grid stay usable on phone and tablet',
    (tester) async {
      FlutterSecureStorage.setMockInitialValues({});
      final root = Directory.systemTemp.createTempSync(
        'lab-mobile-device-layout-',
      );
      late LocalVault vault;
      await tester.runAsync(() async {
        vault = LocalVault(directory: () async => root);
        await vault.load();
        await vault.register(
          'mobile',
          'correct mobile password',
          [0, 1],
          ['school name', 'nickname'],
          false,
          0,
        );
        await vault.saveDevices([
          Device.create('192.0.2.1').copy({
            'name': 'Pi-hole DNS',
            'group': 'Home lab',
            'icon': 'PiHole',
          }),
          Device.create('192.0.2.2').copy({
            'name': 'Dell iDRAC',
            'group': 'Home lab',
            'icon': 'DellIdrac',
          }),
        ]);
      });
      final health = HealthMonitor(() => vault.devices);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      for (final size in [const Size(360, 900), const Size(1024, 900)]) {
        tester.view.physicalSize = size;
        await tester.pumpWidget(
          MaterialApp(
            theme: appTheme(),
            home: Scaffold(
              body: DevicesPage(
                vault: vault,
                health: health,
                edit: ([device]) async {},
                action: (_, _) async {},
                importQr: () async {},
                notice: (_) {},
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        expect(find.text('Home lab'), findsOneWidget);
        expect(find.text('Scan QR'), findsNothing);
        expect(find.text('Scan network'), findsNothing);
        expect(find.byTooltip('Scan QR'), findsOneWidget);
        expect(find.byTooltip('Scan network'), findsOneWidget);
        expect(find.byTooltip('Add Device'), findsOneWidget);
        await tester.tap(find.byTooltip('Switch to grid view'));
        await tester.pumpAndSettle();
        expect(find.byTooltip('Switch to list view'), findsOneWidget);
        expect(tester.takeException(), isNull);
        await tester.tap(find.byTooltip('Switch to list view'));
        await tester.pumpAndSettle();

        expect(tester.takeException(), isNull);
        await tester.tap(find.byType(PopupMenuButton<String>).first);
        await tester.pumpAndSettle();
        expect(find.text('SSH shell'), findsOneWidget);
        expect(find.text('Remove device'), findsOneWidget);
        expect(tester.takeException(), isNull);
        await tester.tapAt(const Offset(5, 5));
        await tester.pumpAndSettle();
      }
      await tester.pumpWidget(const SizedBox());
      health.dispose();
      vault.dispose();
      await tester.runAsync(() => root.delete(recursive: true));
    },
  );
}
