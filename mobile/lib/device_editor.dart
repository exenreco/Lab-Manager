import 'package:flutter/material.dart';

import 'device.dart';
import 'ui.dart';
import 'vault.dart';

class DeviceEditor extends StatefulWidget {
  final LocalVault vault;
  final Device? device;
  const DeviceEditor({super.key, required this.vault, this.device});
  @override
  State<DeviceEditor> createState() => _DeviceEditorState();
}

class _DeviceEditorState extends State<DeviceEditor> {
  late Device _draft;
  late final TextEditingController _name,
      _ip,
      _web,
      _ssh,
      _user,
      _group,
      _tags,
      _notes,
      _links;
  bool _many = false, _busy = false;
  String? _error;
  @override
  void initState() {
    super.initState();
    _draft = widget.device ?? Device.create('');
    _name = TextEditingController(text: widget.device?.name ?? '');
    _ip = TextEditingController(text: widget.device?.ip ?? '');
    _web = TextEditingController(text: '${_draft.webPort}');
    _ssh = TextEditingController(text: '${_draft.sshPort}');
    _user = TextEditingController(text: _draft.username);
    _group = TextEditingController(text: _draft.group);
    _tags = TextEditingController(text: _draft.tags);
    _notes = TextEditingController(text: _draft.notes);
    _links = TextEditingController(text: _draft.data['links'] as String? ?? '');
  }

  @override
  void dispose() {
    for (final value in [
      _name,
      _ip,
      _web,
      _ssh,
      _user,
      _group,
      _tags,
      _notes,
      _links,
    ]) {
      value.dispose();
    }
    super.dispose();
  }

  Widget _field(
    String label,
    TextEditingController controller, {
    int lines = 1,
    bool number = false,
    String? hint,
  }) => Padding(
    padding: const EdgeInsets.only(bottom: 14),
    child: TextField(
      controller: controller,
      maxLines: lines,
      keyboardType: number
          ? TextInputType.number
          : lines > 1
          ? TextInputType.multiline
          : TextInputType.text,
      autocorrect: false,
      decoration: InputDecoration(labelText: label, hintText: hint),
    ),
  );
  Future<void> _save() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final addresses = _many
          ? _ip.text
                .split(RegExp(r'[\s,;]+'))
                .where((s) => s.isNotEmpty)
                .toSet()
                .toList()
          : [_ip.text.trim()];
      if (addresses.isEmpty) {
        throw const FormatException('Enter at least one IP address.');
      }
      final created = addresses
          .map(
            (ip) => _draft.copy({
              'ip': ip,
              'name': _name.text.trim().isEmpty
                  ? 'Device $ip'
                  : _many
                  ? '${_name.text.trim()} $ip'
                  : _name.text.trim(),
              'web_port': int.tryParse(_web.text) ?? 0,
              'ssh_port': int.tryParse(_ssh.text) ?? 0,
              'username': _user.text.trim(),
              'group': _group.text.trim(),
              'tags': _tags.text.trim(),
              'notes': _notes.text,
              'links': _links.text,
            }),
          )
          .toList();
      for (final device in created) {
        device.validate();
      }
      final devices = widget.vault.devices;
      if (widget.device != null) {
        final index = devices.indexWhere((d) => d.ip == widget.device!.ip);
        if (index < 0) throw StateError('Device was removed.');
        devices[index] = created.single;
      } else {
        final existing = devices.map((d) => d.ip).toSet();
        devices.addAll(created.where((d) => !existing.contains(d.ip)));
      }
      await widget.vault.saveDevices(devices);
      if (mounted) Navigator.pop(context, true);
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
            Expanded(
              child: Text(
                widget.device == null ? 'Add Device' : 'Device settings',
                style: Theme.of(context).textTheme.titleLarge,
              ),
            ),
            IconButton(
              tooltip: 'Close',
              onPressed: () => Navigator.pop(context),
              icon: const Glyph('x'),
            ),
          ],
        ),
        const SizedBox(height: 18),
        if (widget.device == null) ...[
          SegmentedButton<bool>(
            segments: const [
              ButtonSegment(value: false, label: Text('One')),
              ButtonSegment(value: true, label: Text('Many')),
            ],
            selected: {_many},
            onSelectionChanged: (value) => setState(() => _many = value.first),
          ),
          const SizedBox(height: 18),
        ],
        _field(
          _many ? 'Name prefix (optional)' : 'Device name',
          _name,
          hint: 'Name your device',
        ),
        Row(
          children: [
            DeviceIcon(_draft, icons: widget.vault.icons, size: 44),
            const Spacer(),
            OutlinedButton(
              onPressed: () async {
                final icon = await showPanel<String>(
                  context,
                  IconChooser(_draft.icon),
                );
                if (icon != null && mounted) {
                  setState(
                    () => _draft = _draft.copy({
                      'icon': icon,
                      'custom_icon': null,
                    }),
                  );
                }
              },
              child: Text(
                widget.device == null ? 'Choose icon' : 'Change icon',
              ),
            ),
          ],
        ),
        const SizedBox(height: 20),
        _field(
          _many ? 'IP addresses' : 'IP address',
          _ip,
          lines: _many ? 4 : 1,
          hint: _many ? 'One or more IPv4 or IPv6 addresses' : '192.168.1.10',
        ),
        _field('Web port', _web, number: true),
        DropdownButtonFormField<bool>(
          initialValue: _draft.data['https'] == true,
          decoration: const InputDecoration(labelText: 'Protocol'),
          items: const [
            DropdownMenuItem(value: false, child: Text('HTTP')),
            DropdownMenuItem(value: true, child: Text('HTTPS')),
          ],
          onChanged: (v) =>
              setState(() => _draft = _draft.copy({'https': v ?? false})),
        ),
        const SizedBox(height: 14),
        DropdownButtonFormField<String>(
          initialValue: _draft.data['os'] as String? ?? 'Automatic',
          isExpanded: true,
          decoration: const InputDecoration(labelText: 'OS'),
          items: [
            DropdownMenuItem(
              value: 'Automatic',
              child: Text('Automatic · ${osLabels[_draft.os]}'),
            ),
            for (final os in operatingSystems)
              DropdownMenuItem(value: os, child: Text(osLabels[os]!)),
          ],
          onChanged: (v) => setState(
            () => _draft = _draft.copy({'os': v == 'Automatic' ? null : v}),
          ),
        ),
        const SizedBox(height: 14),
        ExpansionTile(
          tilePadding: EdgeInsets.zero,
          childrenPadding: const EdgeInsets.only(top: 12, left: 2, right: 2),
          title: const Text('SSH settings'),
          leading: const Glyph('terminal'),
          children: [
            _field('SSH username', _user),
            _field('SSH port', _ssh, number: true),
          ],
        ),
        ExpansionTile(
          tilePadding: EdgeInsets.zero,
          childrenPadding: const EdgeInsets.only(top: 12, left: 2, right: 2),
          title: const Text('Organization & notes'),
          leading: const Glyph('folder'),
          children: [
            _field('Group', _group),
            _field('Tags', _tags, hint: 'Separate tags with commas'),
            _field('Notes', _notes, lines: 4),
            _field('Links', _links, lines: 2),
          ],
        ),
        if (_error != null)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 12),
            child: Text(
              _error!,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ),
        const SizedBox(height: 16),
        Row(
          children: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancel'),
            ),
            const Spacer(),
            FilledButton(
              onPressed: _busy ? null : _save,
              child: Text(
                _busy
                    ? 'Saving…'
                    : widget.device == null
                    ? 'Save Device/s'
                    : 'Done',
              ),
            ),
          ],
        ),
      ],
    ),
  );
}

