#ifndef FLUTTER_PLUGIN_FLOATING_WINDOWS_PLUGIN_H_
#define FLUTTER_PLUGIN_FLOATING_WINDOWS_PLUGIN_H_
#include <flutter_plugin_registrar.h>

#ifdef FLUTTER_PLUGIN_IMPL
#define FLOATING_WINDOWS_EXPORT __declspec(dllexport)
#else
#define FLOATING_WINDOWS_EXPORT __declspec(dllimport)
#endif

#if defined(__cplusplus)
extern "C" {
#endif
/// Registers the transparent widget surface host for a Flutter engine.
FLOATING_WINDOWS_EXPORT void FloatingWindowsPluginRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar);
#if defined(__cplusplus)
}
#endif
#endif
