#ifndef FLUTTER_PLUGIN_OPERIT_KEEP_ALIVE_PLUGIN_H_
#define FLUTTER_PLUGIN_OPERIT_KEEP_ALIVE_PLUGIN_H_
#include <flutter_plugin_registrar.h>
#ifdef FLUTTER_PLUGIN_IMPL
#define OPERIT_KEEP_ALIVE_EXPORT __declspec(dllexport)
#else
#define OPERIT_KEEP_ALIVE_EXPORT __declspec(dllimport)
#endif
#ifdef __cplusplus
extern "C" {
#endif
/// Registers a per-engine transport; only explicit initialization claims a tray icon.
OPERIT_KEEP_ALIVE_EXPORT void OperitKeepAlivePluginRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar);
#ifdef __cplusplus
}
#endif
#endif
