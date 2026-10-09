#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct {
    int32_t x1;
    int32_t y1;
    int32_t x2;
    int32_t y2;
} operit_ui_area_t;

typedef void (*operit_ui_flush_cb_t)(const operit_ui_area_t *area,
                                       const uint8_t *pixels,
                                       size_t length,
                                       void *user_data);
typedef bool (*operit_ui_touch_cb_t)(uint16_t *x, uint16_t *y, void *user_data);
typedef void (*operit_ui_action_cb_t)(const char *action, void *user_data);

bool operit_ui_init(uint16_t width,
                      uint16_t height,
                      operit_ui_flush_cb_t flush_cb,
                      operit_ui_touch_cb_t touch_cb,
                      operit_ui_action_cb_t action_cb,
                      void *user_data);
/* External bounded strip painter. It may only modify its leased region (y >= 24). */
typedef void (*operit_ui_scene_cb_t)(uint8_t *pixels, size_t length, unsigned y, unsigned rows, uint32_t tick, void *user_data);
void operit_ui_scene_painter(operit_ui_scene_cb_t painter);
void operit_ui_set_scene(bool active, uint32_t revision, uint32_t tick);
void operit_ui_pump(uint32_t elapsed_ms);
void operit_ui_set_touch(uint16_t x, uint16_t y, bool pressed);
void operit_ui_navigate_home(void);
void operit_ui_set_connection(bool wifi_ready, bool edge_ready);
void operit_ui_set_paired(bool paired);
void operit_ui_set_expression(const char *expression);
void operit_ui_set_pairing_code(const char *code);
void operit_ui_set_space_state(const char *state);
void operit_ui_set_space_join_prompt(const char *text, bool busy);
void operit_ui_set_chat_preview(const char *preview);
void operit_ui_set_chat_screen(const char *text);
void operit_ui_set_chat_identity(const char *id, const char *character);
void operit_ui_set_message(unsigned index, bool user, const char *text);
void operit_ui_set_chat_history(bool older, bool newer);
void operit_ui_finish_messages(unsigned count);
void operit_ui_set_conversation(unsigned index, const char *id, const char *title, const char *character, bool selected);
void operit_ui_finish_conversations(unsigned count);
void operit_ui_action_error(const char *error);
void operit_ui_set_chat_task(const char *text);
void operit_ui_set_plugin(unsigned index, const char *id, const char *name, unsigned status, unsigned latency_ms);
void operit_ui_set_plugin_test(unsigned index,unsigned status,unsigned ms,const char *error);
void operit_ui_set_plugin_testing(bool testing);
void operit_ui_set_plugin_category(bool exclusive);
void operit_ui_set_plugin_details(const char *id, const char *description, const char *tools,
    unsigned offset, unsigned total, bool loading, const char *error);
void operit_ui_finish_plugins(unsigned count, unsigned offset, unsigned total, bool loading, const char *error);
const char *operit_ui_chat_draft(void);
void operit_ui_set_chat_draft(const char *text);
void operit_ui_submit_chat(void);
void operit_ui_chat_send_result(bool ok, const char *error);

void operit_ui_set_theme(unsigned index, bool circular);
/* Emoji style IDs are defined in operit_emoji.h. Invalid IDs are rejected. */
bool operit_ui_set_emoji_style(unsigned style);
unsigned operit_ui_emoji_style(void);
void operit_ui_navigate_apps(void);
unsigned operit_ui_theme_index(void);
bool operit_ui_round_icons(void);

const char *operit_ui_current_page(void);

/* Structured screen inspection used by the ESP32 editor and CLI test tools.
 * The returned strings are owned by the UI renderer and remain valid until
 * the next call into one of the debug JSON functions. */
const char *operit_ui_debug_tree(void);
const char *operit_ui_debug_snapshot(void);
bool operit_ui_debug_tap(const char *id);
bool operit_ui_debug_swipe(const char *direction);

void operit_ui_layout_clear(uint32_t background);
int operit_ui_layout_add(int type, int parent, int x, int y, int w, int h,
    uint32_t color, int radius, int value, const char *text, const char *action);
void operit_ui_layout_geometry(int index,int x,int y,int w,int h);
void operit_ui_layout_bind(int index,const char *action,const char *long_action);

void operit_ui_layout_page_meta(const char *id,const char *left,const char *right);
void operit_ui_layout_style(int index,const char *binding,int font_size);
