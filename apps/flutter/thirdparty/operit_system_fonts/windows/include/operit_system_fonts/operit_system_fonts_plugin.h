#pragma once
#include <flutter_plugin_registrar.h>
#ifdef FLUTTER_PLUGIN_IMPL
#define OPERIT_FONTS_EXPORT __declspec(dllexport)
#else
#define OPERIT_FONTS_EXPORT __declspec(dllimport)
#endif
#ifdef __cplusplus
extern "C" {
#endif
OPERIT_FONTS_EXPORT void OperitSystemFontsPluginRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar);
#ifdef __cplusplus
}
#endif