class IconChooser extends StatefulWidget {
  final String selected;
  const IconChooser(this.selected, {super.key});
  @override
  State<IconChooser> createState() => _IconChooserState();
}

class _IconChooserState extends State<IconChooser> {
  String _query = '';
  @override
  Widget build(BuildContext context) {
    final icons = deviceIcons.keys
        .where((v) => v.toLowerCase().contains(_query.toLowerCase()))
        .toList();
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Row(
          children: [
            const Expanded(
              child: Text(
                'Choose a device icon',
                style: TextStyle(fontSize: 20),
              ),
            ),
            IconButton(
              onPressed: () => Navigator.pop(context),
              tooltip: 'Close',
              icon: const Glyph('x'),
            ),
          ],
        ),
        const SizedBox(height: 12),
        TextField(
          decoration: const InputDecoration(hintText: 'Search icons'),
          onChanged: (value) => setState(() => _query = value),
        ),
        const SizedBox(height: 16),
        SizedBox(
          height: 360,
          child: GridView.builder(
            gridDelegate: const SliverGridDelegateWithMaxCrossAxisExtent(
              maxCrossAxisExtent: 120,
              mainAxisExtent: 92,
              crossAxisSpacing: 8,
              mainAxisSpacing: 8,
            ),
            itemCount: icons.length,
            itemBuilder: (context, index) {
              final name = icons[index];
              return InkWell(
                onTap: () => Navigator.pop(context, name),
                borderRadius: BorderRadius.circular(12),
                child: Container(
                  decoration: BoxDecoration(
                    color: surface,
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(
                      color: name == widget.selected ? green : line,
                    ),
                  ),
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      DeviceIcon(
                        Device.create('192.0.2.1').copy({'icon': name}),
                        size: 30,
                      ),
                      const SizedBox(height: 10),
                      Text(
                        name,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(fontSize: 11),
                      ),
                    ],
                  ),
                ),
              );
            },
          ),
        ),
      ],
    );
  }
}
