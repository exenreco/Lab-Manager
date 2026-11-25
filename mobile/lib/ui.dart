import 'dart:typed_data';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';

import 'device.dart';

const green = Color(0xff49b980),
    surface = Color(0xff232323),
    navigation = Color(0xff121212),
    line = Color(0xff383838);
ThemeData appTheme() => ThemeData(
  brightness: Brightness.dark,
  useMaterial3: true,
  scaffoldBackgroundColor: const Color(0xff1d1d1d),
  colorScheme: const ColorScheme.dark(
    primary: green,
    surface: surface,
    secondary: green,
  ),
  appBarTheme: const AppBarTheme(
    backgroundColor: navigation,
    surfaceTintColor: Colors.transparent,
  ),
  inputDecorationTheme: InputDecorationTheme(
    filled: true,
    fillColor: navigation,
    hintStyle: const TextStyle(color: Color(0xff777777)),
    border: OutlineInputBorder(
      borderRadius: BorderRadius.circular(12),
      borderSide: const BorderSide(color: line),
    ),
    enabledBorder: OutlineInputBorder(
      borderRadius: BorderRadius.circular(12),
      borderSide: const BorderSide(color: line),
    ),
    contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 14),
  ),
  cardTheme: CardThemeData(
    color: surface,
    margin: EdgeInsets.zero,
    shape: RoundedRectangleBorder(
      borderRadius: BorderRadius.circular(18),
      side: const BorderSide(color: line),
    ),
  ),
  navigationBarTheme: const NavigationBarThemeData(
    backgroundColor: navigation,
    indicatorColor: Color(0xff264234),
  ),
  dividerTheme: const DividerThemeData(color: line),
);

class Glyph extends StatelessWidget {
  final String name;
  final double size;
  final Color? color;
  const Glyph(
    this.name, {
    super.key,
    this.size = 20,
    this.color = const Color(0xffc5c5c5),
  });
  @override
  Widget build(BuildContext context) => SvgPicture.asset(
    'assets/icons/$name.svg',
    width: size,
    height: size,
    colorFilter: color == null
        ? null
        : ColorFilter.mode(color!, BlendMode.srcIn),
  );
}

class DeviceIcon extends StatelessWidget {
  final Device device;
  final Map<String, dynamic> icons;
  final double size;
  const DeviceIcon(
    this.device, {
    super.key,
    this.icons = const {},
    this.size = 36,
  });
  @override
  Widget build(BuildContext context) {
    final imported = icons[device.data['custom_icon']];
    if (imported is List) {
      return Image.memory(
        Uint8List.fromList(List<int>.from(imported)),
        width: size,
        height: size,
        errorBuilder: (_, _, _) => Glyph('server', size: size),
      );
    }
    return Glyph(
      deviceIcons[device.icon] ?? 'server',
      size: size,
      color: device.icon == 'Server' ? green : null,
    );
  }
}

class StatusDot extends StatelessWidget {
  final bool? online;
  const StatusDot(this.online, {super.key});
  @override
  Widget build(BuildContext context) => Tooltip(
    message: online == null
        ? 'Not checked'
        : online!
        ? 'Online'
        : 'Offline',
    child: Container(
      width: 8,
      height: 8,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        color: online == null
            ? Colors.grey
            : online!
            ? green
            : const Color(0xffe66e6e),
      ),
    ),
  );
}

class GlassPanel extends StatelessWidget {
  final Widget child;
  const GlassPanel({super.key, required this.child});
  @override
  Widget build(BuildContext context) => ClipRRect(
    borderRadius: BorderRadius.circular(20),
    child: BackdropFilter(
      filter: ImageFilter.blur(sigmaX: 18, sigmaY: 18),
      child: Container(
        decoration: BoxDecoration(
          color: navigation.withValues(alpha: .92),
          border: Border.all(color: line),
          borderRadius: BorderRadius.circular(20),
        ),
        padding: const EdgeInsets.all(20),
        child: child,
      ),
    ),
  );
}

Future<T?> showPanel<T>(BuildContext context, Widget child) =>
    showModalBottomSheet<T>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      backgroundColor: Colors.transparent,
      builder: (context) => Padding(
        padding: EdgeInsets.only(
          left: 8,
          right: 8,
          bottom: MediaQuery.viewInsetsOf(context).bottom + 8,
        ),
        child: ConstrainedBox(
          constraints: BoxConstraints(
            maxHeight: MediaQuery.sizeOf(context).height * .9,
            maxWidth: 600,
          ),
          child: GlassPanel(child: child),
        ),
      ),
    );
void message(BuildContext context, Object text) {
  if (context.mounted) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(text.toString()),
        behavior: SnackBarBehavior.floating,
      ),
    );
  }
}

Future<bool> confirm(
  BuildContext context,
  String title,
  String body, {
  String action = 'Continue',
}) async =>
    await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(title),
        content: SingleChildScrollView(child: Text(body)),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: Text(action),
          ),
        ],
      ),
    ) ??
    false;
const avatarNames = ['Alex', 'Sam', 'Jordan', 'Morgan', 'Casey'];

class Avatar extends StatelessWidget {
  final int index;
  const Avatar(this.index, {super.key});
  @override
  Widget build(BuildContext context) => CircleAvatar(
    backgroundColor: [
      const Color(0xff405c52),
      const Color(0xff4c5069),
      const Color(0xff6b5249),
      const Color(0xff5a4c68),
      const Color(0xff4a606b),
    ][index.clamp(0, 4)],
    child: Text(
      avatarNames[index.clamp(0, 4)][0],
      style: const TextStyle(color: Colors.white),
    ),
  );
}
