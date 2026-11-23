import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/host/RuntimeHostInteractionSubscriber.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/core/runtime/RuntimeBootstrapManager.dart';

/// Probes a real Termux installation from an explicitly launched Android build.
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const MaterialApp(home: Scaffold(body: Text('Termux device test'))));
  try {
    await ClientLogger.initialize();
    await RuntimeBootstrapManager.instance.initialize();
    RuntimeHostInteractionSubscriber.install();
    await _exerciseTermux();
    debugPrint('TERMUX_SMOKE_DONE PASS');
  } catch (error, stack) {
    debugPrint('TERMUX_SMOKE_DONE ERROR $error\n$stack');
  } finally {
    await RuntimeHostInteractionSubscriber.uninstall();
  }
}

/// Saves a temporary profile and exercises the same Core API used by the workspace.
Future<void> _exerciseTermux() async {
  const clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
  final terminal = clients.servicesRuntimeTerminalService;
  final previous = await terminal.terminalProfileSettings();
  final support = (await terminal.terminalProfileSupport()).singleWhere(
    (entry) => entry.terminal == 'termux',
  );
  final profile = TerminalProfile(
    id: 'device_termux_${DateTime.now().microsecondsSinceEpoch}',
    name: 'Termux device smoke',
    nameKey: null,
    terminal: support.terminal,
    terminalType: support.terminalType,
    options: const {'directoryMode': 'home'},
  );
  final installed = await terminal.saveTerminalProfileSettings(
    settings: TerminalProfileSettings(
      schemaVersion: previous.schemaVersion,
      revision: previous.revision,
      defaultProfileId: profile.id,
      profiles: [...previous.profiles, profile],
    ),
  );
  String? session;
  StreamSubscription<String>? subscription;
  try {
    final info = await terminal.terminalInfo();
    _require(info.terminal == 'profile:${profile.id}', 'default');
    final id = session = await terminal.startTerminalPty(
      sessionName: 'Termux device smoke',
      terminal: info.terminal,
      terminalType: profile.terminalType,
      workingDir: '/virtual/linux',
      rows: 24,
      cols: 80,
    );
    final output = StringBuffer();
    subscription = terminal.terminalPtyOutput(sessionId: id).listen((encoded) {
      output.write(utf8.decode(base64Decode(encoded), allowMalformed: true));
    });

    /// Forwards one complete input sequence to the workspace PTY.
    Future<void> write(String text) async {
      await terminal.writeTerminalPty(
        sessionId: id,
        dataBase64: base64Encode(utf8.encode(text)),
      );
    }

    /// Waits for a marker assembled by the shell and absent from command echo.
    Future<void> waitFor(String marker) => _until(
      () async => output.toString().contains(marker),
      () => 'Missing $marker in $output',
    );

    await write(
      r"printf '%s%s\n' SMOKE_ FIRST; sleep 2; printf '%s%s\n' SMOKE_ SECOND" '\r',
    );
    await waitFor('SMOKE_FIRST');
    _require(!output.toString().contains('SMOKE_SECOND'), 'streaming output');
    await waitFor('SMOKE_SECOND');
    debugPrint('TERMUX_SMOKE output arrived in two stages');
    await terminal.resizeTerminalPty(sessionId: id, rows: 31, cols: 101);
    await write(
      r"printf 'PTY='; stty size; printf 'UID='; id -u; printf 'DIR='; pwd; "
      r"printf '%s%s\n' READY_ INPUT; read answer; echo ANSWER=$answer" '\r',
    );
    await waitFor('PTY=31 101');
    await waitFor('READY_INPUT');
    await write('interactive\r');
    await waitFor('ANSWER=interactive');
    final identity = RegExp(r'UID=([0-9]+)').firstMatch(output.toString());
    _require(identity != null && int.parse(identity.group(1)!) >= 10000, 'app UID');
    _require(output.toString().contains('DIR=/data/data/com.termux/files/home'), 'home');
    debugPrint('TERMUX_SMOKE PTY size, input and app UID ${identity!.group(1)}');

    await write(r"printf '%s%s\n' INTERRUPT_ READY; sleep 20" '\r');
    await waitFor('INTERRUPT_READY');
    await write('\x03');
    await write('exit 37\r');
    int? status;
    await _until(
      () async => (status = await terminal.pollTerminalPtyExit(sessionId: id)) != null,
      () => 'Interrupted shell did not exit',
    );
    _require(status == 37, 'exit 37, received $status');
    debugPrint('TERMUX_SMOKE Ctrl-C and exit 37');
  } finally {
    await subscription?.cancel();
    try {
      if (session != null) await terminal.closeTerminalPty(sessionId: session);
    } finally {
      final current = await terminal.terminalProfileSettings();
      if (current.revision == installed.revision) {
        await terminal.saveTerminalProfileSettings(
          settings: TerminalProfileSettings(
            schemaVersion: previous.schemaVersion,
            revision: current.revision,
            defaultProfileId: previous.defaultProfileId,
            profiles: previous.profiles,
          ),
        );
      }
    }
  }
}

/// Bounds each observation so an unresponsive device cannot hang the probe.
Future<void> _until(
  Future<bool> Function() ready,
  String Function() describeFailure,
) async {
  final watch = Stopwatch()..start();
  while (!await ready()) {
    if (watch.elapsed > const Duration(seconds: 7)) {
      throw StateError(describeFailure());
    }
    await Future<void>.delayed(const Duration(milliseconds: 50));
  }
}

/// Fails the probe even on builds with assertions disabled.
void _require(bool passed, String description) {
  if (!passed) throw StateError(description);
}
