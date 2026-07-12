#include "webview_all_windows/webview_windows_plugin.h"

#include <flutter/plugin_registrar_windows.h>

#include "plugin/windows_host_api.h"

void WebviewWindowsPluginRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar) {
  webview_all_windows::WindowsHostApi::RegisterWithRegistrar(
      flutter::PluginRegistrarManager::GetInstance()
          ->GetRegistrar<flutter::PluginRegistrarWindows>(registrar));
}
