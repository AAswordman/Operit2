//
//  Generated file. Do not edit.
//

// clang-format off

#include "generated_plugin_registrant.h"

#include <audioplayers_linux/audioplayers_linux_plugin.h>
#include <desktop_drop/desktop_drop_plugin.h>
#include <desktop_multi_window/desktop_multi_window_plugin.h>
#include <dynamic_color/dynamic_color_plugin.h>
#include <file_selector_linux/file_selector_plugin.h>
#include <printing/printing_plugin.h>
#include <record_linux/record_linux_plugin.h>
#include <url_launcher_linux/url_launcher_plugin.h>
#include <webview_all_linux/webview_all_linux_plugin.h>

void fl_register_plugins(FlPluginRegistry* registry) {
  g_autoptr(FlPluginRegistrar) audioplayers_linux_registrar =
      fl_plugin_registry_get_registrar_for_plugin(registry, "AudioplayersLinuxPlugin");
  audioplayers_linux_plugin_register_with_registrar(audioplayers_linux_registrar);
  g_autoptr(FlPluginRegistrar) desktop_drop_registrar =
      fl_plugin_registry_get_registrar_for_plugin(registry, "DesktopDropPlugin");
  desktop_drop_plugin_register_with_registrar(desktop_drop_registrar);
  g_autoptr(FlPluginRegistrar) desktop_multi_window_registrar =
      fl_plugin_registry_get_registrar_for_plugin(registry, "DesktopMultiWindowPlugin");
  desktop_multi_window_plugin_register_with_registrar(desktop_multi_window_registrar);
  g_autoptr(FlPluginRegistrar) dynamic_color_registrar =
      fl_plugin_registry_get_registrar_for_plugin(registry, "DynamicColorPlugin");
  dynamic_color_plugin_register_with_registrar(dynamic_color_registrar);
  g_autoptr(FlPluginRegistrar) file_selector_linux_registrar =
      fl_plugin_registry_get_registrar_for_plugin(registry, "FileSelectorPlugin");
  file_selector_plugin_register_with_registrar(file_selector_linux_registrar);
  g_autoptr(FlPluginRegistrar) printing_registrar =
      fl_plugin_registry_get_registrar_for_plugin(registry, "PrintingPlugin");
  printing_plugin_register_with_registrar(printing_registrar);
  g_autoptr(FlPluginRegistrar) record_linux_registrar =
      fl_plugin_registry_get_registrar_for_plugin(registry, "RecordLinuxPlugin");
  record_linux_plugin_register_with_registrar(record_linux_registrar);
  g_autoptr(FlPluginRegistrar) url_launcher_linux_registrar =
      fl_plugin_registry_get_registrar_for_plugin(registry, "UrlLauncherPlugin");
  url_launcher_plugin_register_with_registrar(url_launcher_linux_registrar);
  g_autoptr(FlPluginRegistrar) webview_all_linux_registrar =
      fl_plugin_registry_get_registrar_for_plugin(registry, "WebviewAllLinuxPlugin");
  webview_all_linux_plugin_register_with_registrar(webview_all_linux_registrar);
}
