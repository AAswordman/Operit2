// ignore_for_file: file_names

import 'dart:io';

import '../bridge/RemoteCoreProxy.dart';

class RuntimeDeviceInfoProvider {
  const RuntimeDeviceInfoProvider._();

  static Future<RemoteDeviceInfo> current() async {
    return RemoteDeviceInfo(
      platform: Platform.operatingSystem,
      model: Platform.localHostname,
    );
  }
}
