#ifndef FLOATING_WINDOWS_LINUX_PLUGIN_H_
#define FLOATING_WINDOWS_LINUX_PLUGIN_H_
#include <flutter_linux/flutter_linux.h>
G_BEGIN_DECLS
#ifdef FLUTTER_PLUGIN_IMPL
#define FLOATING_WINDOWS_EXPORT __attribute__((visibility("default")))
#else
#define FLOATING_WINDOWS_EXPORT
#endif
/// Registers the widget host on the supplied engine, never a global window.
FLOATING_WINDOWS_EXPORT void floating_windows_plugin_register_with_registrar(
    FlPluginRegistrar* registrar);
G_END_DECLS
#endif
