import 'dart:convert';
import 'dart:io';

import 'package:cryptography/cryptography.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:path_provider/path_provider.dart';

import 'device.dart';
import 'transfer.dart';

const recoveryQuestions = [
  'First school',
  'Childhood nickname',
  'A memorable teacher',
  'First pet',
  'Parents’ meeting city',
  'A memorable childhood place',
];
final _cipher = AesGcm.with256bits();

Future<String> passwordHash(String value, List<int> salt) async {
  return compute(_derivePassword, {'value': value, 'salt': salt});
}

Future<String> _derivePassword(Map<String, dynamic> parameters) async {
  final value = parameters['value'] as String,
      salt = List<int>.from(parameters['salt'] as List);
  final algorithm = Pbkdf2(
    macAlgorithm: Hmac.sha256(),
    iterations: 210000,
    bits: 256,
  );
  return base64Encode(
    await (await algorithm.deriveKey(
      secretKey: SecretKey(utf8.encode(value)),
      nonce: salt,
    )).extractBytes(),
  );
}

bool equalHash(String a, String b) {
  final first = utf8.encode(a), second = utf8.encode(b);
  var difference = first.length ^ second.length;
  for (var i = 0; i < first.length && i < second.length; i++) {
    difference |= first[i] ^ second[i];
  }
  return difference == 0;
}

Future<Uint8List> sealJson(Map<String, dynamic> value, SecretKey key) async {
  final box = await _cipher.encrypt(
    utf8.encode(jsonEncode(value)),
    secretKey: key,
  );
  return Uint8List.fromList(box.concatenation());
}

Future<Map<String, dynamic>> openJson(List<int> bytes, SecretKey key) async {
  final box = SecretBox.fromConcatenation(
    bytes,
    nonceLength: 12,
    macLength: 16,
  );
  return Map<String, dynamic>.from(
    jsonDecode(utf8.decode(await _cipher.decrypt(box, secretKey: key))) as Map,
  );
}

class LocalVault extends ChangeNotifier {
  final FlutterSecureStorage secure;
  final Future<Directory> Function() directory;
  LocalVault({
    FlutterSecureStorage? secure,
    Future<Directory> Function()? directory,
  }) : secure = secure ?? const FlutterSecureStorage(),
       directory = directory ?? getApplicationSupportDirectory;
  SecretKey? _key;
  late File _file;
  Map<String, dynamic> _data = {'accounts': <String, dynamic>{}};
  String? username;
  bool ready = false;
  bool _disposed = false;
  Future<void> _pending = Future.value();
  Map<String, dynamic> get _accounts =>
      Map<String, dynamic>.from(_data['accounts'] as Map);
  Map<String, dynamic> get account =>
      Map<String, dynamic>.from(_accounts[username] as Map);
  List<Device> get devices => username == null
      ? []
      : (account['devices'] as List)
            .map((v) => Device(Map<String, dynamic>.from(v as Map)))
            .toList();
  Map<String, dynamic> get monitoring =>
      Map<String, dynamic>.from(account['monitoring'] as Map? ?? {});
  Map<String, dynamic> get icons =>
      Map<String, dynamic>.from(account['icons'] as Map? ?? {});
  int get avatar => username == null ? 0 : account['avatar'] as int? ?? 0;
  bool get hasAccounts => _accounts.isNotEmpty;
  List<String> get usernames => _accounts.keys.toList()..sort();

  Future<void> load() async {
    final root = await directory();
    await root.create(recursive: true);
    _file = File('${root.path}/mobile-vault.enc');
    final existing = await _file.exists();
    var encoded = await secure.read(key: 'lab-manager-mobile-key-v1');
    if (encoded == null) {
      if (existing) {
        throw StateError(
          'The encryption key is missing. Existing data was preserved.',
        );
      }
      final key = await _cipher.newSecretKey();
      encoded = base64Encode(await key.extractBytes());
      await secure.write(key: 'lab-manager-mobile-key-v1', value: encoded);
    }
    _key = SecretKey(base64Decode(encoded));
    if (existing) {
      if (await _file.length() > 48 * 1024 * 1024) {
        throw StateError('Invalid encrypted vault size.');
      }
      _data = await openJson(await _file.readAsBytes(), _key!);
      if (_data['accounts'] is! Map) {
        throw StateError('Invalid encrypted account data.');
      }
    }
    final remembered = await secure.read(key: 'lab-manager-mobile-session-v1');
    if (remembered != null && _accounts.containsKey(remembered)) {
      username = remembered;
    }
    ready = true;
    notifyListeners();
  }

