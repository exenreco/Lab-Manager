import 'dart:async';
import 'dart:convert';

import 'package:dartssh2/dartssh2.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:xterm/xterm.dart';

import 'device.dart';
import 'vault.dart';
import 'ui.dart';

class RemoteTab {
  final Device device;
  final String? command;
  final terminal = Terminal(maxLines: 2000), selection = TerminalController();
  SSHClient? client;
  SSHSession? session;
  SSHSocket? socket;
  final subscriptions = <StreamSubscription>[];
  String status = 'Connecting…';
  bool closed = false;
  int generation = 0;
  RemoteTab(this.device, this.command);
  Future<void> disconnect() async {
    generation++;
    session?.close();
    session = null;
    for (final subscription in subscriptions) {
      await subscription.cancel();
    }
    subscriptions.clear();
    try {
      await client?.close();
    } catch (_) {
      /* Already disconnected. */
    }
    client = null;
    socket?.close();
    socket = null;
    status = 'Disconnected';
  }
}

class TerminalHub extends ChangeNotifier {
  final LocalVault vault;
  final List<RemoteTab> tabs = [];
  int selected = 0;
  bool _disposed = false;
  TerminalHub(this.vault);
  RemoteTab? get active =>
      tabs.isEmpty ? null : tabs[selected.clamp(0, tabs.length - 1)];
  void refresh() {
    if (!_disposed) notifyListeners();
  }

  Future<void> open(
    BuildContext context,
    Device device, {
    String? command,
  }) async {
    final tab = RemoteTab(device, command);
    tabs.add(tab);
    selected = tabs.length - 1;
    refresh();
    await connect(context, tab);
  }

  Future<void> connect(BuildContext context, RemoteTab tab) async {
    await tab.disconnect();
    final attempt = tab.generation;
    if (tab.closed || !context.mounted || _disposed) return;
    final credentials = await showPanel<Map<String, dynamic>>(
      context,
      CredentialsPanel(vault: vault, device: tab.device),
    );
    if (credentials == null) {
      tab.status = 'Connection cancelled';
      refresh();
      return;
    }
    if (!context.mounted ||
        tab.closed ||
        _disposed ||
        attempt != tab.generation) {
      return;
    }
    final owner = vault.username;
    final endpoint = '${tab.device.ip}:${tab.device.sshPort}';
    tab.status = 'Connecting…';
    refresh();
    try {
      final socket = await SSHSocket.connect(
        tab.device.ip,
        tab.device.sshPort,
        timeout: const Duration(seconds: 8),
      );
      tab.socket = socket;
      if (tab.closed ||
          _disposed ||
          owner != vault.username ||
          attempt != tab.generation) {
        socket.close();
        return;
      }
      final identity = credentials['key'] as String? ?? '';
      final password = credentials['password'] as String? ?? '';
      tab.client = SSHClient(
        socket,
        username: tab.device.username,
        authTimeout: const Duration(seconds: 20),
        keepAliveInterval: const Duration(seconds: 15),
        identities: identity.isEmpty
            ? null
            : SSHKeyPair.fromPem(
                identity,
                credentials['passphrase'] as String?,
              ),
        onPasswordRequest: () => password.isEmpty ? null : password,
        onUserInfoRequest: (request) =>
            List.generate(request.prompts.length, (_) => password),
        onVerifyHostKey: (type, bytes) async {
          if (tab.closed ||
              _disposed ||
              owner != vault.username ||
              attempt != tab.generation ||
              !context.mounted) {
            return false;
          }
          final fingerprint = utf8.decode(bytes);
          final trusted = vault.trusted(endpoint);
          if (trusted != null) {
            if (trusted != fingerprint) {
              throw StateError(
                'HOST KEY CHANGED for $endpoint. Connection blocked. Verify the device identity before forgetting its saved key in Settings.',
              );
            }
            return true;
          }
          final accepted = await confirm(
            context,
            'Verify SSH host key',
            '$endpoint\n$type\n\n$fingerprint\n\nCompare this fingerprint with the device administrator before trusting it.',
            action: 'Trust this host',
          );
          if (!accepted ||
              tab.closed ||
              _disposed ||
              owner != vault.username ||
              attempt != tab.generation) {
            return false;
          }
          await vault.trust(endpoint, fingerprint);
          return true;
        },
      );
      final pty = SSHPtyConfig(
        width: tab.terminal.viewWidth,
        height: tab.terminal.viewHeight,
      );
      final session = tab.command == null
          ? await tab.client!.shell(pty: pty)
          : await tab.client!.execute(tab.command!, pty: pty);
      if (tab.closed ||
          _disposed ||
          owner != vault.username ||
          attempt != tab.generation) {
        session.close();
        await tab.disconnect();
        return;
      }
      tab.session = session;
      tab.terminal.onOutput = (value) {
        if (tab.session != null) {
          tab.session!.stdin.add(Uint8List.fromList(utf8.encode(value)));
        }
      };
      tab.terminal.onResize = (width, height, pixelWidth, pixelHeight) =>
          tab.session?.resizeTerminal(width, height, pixelWidth, pixelHeight);
      tab.subscriptions.add(
        session.stdout
            .cast<List<int>>()
            .transform(const Utf8Decoder(allowMalformed: true))
            .listen(tab.terminal.write),
      );
      tab.subscriptions.add(
        session.stderr
            .cast<List<int>>()
            .transform(const Utf8Decoder(allowMalformed: true))
            .listen(tab.terminal.write),
      );
      tab.status = 'Connected';
      refresh();
      unawaited(
        session.done
            .then((_) {
              if (attempt != tab.generation || tab.closed || _disposed) return;
              tab.status =
                  'Disconnected · exit ${session.exitCode ?? "unknown"}';
              tab.session = null;
              refresh();
            })
            .catchError((Object error) {
              if (attempt != tab.generation || tab.closed || _disposed) return;
              tab.status = error.toString();
              refresh();
            }),
      );
    } catch (error) {
      if (attempt != tab.generation || tab.closed || _disposed) return;
      await tab.disconnect();
      tab.status = 'SSH: $error';
      tab.terminal.write('\r\n${tab.status}\r\n');
      refresh();
    }
  }

