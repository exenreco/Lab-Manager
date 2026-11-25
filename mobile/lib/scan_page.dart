import 'package:flutter/material.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

import 'transfer.dart';
import 'ui.dart';

class ScanPage extends StatefulWidget {
  const ScanPage({super.key});
  @override
  State<ScanPage> createState() => _ScanPageState();
}

class _ScanPageState extends State<ScanPage> {
  final _collector = TransferCollector(),
      _scanner = MobileScannerController(formats: [BarcodeFormat.qrCode]),
      _paste = TextEditingController();
  bool _manual = false, _previewing = false;
  String? _error;
  @override
  void dispose() {
    _scanner.dispose();
    _paste.dispose();
    super.dispose();
  }

  Future<void> _accept(String text) async {
    if (_previewing) return;
    try {
      for (final page
          in text.split(RegExp(r'\s+')).where((value) => value.isNotEmpty)) {
        _collector.add(page);
      }
      setState(() => _error = null);
      if (_collector.complete) {
        _previewing = true;
        final imported = _collector.decode();
        final accepted = await confirm(
          context,
          'Import ${imported.backup.devices.length} device(s)?',
          'From desktop workspace: ${imported.username}\n\n${imported.backup.devices.take(10).map((d) => '${d.name} · ${d.ip}').join('\n')}\n\nMatching IP addresses will be updated. Notes, icons, monitoring settings and avatar choice are included. Passwords and private keys are not transferred.',
          action: 'Import devices',
        );
        if (accepted && mounted) {
          Navigator.pop(context, imported);
          return;
        }
        _previewing = false;
      }
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
      _previewing = false;
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(
      title: const Text('Transfer from desktop'),
      actions: [
        IconButton(
          tooltip: 'Reset transfer',
          onPressed: () => setState(() {
            _collector.reset();
            _error = null;
          }),
          icon: const Glyph('refresh-cw'),
        ),
      ],
    ),
    body: Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(16),
          child: SegmentedButton<bool>(
            segments: const [
              ButtonSegment(value: false, label: Text('Scan QR')),
              ButtonSegment(value: true, label: Text('Paste codes')),
            ],
            selected: {_manual},
            onSelectionChanged: (value) =>
                setState(() => _manual = value.first),
          ),
        ),
        Expanded(
          child: _manual
              ? Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: _paste,
                          maxLines: null,
                          expands: true,
                          autocorrect: false,
                          decoration: const InputDecoration(
                            hintText: 'Paste all LMQR1 pages, one per line.',
                          ),
                        ),
                      ),
                      const SizedBox(height: 16),
                      FilledButton(
                        onPressed: () => _accept(_paste.text),
                        child: const Text('Read transfer'),
                      ),
                    ],
                  ),
                )
              : MobileScanner(
                  controller: _scanner,
                  onDetect: (capture) {
                    for (final code in capture.barcodes) {
                      if (code.rawValue != null) {
                        _accept(code.rawValue!);
                        break;
                      }
                    }
                  },
                  errorBuilder: (context, error) => Center(
                    child: Padding(
                      padding: const EdgeInsets.all(24),
                      child: Text(
                        'Camera unavailable or permission denied. You can use Paste codes or import a desktop backup file instead.\n\n$error',
                      ),
                    ),
                  ),
                ),
        ),
        SafeArea(
          top: false,
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              children: [
                Text(
                  '${_collector.received} / ${_collector.count == 0 ? '—' : _collector.count} pages collected',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
                const Text(
                  'On desktop: Settings → Transfer to mobile → Show QR code. Scan every page of the same transfer.',
                  textAlign: TextAlign.center,
                  style: TextStyle(color: Colors.grey),
                ),
                if (_error != null)
                  Text(
                    _error!,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
              ],
            ),
          ),
        ),
      ],
    ),
  );
}
