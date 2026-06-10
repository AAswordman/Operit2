// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

String toolPackageDisplayName(core_proxy.ToolPackage package) {
  final displayName = localizedText(package.displayName).trim();
  if (displayName.isNotEmpty) {
    return displayName;
  }
  return package.name;
}

String toolPkgContainerDisplayName(core_proxy.ToolPkgContainerRuntime plugin) {
  final displayName = localizedText(plugin.displayName).trim();
  if (displayName.isNotEmpty) {
    return displayName;
  }
  return plugin.packageName;
}

String bundledExternalPackageDisplayName(
  core_proxy.BundledExternalPackageCandidate package,
) {
  final displayName = localizedText(package.displayName).trim();
  if (displayName.isNotEmpty) {
    return displayName;
  }
  return package.packageName;
}

String toolPkgSubpackageDisplayName(
  core_proxy.ToolPkgSubpackageRuntime subpackage,
) {
  final displayName = localizedText(subpackage.displayName).trim();
  if (displayName.isNotEmpty) {
    return displayName;
  }
  return subpackage.packageName;
}

bool toolPkgHasUi(core_proxy.ToolPkgContainerRuntime plugin) {
  return plugin.uiModules.isNotEmpty || plugin.uiRoutes.isNotEmpty;
}

String localizedText(Object? value) {
  if (value == null) {
    return '';
  }
  if (value is String) {
    return value;
  }
  if (value is Map<Object?, Object?>) {
    final values = value['values'];
    if (values is Map<Object?, Object?>) {
      return _localizedTextFromMap(values);
    }
    return _localizedTextFromMap(value);
  }
  return value.toString();
}

int packageCategoryOrder(String category) {
  return switch (category) {
    'Automatic' => 0,
    'Experimental' => 1,
    'Draw' => 2,
    'Other' => 3,
    _ => 100,
  };
}

IconData packageCategoryIcon(String category) {
  return switch (category) {
    'Automatic' => Icons.auto_mode,
    'Experimental' => Icons.science_outlined,
    'Draw' => Icons.palette_outlined,
    _ => Icons.inventory_2_outlined,
  };
}

String _localizedTextFromMap(Map<Object?, Object?> values) {
  final zh = values['zh'];
  if (zh is String && zh.trim().isNotEmpty) {
    return zh;
  }
  final defaultValue = values['default'];
  if (defaultValue is String && defaultValue.trim().isNotEmpty) {
    return defaultValue;
  }
  final en = values['en'];
  if (en is String && en.trim().isNotEmpty) {
    return en;
  }
  for (final value in values.values) {
    if (value is String && value.trim().isNotEmpty) {
      return value;
    }
  }
  return '';
}
