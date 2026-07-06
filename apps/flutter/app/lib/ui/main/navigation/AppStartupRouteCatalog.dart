// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../features/FeatureStartupRoutes.dart';
import 'StartupRouteStrategy.dart';

class AppStartupRouteCatalog {
  const AppStartupRouteCatalog._();

  static StartupRouteRegistry build() {
    final registry = StartupRouteRegistry();
    FeatureStartupRoutes.registerAll(registry);
    return registry;
  }
}

class AppStartupRouteHost extends StatefulWidget {
  const AppStartupRouteHost({super.key});

  @override
  State<AppStartupRouteHost> createState() => _AppStartupRouteHostState();
}

class _AppStartupRouteHostState extends State<AppStartupRouteHost> {
  late final StartupRouteRegistry _registry = AppStartupRouteCatalog.build();

  @override
  Widget build(BuildContext context) {
    return StartupRouteHost(registry: _registry);
  }
}
