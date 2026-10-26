#ifndef OperitFlutterBridge_h
#define OperitFlutterBridge_h

#include <stdint.h>
#include <stddef.h>

void *operit_flutter_bridge_create(void);
void *operit_flutter_bridge_create_with_storage_roots(
    const char *runtime_root,
    const char *workspace_root);
char *operit_flutter_bridge_create_error(void);
/// Executes independent data maintenance without constructing Core.
char *operit_flutter_bridge_data_rescue(const char *default_runtime_root, const char *request);
/// Reads the client bootstrap record before the native Runtime is created.
char *operit_flutter_bridge_runtime_bootstrap_read(const char *default_runtime_root);
/// Writes the client bootstrap record before the native Runtime is created.
char *operit_flutter_bridge_runtime_bootstrap_write(
    const char *default_runtime_root,
    const char *content);
/// Creates a retained direct Dart FFI connection.
char *operit_flutter_bridge_ffi_connect(void *handle);
void operit_flutter_bridge_destroy(void *handle);
char *operit_flutter_bridge_emit_runtime_event(void *handle, const char *event_json);
void operit_flutter_bridge_free_string(char *value);

#endif
