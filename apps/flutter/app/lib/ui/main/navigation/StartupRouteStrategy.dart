// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../screens/OperitMainScreen.dart';

typedef StartupRouteCompleteCallback = void Function();
typedef StartupRoutePageBuilder =
    Widget Function(BuildContext context, StartupRouteCompleteCallback complete);

abstract class StartupRouteStrategy {
  const StartupRouteStrategy();

  Future<StartupRouteDecision?> resolve();
}

class StartupRouteDecision {
  const StartupRouteDecision({required this.builder});

  final StartupRoutePageBuilder builder;
}

class StartupRouteRegistry {
  final List<StartupRouteStrategy> _strategies = <StartupRouteStrategy>[];

  void register(StartupRouteStrategy strategy) {
    for (final registered in _strategies) {
      if (registered.runtimeType == strategy.runtimeType) {
        return;
      }
    }
    _strategies.add(strategy);
  }

  List<StartupRouteStrategy> strategies() {
    return List<StartupRouteStrategy>.unmodifiable(_strategies);
  }
}

class StartupRouteHost extends StatefulWidget {
  const StartupRouteHost({super.key, required this.registry});

  final StartupRouteRegistry registry;

  @override
  State<StartupRouteHost> createState() => _StartupRouteHostState();
}

class _StartupRouteHostState extends State<StartupRouteHost> {
  late Future<StartupRouteDecision?> _decisionFuture;
  bool _startupRouteCompleted = false;

  @override
  void initState() {
    super.initState();
    _decisionFuture = _resolveStartupRoute();
  }

  @override
  void didUpdateWidget(covariant StartupRouteHost oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.registry != widget.registry) {
      _startupRouteCompleted = false;
      _decisionFuture = _resolveStartupRoute();
    }
  }

  Future<StartupRouteDecision?> _resolveStartupRoute() async {
    for (final strategy in widget.registry.strategies()) {
      final decision = await strategy.resolve();
      if (decision != null) {
        return decision;
      }
    }
    return null;
  }

  void _completeStartupRoute() {
    if (!mounted) {
      return;
    }
    setState(() {
      _startupRouteCompleted = true;
    });
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<StartupRouteDecision?>(
      future: _decisionFuture,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          Error.throwWithStackTrace(snapshot.error!, snapshot.stackTrace!);
        }
        if (snapshot.connectionState != ConnectionState.done) {
          return const StartupRouteShell();
        }
        if (_startupRouteCompleted) {
          return const OperitMainScreen();
        }
        final decision = snapshot.data;
        if (decision != null) {
          return decision.builder(context, _completeStartupRoute);
        }
        return const OperitMainScreen();
      },
    );
  }
}

class StartupRouteShell extends StatelessWidget {
  const StartupRouteShell({super.key});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: colorScheme.surface,
      child: const SizedBox.expand(),
    );
  }
}
