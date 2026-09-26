#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct {
    int32_t x1;
    int32_t y1;
    int32_t x2;
    int32_t y2;
} operit_lvgl_area_t;

typedef void (*operit_lvgl_flush_cb_t)(const operit_lvgl_area_t *area,
                                       const uint8_t *pixels,
                                       size_t length,
                                       void *user_data);
typedef bool (*operit_lvgl_touch_cb_t)(uint16_t *x, uint16_t *y, void *user_data);
typedef void (*operit_lvgl_action_cb_t)(const char *action, void *user_data);

bool operit_lvgl_init(uint16_t width,
                      uint16_t height,
                      operit_lvgl_flush_cb_t flush_cb,
                      operit_lvgl_touch_cb_t touch_cb,
                      operit_lvgl_action_cb_t action_cb,
                      void *user_data);
void operit_lvgl_pump(uint32_t elapsed_ms);
void operit_lvgl_set_touch(uint16_t x, uint16_t y, bool pressed);
void operit_lvgl_navigate_home(void);
void operit_lvgl_set_connection(bool wifi_ready, bool edge_ready);
void operit_lvgl_set_expression(const char *expression);
void operit_lvgl_set_pairing_code(const char *code);
void operit_lvgl_set_space_state(const char *state);
void operit_lvgl_set_chat_preview(const char *preview);
void operit_lvgl_set_chat_screen(const char *text);
void operit_lvgl_set_chat_task(const char *text);
const char *operit_lvgl_chat_draft(void);
void operit_lvgl_set_chat_draft(const char *text);
void operit_lvgl_submit_chat(void);
void operit_lvgl_chat_send_result(bool ok, const char *error);

void operit_lvgl_set_theme(unsigned index, bool circular);
void operit_lvgl_navigate_apps(void);
unsigned operit_lvgl_theme_index(void);
bool operit_lvgl_round_icons(void);

const char *operit_lvgl_current_page(void);

/* Structured screen inspection used by the ESP32 editor and CLI test tools.
 * The returned strings are owned by the LVGL runtime and remain valid until
 * the next call into one of the debug JSON functions. */
const char *operit_lvgl_debug_tree(void);
const char *operit_lvgl_debug_snapshot(void);
bool operit_lvgl_debug_tap(const char *id);
bool operit_lvgl_debug_swipe(const char *direction);

void operit_lvgl_layout_clear(uint32_t background);
int operit_lvgl_layout_add(int type, int parent, int x, int y, int w, int h,
    uint32_t color, int radius, int value, const char *text, const char *action);
void operit_lvgl_layout_geometry(int index,int x,int y,int w,int h);
void operit_lvgl_layout_bind(int index,const char *action,const char *long_action);

void operit_lvgl_layout_page_meta(const char *id,const char *left,const char *right);
void operit_lvgl_layout_style(int index,const char *binding,int font_size);
