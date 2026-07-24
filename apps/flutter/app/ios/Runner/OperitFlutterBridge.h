#ifndef OperitFlutterBridge_h
#define OperitFlutterBridge_h

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

void *operit_flutter_bridge_create(void);
char *operit_ios_native_runtime_call(const char *command, const char *request_json);
void operit_ios_native_runtime_free(char *value);
void *operit_flutter_bridge_create_with_storage_roots(
    const char *runtime_root,
    const char *workspace_root);
char *operit_flutter_bridge_create_error(void);
void operit_flutter_bridge_destroy(void *handle);
typedef struct OperitByteBuffer {
    uint8_t *ptr;
    uintptr_t len;
} OperitByteBuffer;
OperitByteBuffer operit_flutter_bridge_native_call(void *handle, const uint8_t *request_ptr, uintptr_t request_len);
OperitByteBuffer operit_flutter_bridge_push_open(void *handle, const uint8_t *request_ptr, uintptr_t request_len);
OperitByteBuffer operit_flutter_bridge_push_item(void *handle, const uint8_t *item_ptr, uintptr_t item_len);
OperitByteBuffer operit_flutter_bridge_push_close(void *handle, const char *push_id);
OperitByteBuffer operit_flutter_bridge_watch_snapshot(void *handle, const uint8_t *request_ptr, uintptr_t request_len);
OperitByteBuffer operit_flutter_bridge_watch_stream(void *handle, const uint8_t *request_ptr, uintptr_t request_len);
OperitByteBuffer operit_flutter_bridge_next_watch_channel_event(void *handle);
OperitByteBuffer operit_flutter_bridge_close_watch_stream(void *handle, const char *subscription_id);
void operit_flutter_bridge_free_bytes(OperitByteBuffer value);
char *operit_flutter_bridge_start_web_access_server(
    void *handle,
    const char *bind_address,
    const char *token,
    const char *shutdown_token,
    const char *web_root,
    const char *device_info_json,
    const char *enable_web_access,
    const char *enable_discovery
);
char *operit_flutter_bridge_stop_web_access_server(void *handle);
char *operit_flutter_bridge_emit_runtime_event(void *handle, const char *event_json);
void operit_flutter_bridge_free_string(char *value);

#ifdef __cplusplus
}
#endif

#endif
