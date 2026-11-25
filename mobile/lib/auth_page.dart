import 'package:flutter/material.dart';

import 'vault.dart';
import 'ui.dart';

class AuthPage extends StatefulWidget {
  final LocalVault vault;
  const AuthPage(this.vault, {super.key});
  @override
  State<AuthPage> createState() => _AuthPageState();
}

class _AuthPageState extends State<AuthPage> {
  final _user = TextEditingController(),
      _password = TextEditingController(),
      _repeat = TextEditingController();
  final _answers = [TextEditingController(), TextEditingController()];
  int _mode = 0, _avatar = 0;
  final _questions = [0, 1];
  bool _remember = false, _busy = false;
  String? _error;
  @override
  void dispose() {
    for (final c in [_user, _password, _repeat, ..._answers]) {
      c.dispose();
    }
    super.dispose();
  }

  Future<void> _submit() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      if (_mode != 0 && _password.text != _repeat.text) {
        throw const FormatException('Passwords do not match.');
      }
      if (_mode == 0) {
        await widget.vault.signIn(_user.text, _password.text, _remember);
      } else if (_mode == 1) {
        await widget.vault.register(
          _user.text,
          _password.text,
          _questions,
          _answers.map((c) => c.text).toList(),
          _remember,
          _avatar,
        );
      } else {
        await widget.vault.recover(
          _user.text,
          _answers.map((c) => c.text).toList(),
          _password.text,
        );
        if (mounted) {
          setState(() => _mode = 0);
          message(context, 'Password reset. Sign in with your new password.');
        }
      }
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final recovery = widget.vault.questions(_user.text);
    return Scaffold(
      appBar: AppBar(
        title: const Row(
          children: [
            Glyph('server', color: green),
            SizedBox(width: 10),
            Text('Lab Manager'),
          ],
        ),
      ),
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 480),
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(24),
            child: Card(
              child: Padding(
                padding: const EdgeInsets.all(22),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Text(switch (_mode) {
                      0 => 'Welcome back',
                      1 => 'Create your workspace',
                      _ => 'Recover your account',
                    }, style: Theme.of(context).textTheme.headlineSmall),
                    const SizedBox(height: 8),
                    const Text(
                      'Your devices. Your network. Stored on this device.',
                      style: TextStyle(color: Colors.grey),
                    ),
                    const SizedBox(height: 24),
                    TextField(
                      controller: _user,
                      enabled: !_busy,
                      autocorrect: false,
                      onChanged: (_) => setState(() {}),
                      decoration: const InputDecoration(
                        labelText: 'Username',
                        prefixIcon: Padding(
                          padding: EdgeInsets.all(14),
                          child: Glyph('user'),
                        ),
                      ),
                    ),
                    const SizedBox(height: 14),
                    if (_mode == 2) ...[
                      if (recovery.length != 2)
                        const Padding(
                          padding: EdgeInsets.only(bottom: 12),
                          child: Text(
                            'Enter your account username to see its recovery questions.',
                          ),
                        ),
                      for (var i = 0; i < recovery.length; i++)
                        Padding(
                          padding: const EdgeInsets.only(bottom: 14),
                          child: TextField(
                            controller: _answers[i],
                            obscureText: true,
                            decoration: InputDecoration(
                              labelText: recoveryQuestions[recovery[i]],
                            ),
                          ),
                        ),
                    ],
                    TextField(
                      controller: _password,
                      enabled: !_busy,
                      obscureText: true,
                      autocorrect: false,
                      enableSuggestions: false,
                      onSubmitted: (_) {
                        if (_mode == 0 && !_busy) _submit();
                      },
                      decoration: InputDecoration(
                        labelText: _mode == 2 ? 'New password' : 'Password',
                        hintText: _mode == 0 ? null : 'At least 12 characters',
                        prefixIcon: const Padding(
                          padding: EdgeInsets.all(14),
                          child: Glyph('lock'),
                        ),
                      ),
                    ),
                    if (_mode != 0) ...[
                      const SizedBox(height: 14),
                      TextField(
                        controller: _repeat,
                        enabled: !_busy,
                        obscureText: true,
                        autocorrect: false,
                        decoration: const InputDecoration(
                          labelText: 'Confirm password',
                        ),
                      ),
                    ],
                    if (_mode == 1) ...[
                      const SizedBox(height: 22),
                      const Text('Profile avatar'),
                      const SizedBox(height: 12),
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: List.generate(
                          5,
                          (i) => Semantics(
                            label: 'Avatar ${i + 1}',
                            selected: _avatar == i,
                            button: true,
                            child: InkWell(
                              onTap: () => setState(() => _avatar = i),
                              borderRadius: BorderRadius.circular(28),
                              child: Container(
                                padding: const EdgeInsets.all(3),
                                decoration: BoxDecoration(
                                  shape: BoxShape.circle,
                                  border: Border.all(
                                    color: _avatar == i
                                        ? green
                                        : Colors.transparent,
                                    width: 2,
                                  ),
                                ),
                                child: Avatar(i),
                              ),
                            ),
                          ),
                        ),
                      ),
                      const SizedBox(height: 22),
                      const Text('Account recovery'),
                      const SizedBox(height: 8),
                      for (var i = 0; i < 2; i++) ...[
                        DropdownButtonFormField<int>(
                          initialValue: _questions[i],
                          isExpanded: true,
                          decoration: InputDecoration(
                            labelText: 'Security question ${i + 1}',
                          ),
                          items: List.generate(
                            recoveryQuestions.length,
                            (j) => DropdownMenuItem(
                              value: j,
                              child: Text(recoveryQuestions[j]),
                            ),
                          ),
                          onChanged: (v) =>
                              setState(() => _questions[i] = v ?? i),
                        ),
                        const SizedBox(height: 10),
                        TextField(
                          controller: _answers[i],
                          obscureText: true,
                          decoration: const InputDecoration(
                            labelText: 'Your answer',
                          ),
                        ),
                        const SizedBox(height: 14),
                      ],
                    ],
                    if (_mode != 2)
                      CheckboxListTile(
                        contentPadding: EdgeInsets.zero,
                        value: _remember,
                        onChanged: _busy
                            ? null
                            : (v) => setState(() => _remember = v ?? false),
                        title: const Text('Automatically sign in'),
                        subtitle: const Text(
                          'Only enable on a trusted personal device.',
                        ),
                      ),
                    if (_error != null)
                      Padding(
                        padding: const EdgeInsets.symmetric(vertical: 12),
                        child: Text(
                          _error!,
                          style: TextStyle(
                            color: Theme.of(context).colorScheme.error,
                          ),
                        ),
                      ),
                    const SizedBox(height: 12),
                    FilledButton(
                      onPressed: _busy ? null : _submit,
                      child: Padding(
                        padding: const EdgeInsets.all(12),
                        child: _busy
                            ? const SizedBox(
                                width: 20,
                                height: 20,
                                child: CircularProgressIndicator(
                                  strokeWidth: 2,
                                ),
                              )
                            : Text(switch (_mode) {
                                0 => 'Sign in',
                                1 => 'Create account',
                                _ => 'Reset password',
                              }),
                      ),
                    ),
                    const SizedBox(height: 10),
                    TextButton(
                      onPressed: _busy
                          ? null
                          : () => setState(() {
                              _mode = _mode == 1 ? 0 : 1;
                              _error = null;
                            }),
                      child: Text(
                        _mode == 1
                            ? 'Already have an account? Sign in'
                            : 'Create a local account',
                      ),
                    ),
                    TextButton(
                      onPressed: _busy
                          ? null
                          : () => setState(() {
                              _mode = _mode == 2 ? 0 : 2;
                              _error = null;
                            }),
                      child: Text(
                        _mode == 2
                            ? 'Back to sign in'
                            : 'Forgot your password?',
                      ),
                    ),
                    const SizedBox(height: 12),
                    const Text(
                      'Account records and inventory are encrypted. Desktop login credentials are not transferred.',
                      style: TextStyle(color: Colors.grey, fontSize: 12),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
