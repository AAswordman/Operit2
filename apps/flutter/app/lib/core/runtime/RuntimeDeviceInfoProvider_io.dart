// ignore_for_file: file_names

import 'dart:io';

import '../proxy/generated/CoreProxyModels.g.dart' as generated;

class RuntimeDeviceInfoProvider {
  const RuntimeDeviceInfoProvider._();

  static Future<generated.RemoteDeviceInfo> current() async {
    return generated.RemoteDeviceInfo(
      platform: Platform.operatingSystem,
      model: Platform.localHostname,
    );
  }
}