  Future<void> _save(Map<String, dynamic> data) async {
    final encrypted = await sealJson(data, _key!);
    final temporary = File('${_file.path}.tmp');
    await temporary.writeAsBytes(encrypted, flush: true);
    await temporary.rename(_file.path);
  }

  Future<void> _transaction(void Function(Map<String, dynamic>) change) {
    final operation = _pending.then((_) async {
      final accounts = _accounts;
      change(accounts);
      final next = {'accounts': accounts};
      await _save(next);
      _data = next;
      if (!_disposed) notifyListeners();
    });
    _pending = operation.catchError((Object _) {});
    return operation;
  }

  Future<void> _edit(String user, void Function(Map<String, dynamic>) change) {
    return _transaction((accounts) {
      final value = Map<String, dynamic>.from(accounts[user] as Map? ?? {});
      change(value);
      accounts[user] = value;
    });
  }

  Future<void> _replace(String user, Map<String, dynamic> patch) =>
      _edit(user, (value) => value.addAll(patch));

  Future<void> register(
    String user,
    String password,
    List<int> questions,
    List<String> answers,
    bool remember,
    int avatar,
  ) async {
    user = user.trim().toLowerCase();
    if (!RegExp(r'^[a-z0-9_.-]{1,64}$').hasMatch(user)) {
      throw const FormatException(
        'Use 1–64 letters, digits, dots, underscores or hyphens.',
      );
    }
    if (password.length < 12 || password.length > 256) {
      throw const FormatException('Use a password of 12–256 characters.');
    }
    if (_accounts.containsKey(user)) {
      throw const FormatException('That account already exists.');
    }
    if (questions.length != 2 ||
        questions[0] == questions[1] ||
        answers.any((a) => a.trim().length < 3)) {
      throw const FormatException(
        'Choose two different questions and answers of at least 3 characters.',
      );
    }
    final salt = await (await _cipher.newSecretKey()).extractBytes();
    await _replace(user, {
      'salt': base64Encode(salt),
      'password': await passwordHash(password, salt),
      'questions': questions,
      'answers': await Future.wait(
        answers.map(
          (answer) => passwordHash(answer.trim().toLowerCase(), salt),
        ),
      ),
      'avatar': avatar,
      'devices': [],
      'monitoring': {},
      'icons': {},
      'trust': {},
      'credentials': {},
      'failures': 0,
      'blocked': 0,
    });
    await signIn(user, password, remember);
  }

  Future<void> signIn(String user, String password, bool remember) async {
    user = user.trim().toLowerCase();
    final raw = _accounts[user];
    if (raw is! Map) {
      throw const FormatException('Incorrect username or password.');
    }
    final value = Map<String, dynamic>.from(raw);
    final now = DateTime.now().millisecondsSinceEpoch;
    if ((value['blocked'] as int? ?? 0) > now) {
      throw const FormatException(
        'Too many attempts. Try again in one minute.',
      );
    }
    if (!equalHash(
      await passwordHash(password, base64Decode(value['salt'] as String)),
      value['password'] as String,
    )) {
      final failures = (value['failures'] as int? ?? 0) + 1;
      value['failures'] = failures;
      if (failures >= 5) {
        value['blocked'] = now + 60000;
        value['failures'] = 0;
      }
      await _replace(user, {
        'failures': value['failures'],
        'blocked': value['blocked'],
      });
      throw const FormatException('Incorrect username or password.');
    }
    value['failures'] = 0;
    value['blocked'] = 0;
    await _replace(user, {'failures': 0, 'blocked': 0});
    if (remember) {
      await secure.write(key: 'lab-manager-mobile-session-v1', value: user);
    } else {
      await secure.delete(key: 'lab-manager-mobile-session-v1');
    }
    username = user;
    notifyListeners();
  }