  Future<void> close(RemoteTab tab) async {
    tab.closed = true;
    await tab.disconnect();
    tabs.remove(tab);
    selected = selected.clamp(0, (tabs.length - 1).clamp(0, tabs.length));
    tab.selection.dispose();
    refresh();
  }

  @override
  void dispose() {
    _disposed = true;
    for (final tab in tabs) {
      tab.closed = true;
      unawaited(tab.disconnect());
      tab.selection.dispose();
    }
    tabs.clear();
    super.dispose();
  }
}

class CredentialsPanel extends StatefulWidget {
  final LocalVault vault;
  final Device device;
  const CredentialsPanel({
    super.key,
    required this.vault,
    required this.device,
  });
  @override
  State<CredentialsPanel> createState() => _CredentialsPanelState();
}

class _CredentialsPanelState extends State<CredentialsPanel> {
  final _password = TextEditingController(),
      _key = TextEditingController(),
      _passphrase = TextEditingController();
  bool _useKey = false, _remember = false, _busy = false;
  String? _error;
  @override
  void initState() {
    super.initState();
    final saved = widget.vault.credential(widget.device.key);
    if (saved != null) {
      _useKey = (saved['key'] ?? '') != '';
      _remember = true;
      _password.text = saved['password'] as String? ?? '';
      _key.text = saved['key'] as String? ?? '';
      _passphrase.text = saved['passphrase'] as String? ?? '';
    }
  }

  @override
  void dispose() {
    _password.dispose();
    _key.dispose();
    _passphrase.dispose();
    super.dispose();
  }

  Future<void> _connect() async {
    setState(() => _busy = true);
    try {
      if (_useKey) {
        SSHKeyPair.fromPem(
          _key.text,
          _passphrase.text.isEmpty ? null : _passphrase.text,
        );
      } else if (_password.text.isEmpty) {
        throw const FormatException('Enter your SSH password.');
      }
      final value = <String, dynamic>{
        'password': _useKey ? '' : _password.text,
        'key': _useKey ? _key.text : '',
        'passphrase': _useKey ? _passphrase.text : '',
      };
      await widget.vault.saveCredential(
        widget.device.key,
        _remember ? value : null,
      );
      if (mounted) Navigator.pop(context, value);
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) => SingleChildScrollView(
    child: Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            DeviceIcon(widget.device),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    widget.device.name,
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                  Text(
                    '${widget.device.username}@${widget.device.ip}:${widget.device.sshPort}',
                    style: const TextStyle(color: Colors.grey),
                  ),
                ],
              ),
            ),
            IconButton(
              tooltip: 'Close',
              onPressed: () => Navigator.pop(context),
              icon: const Glyph('x'),
            ),
          ],
        ),
        const SizedBox(height: 20),
        SegmentedButton<bool>(
          segments: const [
            ButtonSegment(value: false, label: Text('Password')),
            ButtonSegment(value: true, label: Text('SSH key')),
          ],
          selected: {_useKey},
          onSelectionChanged: (value) => setState(() => _useKey = value.first),
        ),
        const SizedBox(height: 16),
        if (_useKey) ...[
          TextField(
            controller: _key,
            minLines: 5,
            maxLines: 8,
            autocorrect: false,
            decoration: const InputDecoration(
              labelText: 'Private SSH key',
              hintText: 'Paste your PEM or OpenSSH private key',
            ),
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _passphrase,
            obscureText: true,
            decoration: const InputDecoration(
              labelText: 'Key passphrase (if encrypted)',
            ),
          ),
        ] else
          TextField(
            controller: _password,
            obscureText: true,
            autocorrect: false,
            decoration: const InputDecoration(labelText: 'SSH password'),
          ),
        CheckboxListTile(
          contentPadding: EdgeInsets.zero,
          title: const Text('Remember securely on this device'),
          subtitle: const Text('Saved only in this account’s encrypted vault.'),
          value: _remember,
          onChanged: (v) => setState(() => _remember = v ?? false),
        ),
        if (_error != null)
          Text(
            _error!,
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          ),
        FilledButton(
          onPressed: _busy ? null : _connect,
          child: Text(_busy ? 'Saving…' : 'Connect'),
        ),
      ],
    ),
  );
}

