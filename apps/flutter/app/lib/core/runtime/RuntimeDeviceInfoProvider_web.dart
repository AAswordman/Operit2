// ignore_for_file: file_names, deprecated_member_use, avoid_web_libraries_in_flutter

import 'dart:html' as html;

import '../link/RemoteRuntimeLinkClient.dart';

class RuntimeDeviceInfoProvider {
  const RuntimeDeviceInfoProvider._();

  static Future<RemoteDeviceInfo> current() async {
    final navigator = html.window.navigator;
    final userAgent = navigator.userAgent;
    final platform = navigator.platform;
    if (platform == null) {
      throw FormatException('browser platform is not available');
    }
    final browser = _browserName(userAgent);
    return RemoteDeviceInfo(platform: platform, model: browser);
  }
}

String _browserName(String userAgent) {
  final match = RegExp(
    r'(Edg|OPR|Firefox|Chrome|CriOS|FxiOS|Version)/([0-9]+)',
  ).firstMatch(userAgent);
  if (match == null) {
    throw FormatException('browser name is not available in userAgent');
  }
  final name = switch (match.group(1)!) {
    'Edg' => 'Edge',
    'OPR' => 'Opera',
    'CriOS' => 'Chrome iOS',
    'FxiOS' => 'Firefox iOS',
    'Version' => 'Safari',
    final value => value,
  };
  return '$name ${match.group(2)!}';
}
