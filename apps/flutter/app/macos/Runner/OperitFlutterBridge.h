#ifndef OperitFlutterBridge_h
#define OperitFlutterBridge_h

#include <stdint.h>
#include <stddef.h>

void *operit_flutter_bridge_create(void);
void *operit_flutter_bridge_create_with_storage_root(const char *storage_root);
char *operit_flutter_bridge_create_error(void);
void operit_flutter_bridge_destroy(void *handle);
char *operit_flutter_bridge_call(void *handle, const uint8_t *request_ptr, uintptr_t request_len);
char *operit_flutter_bridge_watch_snapshot(void *handle, const uint8_t *request_ptr, uintptr_t request_len);
char *operit_flutter_bridge_watch_stream(void *handle, const uint8_t *request_ptr, uintptr_t request_len);
char *operit_flutter_bridge_next_watch_channel_event(void *handle);
char *operit_flutter_bridge_close_watch_stream(void *handle, const char *subscription_id);
char *operit_flutter_bridge_start_web_access_server(
    void *handle,
    const char *bind_address,
    const char *token,
    const char *shutdown_token,
    const char *web_root,
    const char *device_id,
    const char *accepted_sessions_json,
    const char *accepted_session_store_path,
    const char *pairing_code_path,
    const char *device_info_json,
    const char *enable_web_access,
    const char *enable_discovery
);
char *operit_flutter_bridge_discover_devices(void *handle, const char *timeout_ms);
char *operit_flutter_bridge_stop_web_access_server(void *handle);
char *operit_flutter_bridge_remote_pair_start(
    void *handle,
    const char *base_url,
    const char *token_hash,
    const char *client_device_info_json
);
char *operit_flutter_bridge_remote_pair_finish(
    void *handle,
    const char *pairing_id,
    const char *pairing_code
);
char *operit_flutter_bridge_emit_runtime_event(void *handle, const char *event_json);
void operit_flutter_bridge_free_string(char *value);

#endif
