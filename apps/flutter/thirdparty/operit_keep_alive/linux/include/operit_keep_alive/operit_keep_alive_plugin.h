#ifndef FLUTTER_PLUGIN_OPERIT_KEEP_ALIVE_PLUGIN_H_
#define FLUTTER_PLUGIN_OPERIT_KEEP_ALIVE_PLUGIN_H_
#include <flutter_linux/flutter_linux.h>
G_BEGIN_DECLS
#ifdef FLUTTER_PLUGIN_IMPL
#define OPERIT_KEEP_ALIVE_EXPORT __attribute__((visibility("default")))
#else
#define OPERIT_KEEP_ALIVE_EXPORT
#endif
/// Registers the per-engine transport without claiming the main window.
OPERIT_KEEP_ALIVE_EXPORT void operit_keep_alive_plugin_register_with_registrar(FlPluginRegistrar* registrar);
G_END_DECLS
#endif