  List<int> questions(String user) => List<int>.from(
    (_accounts[user.trim().toLowerCase()] as Map?)?['questions'] as List? ?? [],
  );
  Future<void> recover(
    String user,
    List<String> answers,
    String password,
  ) async {
    user = user.trim().toLowerCase();
    final value = Map<String, dynamic>.from(_accounts[user] as Map? ?? {});
    if (value.isEmpty) throw const FormatException('Account not found.');
    if ((value['blocked'] as int? ?? 0) >
        DateTime.now().millisecondsSinceEpoch) {
      throw const FormatException(
        'Too many attempts. Try again in one minute.',
      );
    }
    if (password.length < 12 || password.length > 256) {
      throw const FormatException('Use a password of 12–256 characters.');
    }
    final salt = base64Decode(value['salt'] as String);
    final expected = List<String>.from(value['answers'] as List);
    final actual = await Future.wait(
      answers.map((answer) => passwordHash(answer.trim().toLowerCase(), salt)),
    );
    if (actual.length != 2 ||
        !equalHash(actual[0], expected[0]) ||
        !equalHash(actual[1], expected[1])) {
      value['blocked'] = DateTime.now().millisecondsSinceEpoch + 60000;
      await _replace(user, {'blocked': value['blocked']});
      throw const FormatException(
        'Recovery answers did not match. Try again in one minute.',
      );
    }
    value['password'] = await passwordHash(password, salt);
    value['blocked'] = 0;
    await _replace(user, {'password': value['password'], 'blocked': 0});
    await secure.delete(key: 'lab-manager-mobile-session-v1');
  }

  Future<void> signOut() async {
    await secure.delete(key: 'lab-manager-mobile-session-v1');
    username = null;
    notifyListeners();
  }

  Future<void> removeAccount() async {
    final user = username;
    await _transaction((accounts) => accounts.remove(user));
    await signOut();
  }

  Future<void> saveDevices(List<Device> devices) async {
    final addresses = <String>{};
    for (final device in devices) {
      device.validate();
      if (!addresses.add(InternetAddress(device.ip).address)) {
        throw const FormatException(
          'A device with that address already exists.',
        );
      }
    }
    await _replace(username!, {'devices': devices.map((d) => d.data).toList()});
  }

  Future<void> importBackup(BackupData backup, {int? avatar}) async {
    await _edit(username!, (value) {
      final merged = {
        for (final raw in value['devices'] as List) (raw as Map)['ip']: raw,
        for (final d in backup.devices) d.ip: d.data,
      };
      if (merged.length > 4096) {
        throw const FormatException('Too many devices.');
      }
      value['devices'] = merged.values.toList();
      value['monitoring'] = {
        ...Map<String, dynamic>.from(value['monitoring'] as Map? ?? {}),
        ...backup.monitoring,
      };
      value['icons'] = {
        ...Map<String, dynamic>.from(value['icons'] as Map? ?? {}),
        ...backup.icons,
      };
      if (avatar != null) value['avatar'] = avatar;
    });
  }

  BackupData backup() {
    final current = devices;
    final keys = current.map((d) => d.monitorKey).toSet();
    final retained = {
      for (final entry in monitoring.entries)
        if (keys.contains(entry.key)) entry.key: entry.value,
    };
    return BackupData(current, retained, icons);
  }

  String? trusted(String endpoint) =>
      (account['trust'] as Map? ?? {})[endpoint] as String?;
  Future<void> trust(String endpoint, String fingerprint) async {
    await _edit(username!, (value) {
      value['trust'] = {
        ...Map<String, dynamic>.from(value['trust'] as Map? ?? {}),
        endpoint: fingerprint,
      };
    });
  }

  Map<String, dynamic>? credential(String endpoint) {
    final value = (account['credentials'] as Map? ?? {})[endpoint];
    return value is Map ? Map<String, dynamic>.from(value) : null;
  }

  Future<void> saveCredential(
    String endpoint,
    Map<String, dynamic>? credential,
  ) async {
    await _edit(username!, (value) {
      final credentials = Map<String, dynamic>.from(
        value['credentials'] as Map? ?? {},
      );
      if (credential == null) {
        credentials.remove(endpoint);
      } else {
        credentials[endpoint] = credential;
      }
      value['credentials'] = credentials;
    });
  }

  Future<void> clearSsh() =>
      _replace(username!, {'credentials': {}, 'trust': {}});

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }
}