class TerminalPage extends StatelessWidget {
  final TerminalHub hub;
  final VoidCallback add;
  const TerminalPage({super.key, required this.hub, required this.add});
  @override
  Widget build(BuildContext context) => AnimatedBuilder(
    animation: hub,
    builder: (context, _) {
      final active = hub.active;
      if (active == null) {
        return Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Glyph('terminal', size: 48, color: green),
              const SizedBox(height: 16),
              const Text('Your remote shells, right here.'),
              const SizedBox(height: 16),
              FilledButton.icon(
                onPressed: add,
                icon: const Glyph('plus', color: navigation),
                label: const Text('Connect to a device'),
              ),
            ],
          ),
        );
      }
      return Column(
        children: [
          Container(
            color: navigation,
            height: 52,
            child: Row(
              children: [
                Expanded(
                  child: ListView.separated(
                    scrollDirection: Axis.horizontal,
                    itemCount: hub.tabs.length,
                    separatorBuilder: (_, _) => const SizedBox(width: 4),
                    itemBuilder: (context, index) {
                      final tab = hub.tabs[index];
                      return Container(
                        margin: const EdgeInsets.symmetric(vertical: 5),
                        decoration: BoxDecoration(
                          color: index == hub.selected ? surface : navigation,
                          borderRadius: BorderRadius.circular(12),
                          border: Border.all(color: line),
                        ),
                        child: Row(
                          children: [
                            InkWell(
                              onTap: () {
                                hub.selected = index;
                                hub.refresh();
                              },
                              child: Padding(
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 12,
                                ),
                                child: Row(
                                  children: [
                                    DeviceIcon(tab.device, size: 20),
                                    const SizedBox(width: 8),
                                    Text(tab.device.name),
                                  ],
                                ),
                              ),
                            ),
                            IconButton(
                              tooltip: 'Close terminal',
                              onPressed: () => hub.close(tab),
                              icon: const Glyph('x', size: 16),
                            ),
                          ],
                        ),
                      );
                    },
                  ),
                ),
                if (active.command == null)
                  IconButton(
                    tooltip: 'Add terminal',
                    onPressed: add,
                    icon: const Glyph('plus'),
                  ),
              ],
            ),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
            child: Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        '${active.device.username}@${active.device.ip}:${active.device.sshPort}',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      Text(
                        active.status,
                        style: const TextStyle(
                          fontSize: 11,
                          color: Colors.grey,
                        ),
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ),
                ),
                IconButton(
                  tooltip: 'Copy selection',
                  onPressed: () {
                    final selection = active.selection.selection;
                    if (selection != null) {
                      Clipboard.setData(
                        ClipboardData(
                          text: active.terminal.buffer.getText(selection),
                        ),
                      );
                    }
                  },
                  icon: const Glyph('copy'),
                ),
                IconButton(
                  tooltip: 'Reconnect',
                  onPressed: () => hub.connect(context, active),
                  icon: const Glyph('refresh-cw'),
                ),
                IconButton(
                  tooltip: 'Disconnect',
                  onPressed: () async {
                    await active.disconnect();
                    hub.refresh();
                  },
                  icon: const Glyph('unplug'),
                ),
              ],
            ),
          ),
          Expanded(
            child: TerminalView(
              active.terminal,
              key: ObjectKey(active),
              controller: active.selection,
              padding: const EdgeInsets.all(8),
              textStyle: const TerminalStyle(fontSize: 12),
              onSecondaryTapDown: (_, _) async {
                final value = await Clipboard.getData(Clipboard.kTextPlain);
                if (value != null && active.session != null) {
                  active.session!.stdin.add(
                    Uint8List.fromList(utf8.encode(value.text ?? '')),
                  );
                }
              },
            ),
          ),
          SafeArea(
            top: false,
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: Row(
                children: [
                  for (final item in <(String, String)>[
                    ('Esc', '\x1b'),
                    ('Tab', '\t'),
                    ('Ctrl+C', '\x03'),
                    ('Ctrl+X', '\x18'),
                    ('↑', '\x1b[A'),
                    ('↓', '\x1b[B'),
                    ('←', '\x1b[D'),
                    ('→', '\x1b[C'),
                  ])
                    TextButton(
                      onPressed: () => active.session?.stdin.add(
                        Uint8List.fromList(utf8.encode(item.$2)),
                      ),
                      child: Text(item.$1),
                    ),
                  IconButton(
                    tooltip: 'Paste',
                    onPressed: () async {
                      final value = await Clipboard.getData(
                        Clipboard.kTextPlain,
                      );
                      if (value?.text != null) {
                        active.session?.stdin.add(
                          Uint8List.fromList(utf8.encode(value!.text!)),
                        );
                      }
                    },
                    icon: const Glyph('copy'),
                  ),
                ],
              ),
            ),
          ),
        ],
      );
    },
  );
}
