#include "operit_lvgl.h"
#include "lvgl.h"
#include "esp_timer.h"
#include "layout_store.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <time.h>
#include "operit_font_zh_14.h"

/* A shared palette and geometry keep every component aligned at 320x240. */
typedef struct { uint32_t bg, surface, accent, muted; const char *name; } theme_t;
static const theme_t themes[] = {
    {0x091420, 0x172a3d, 0x53dfc5, 0x96abbc, "Aurora"},
    {0x20121d, 0x382436, 0xffb575, 0xc4a2b3, "Ember"},
    {0x111827, 0x253149, 0x94b8ff, 0x9daecb, "Orbit"}
};
typedef struct {
    int type, parent, x, y, w, h;
    uint32_t color;
    int radius, value;
    const char *text, *action, *long_action, *binding;
    int font_size;
} operit_layout_node_t;
typedef struct { const char *id; uint32_t background; const operit_layout_node_t *nodes; unsigned count; const char *swipe_left, *swipe_right; } operit_layout_page_t;
#include "layout.generated.h"
static void document_home(void);
static bool document_page(const char *id);
static void execute_route(const char *action);
static char pending_route[32], active_page[24], swipe_left[24], swipe_right[24];
static lv_indev_t *pointer_input;
static void queue_route(const char *action) { if(!*pending_route && action) { strncpy(pending_route,action,31);pending_route[31]=0; } }
static unsigned theme_index;
static char page_name[32] = "Chat";
static const char *current_page = page_name;
static bool round_icons;
static lv_display_t *display;
static lv_obj_t *root, *tiles, *clock_label, *connection_label, *face_label;
static lv_obj_t *pairing_label, *space_label, *chat_label, *chat_task_label;
static lv_obj_t *chat_input, *chat_keyboard, *chat_keyboard_close, *chat_send_button;
static lv_obj_t *sidebar_panel, *sidebar_scrim, *chat_content, *sidebar_preview_label;
static char chat_draft[512] = "";
static char submitted_draft[512] = "";
static bool chat_send_pending;
static lv_obj_t *chat_scroll, *wifi_label, *pairing_hint;
static uint16_t touch_x, touch_y;
static bool touch_pressed, wifi_ready, edge_ready;
static char expression[24] = "neutral";
static char pairing_code[20] = "";
static char space_state[40] = "等待连接 Operit";
static char chat_preview[96] = "尚未连接对话";
static char chat_screen[8192] = "尚未连接对话";
static char chat_task[192] = "离线";
static bool sidebar_open;
static int sidebar_width = 213; /* 66.6% default, as requested. */
static bool last_touch_pressed;
static uint16_t touch_start_x, touch_start_y;
static operit_lvgl_flush_cb_t flush_cb;
static operit_lvgl_action_cb_t action_cb;
static void *context;
static uint8_t draw_buffer[320 * 10 * 2] __attribute__((aligned(4)));
static int64_t last_tick;
static void home(void);
static void builtin_home(void);
static void page(const char *name);
static size_t copy_utf8(char *destination, size_t capacity, const char *source);
static const theme_t *theme(void) { return &themes[theme_index]; }

/*
 * A small accessibility-like registry is deliberately kept next to the
 * shared LVGL implementation instead of being inferred from the browser
 * canvas.  The same registry is therefore available on a real ESP32 build
 * and in the WebAssembly editor.  Nodes are metadata only; LVGL remains the
 * source of truth for geometry, visibility and text.
 */
#define OPERIT_DEBUG_NODE_LIMIT 128
#define OPERIT_DEBUG_ID_SIZE 48
#define OPERIT_DEBUG_ROLE_SIZE 20
#define OPERIT_DEBUG_ACTION_SIZE 40
#define OPERIT_DEBUG_TEXT_SIZE 256
#define OPERIT_DEBUG_JSON_SIZE 65536
typedef struct {
    lv_obj_t *object;
    char id[OPERIT_DEBUG_ID_SIZE];
    char role[OPERIT_DEBUG_ROLE_SIZE];
    char action[OPERIT_DEBUG_ACTION_SIZE];
    unsigned sequence;
} operit_debug_node_t;
static operit_debug_node_t debug_nodes[OPERIT_DEBUG_NODE_LIMIT];
static unsigned debug_count;
static char debug_json[OPERIT_DEBUG_JSON_SIZE];

static void debug_copy(char *destination, size_t capacity, const char *source) {
    if (!destination || capacity == 0) return;
    copy_utf8(destination, capacity, source ? source : "");
}

static int debug_find_object(lv_obj_t *object) {
    for (unsigned i = 0; i < debug_count; ++i) {
        if (debug_nodes[i].object == object) return (int)i;
    }
    return -1;
}

static const char *debug_action_id(const char *action) {
    if (!action || !*action) return NULL;
    if (!strcmp(action, "builtin:Pairing")) return "edge_pair";
    if (!strcmp(action, "builtin:Tasks")) return "tasks";
    if (!strcmp(action, "builtin:Settings")) return "settings";
    return action;
}

static int debug_register(lv_obj_t *object, const char *role, const char *id,
                          const char *action) {
    if (!object) return -1;
    int existing = debug_find_object(object);
    if (existing >= 0) {
        if (role && *role) debug_copy(debug_nodes[existing].role,
                                      sizeof(debug_nodes[existing].role), role);
        if (id && *id) debug_copy(debug_nodes[existing].id,
                                  sizeof(debug_nodes[existing].id), id);
        if (action && *action) debug_copy(debug_nodes[existing].action,
                                          sizeof(debug_nodes[existing].action), action);
        return existing;
    }
    if (debug_count >= OPERIT_DEBUG_NODE_LIMIT) return -1;
    unsigned index = debug_count++;
    operit_debug_node_t *node = &debug_nodes[index];
    memset(node, 0, sizeof(*node));
    node->object = object;
    node->sequence = index;
    debug_copy(node->role, sizeof(node->role), role && *role ? role : "panel");
    debug_copy(node->action, sizeof(node->action), action);
    if (id && *id) debug_copy(node->id, sizeof(node->id), id);
    else {
        char generated[OPERIT_DEBUG_ID_SIZE];
        snprintf(generated, sizeof(generated), "node_%u", index);
        debug_copy(node->id, sizeof(node->id), generated);
    }
    return (int)index;
}

static void debug_set_id(lv_obj_t *object, const char *id) {
    int index = debug_find_object(object);
    if (index >= 0 && id && *id) debug_copy(debug_nodes[index].id,
                                            sizeof(debug_nodes[index].id), id);
}

static void debug_reset(void) {
    memset(debug_nodes, 0, sizeof(debug_nodes));
    debug_count = 0;
}

static bool debug_visible(lv_obj_t *object) {
    for (lv_obj_t *current = object; current; current = lv_obj_get_parent(current)) {
        if (lv_obj_has_flag(current, LV_OBJ_FLAG_HIDDEN)) return false;
    }
    return true;
}

static void debug_json_char(size_t *offset, char value) {
    if (*offset + 2 >= sizeof(debug_json)) return;
    debug_json[(*offset)++] = value;
    debug_json[*offset] = 0;
}

static void debug_json_string(size_t *offset, const char *value) {
    debug_json_char(offset, '"');
    const unsigned char *text = (const unsigned char *)(value ? value : "");
    while (*text && *offset + 8 < sizeof(debug_json)) {
        unsigned char c = *text++;
        if (c == '"' || c == '\\') {
            debug_json_char(offset, '\\'); debug_json_char(offset, (char)c);
        } else if (c == '\n') {
            debug_json_char(offset, '\\'); debug_json_char(offset, 'n');
        } else if (c == '\r') {
            debug_json_char(offset, '\\'); debug_json_char(offset, 'r');
        } else if (c == '\t') {
            debug_json_char(offset, '\\'); debug_json_char(offset, 't');
        } else if (c < 0x20) {
            int written = snprintf(debug_json + *offset, sizeof(debug_json) - *offset,
                                    "\\u%04x", c);
            if (written > 0) *offset += (size_t)written;
        } else debug_json_char(offset, (char)c);
    }
    debug_json_char(offset, '"');
}

static void debug_json_text(size_t *offset, lv_obj_t *object, const char *fallback) {
    if (object && lv_obj_check_type(object, &lv_label_class)) {
        debug_json_string(offset, lv_label_get_text(object));
    } else if (object && lv_obj_check_type(object, &lv_textarea_class)) {
        debug_json_string(offset, lv_textarea_get_text(object));
    } else {
        debug_json_string(offset, fallback ? fallback : "");
    }
}

static void debug_append_node(size_t *offset, unsigned index) {
    operit_debug_node_t *node = &debug_nodes[index];
    lv_area_t area;
    lv_obj_get_coords(node->object, &area);
    int parent = -1;
    lv_obj_t *object_parent = lv_obj_get_parent(node->object);
    if (object_parent) parent = debug_find_object(object_parent);
    const char *fallback = "";
    if (!strcmp(node->id, "pairing_code")) fallback = pairing_code;
    debug_json_char(offset, '{');
    debug_json_string(offset, "id"); debug_json_char(offset, ':'); debug_json_string(offset, node->id);
    debug_json_char(offset, ','); debug_json_string(offset, "role"); debug_json_char(offset, ':'); debug_json_string(offset, node->role);
    debug_json_char(offset, ','); debug_json_string(offset, "text"); debug_json_char(offset, ':'); debug_json_text(offset, node->object, fallback);
    debug_json_char(offset, ','); debug_json_string(offset, "action"); debug_json_char(offset, ':'); debug_json_string(offset, node->action);
    debug_json_char(offset, ','); debug_json_string(offset, "parent"); debug_json_char(offset, ':');
    if (parent >= 0) debug_json_string(offset, debug_nodes[parent].id); else debug_json_string(offset, "");
    snprintf(debug_json + *offset, sizeof(debug_json) - *offset,
             ",\"rect\":{\"x\":%d,\"y\":%d,\"w\":%d,\"h\":%d},\"visible\":%s,\"enabled\":%s,\"clickable\":%s}",
             (int)area.x1, (int)area.y1, (int)(area.x2 - area.x1 + 1), (int)(area.y2 - area.y1 + 1),
             debug_visible(node->object) ? "true" : "false",
             lv_obj_has_state(node->object, LV_STATE_DISABLED) ? "false" : "true",
             lv_obj_has_flag(node->object, LV_OBJ_FLAG_CLICKABLE) ? "true" : "false");
    *offset += strlen(debug_json + *offset);
}

static const char *debug_build_json(bool snapshot) {
    size_t offset = 0;
    debug_json[0] = 0;
    debug_json_char(&offset, '{');
    debug_json_string(&offset, "page"); debug_json_char(&offset, ':'); debug_json_string(&offset, current_page);
    debug_json_char(&offset, ','); debug_json_string(&offset, "width"); debug_json_char(&offset, ':');
    offset += (size_t)snprintf(debug_json + offset, sizeof(debug_json) - offset, "320");
    debug_json_char(&offset, ','); debug_json_string(&offset, "height"); debug_json_char(&offset, ':');
    offset += (size_t)snprintf(debug_json + offset, sizeof(debug_json) - offset, "240");
    if (snapshot) {
        debug_json_char(&offset, ','); debug_json_string(&offset, "sidebarOpen"); debug_json_char(&offset, ':');
        offset += (size_t)snprintf(debug_json + offset, sizeof(debug_json) - offset, "%s", sidebar_open ? "true" : "false");
        debug_json_char(&offset, ','); debug_json_string(&offset, "sidebarWidth"); debug_json_char(&offset, ':');
        offset += (size_t)snprintf(debug_json + offset, sizeof(debug_json) - offset, "%d", sidebar_width);
        debug_json_char(&offset, ','); debug_json_string(&offset, "pairingCode"); debug_json_char(&offset, ':');
        debug_json_string(&offset, pairing_code);
        debug_json_char(&offset, ','); debug_json_string(&offset, "chat"); debug_json_char(&offset, ':');
        debug_json_string(&offset, chat_screen);
    }
    debug_json_char(&offset, ','); debug_json_string(&offset, "nodes"); debug_json_char(&offset, ':'); debug_json_char(&offset, '[');
    for (unsigned i = 0; i < debug_count; ++i) {
        if (i) debug_json_char(&offset, ',');
        debug_append_node(&offset, i);
    }
    debug_json_char(&offset, ']'); debug_json_char(&offset, '}');
    return debug_json;
}
/* Default palette tokens follow the selected theme; custom colors stay literal. */
static uint32_t document_color(uint32_t color) {
    if(color==0x091420)return theme()->bg;
    if(color==0x172a3d)return theme()->surface;
    if(color==0x53dfc5)return theme()->accent;
    if(color==0x216c73)return theme_index==0?color:theme()->surface;
    return color;
}

static lv_obj_t *box(lv_obj_t *parent, int x, int y, int w, int h, uint32_t color, int radius) {
    lv_obj_t *o = lv_obj_create(parent);
    lv_obj_remove_style_all(o);
    lv_obj_set_pos(o, x, y); lv_obj_set_size(o, w, h);
    lv_obj_set_style_bg_color(o, lv_color_hex(color), 0);
    lv_obj_set_style_bg_opa(o, LV_OPA_COVER, 0);
    lv_obj_set_style_radius(o, radius, 0);
    lv_obj_clear_flag(o, LV_OBJ_FLAG_SCROLLABLE);
    debug_register(o, "panel", NULL, NULL);
    return o;
}
static lv_obj_t *label(lv_obj_t *p, const char *text, int x, int y, int w, uint32_t color) {
    lv_obj_t *o = lv_label_create(p);
    lv_label_set_text(o, text); lv_obj_set_pos(o, x, y); lv_obj_set_width(o, w);
    lv_obj_set_style_text_color(o, lv_color_hex(color), 0);
    lv_obj_set_style_text_font(o, &operit_font_zh_14, 0);
    lv_label_set_long_mode(o, LV_LABEL_LONG_CLIP);
    lv_obj_clear_flag(o, LV_OBJ_FLAG_CLICKABLE);
    debug_register(o, "label", NULL, NULL);
    return o;
}
static void clicked(lv_event_t *e) {
    const char *name = lv_event_get_user_data(e);
    if (!strcmp(name,"Home")) queue_route("home");
    else if (!strcmp(name,"Palette")) queue_route("theme_next");
    else if (!strcmp(name,"Shape")) queue_route("shape_toggle");
    else if (!strcmp(name,"Online")) queue_route("face_online");
    else if (!strcmp(name,"Run")) queue_route("run_node");
    else if (!strcmp(name,"edge_search") || !strcmp(name,"edge_pair") || !strcmp(name,"edge_chat") || !strcmp(name,"edge_send") ||
             !strcmp(name,"edge_new") || !strcmp(name,"sidebar_toggle") || !strcmp(name,"sidebar_open") ||
             !strcmp(name,"sidebar_close") || !strcmp(name,"chat_keyboard_close") || !strcmp(name,"sidebar_width_cycle")) queue_route(name);
    else if (!strncmp(name, "builtin:", 8) || !strncmp(name, "go:", 3) || !strncmp(name, "page:", 5)) queue_route(name);
    else { char route[32];snprintf(route,sizeof(route),"builtin:%s",name);queue_route(route); }
}
static lv_obj_t *button(lv_obj_t *p, const char *text, const char *action, int x, int y, int w, int h) {
    lv_obj_t *o = box(p, x, y, w, h, theme()->surface, 12);
    debug_register(o, "button", debug_action_id(action), action);
    lv_obj_add_flag(o, LV_OBJ_FLAG_CLICKABLE);
    lv_obj_set_style_bg_color(o, lv_color_hex(theme()->accent), LV_STATE_PRESSED);
    lv_obj_add_event_cb(o, clicked, LV_EVENT_CLICKED, (void *)action);
    lv_obj_t *t = label(o, text, 0, 0, w - 8, 0xf4f8ff);
    lv_obj_set_style_text_align(t, LV_TEXT_ALIGN_CENTER, 0); lv_obj_center(t);
    return o;
}
static void clear(void) {
    active_page[0]=0;
    swipe_left[0]=swipe_right[0]=0;
    clock_label = connection_label = face_label = tiles = NULL;
    pairing_label = space_label = chat_label = chat_task_label = NULL;
    chat_input = chat_keyboard = chat_keyboard_close = chat_send_button = NULL;
    chat_scroll = wifi_label = pairing_hint = NULL;
    sidebar_panel = sidebar_scrim = chat_content = sidebar_preview_label = NULL;
    lv_obj_clean(root);
    lv_obj_set_style_bg_color(root, lv_color_hex(theme()->bg), 0);
    debug_reset();
}
static void tick_clock(lv_timer_t *timer) {
    (void)timer;
    if (!clock_label) return;
    time_t now = time(NULL); struct tm tm;
    char text[24];
    if (now > 1700000000 && localtime_r(&now, &tm)) strftime(text, sizeof(text), "%H:%M", &tm);
    else snprintf(text, sizeof(text), "--:--");
    lv_label_set_text(clock_label, text);
}
static void icon(lv_obj_t *p, const char *symbol, const char *name, int x, int y, uint32_t color) {
    lv_obj_t *o = button(p, symbol, name, x, y, 52, 52);
    lv_obj_set_style_bg_color(o, lv_color_hex(color), 0);
    lv_obj_set_style_radius(o, round_icons ? LV_RADIUS_CIRCLE : 15, 0);
    lv_obj_t *t = label(p, name, x - 7, y + 57, 66, 0xf4f8ff);
    lv_obj_set_style_text_align(t, LV_TEXT_ALIGN_CENTER, 0);
}
static void builtin_home(void) {
    /* Chat is the only primary app. All secondary tools live in its drawer. */
    sidebar_open = false;
    page("Chat");
}
/* Copy whole UTF-8 code points so a bounded display buffer never ends in a broken glyph. */
static size_t copy_utf8(char *destination, size_t capacity, const char *source) {
    if (!destination || capacity == 0) return 0;
    if (!source) source = "";
    size_t out = 0;
    while (*source) {
        unsigned char lead = (unsigned char)*source;
        size_t bytes = lead < 0x80 ? 1 : (lead & 0xe0) == 0xc0 ? 2 :
                       (lead & 0xf0) == 0xe0 ? 3 : (lead & 0xf8) == 0xf0 ? 4 : 1;
        if (out + bytes >= capacity) break;
        bool valid = true;
        for (size_t i = 1; i < bytes; ++i) {
            if (source[i] == 0 || ((unsigned char)source[i] & 0xc0) != 0x80) { valid = false; break; }
        }
        if (!valid) bytes = 1;
        memcpy(destination + out, source, bytes);
        source += bytes;
        out += bytes;
    }
    destination[out] = 0;
    return out;
}

static void remember_chat_draft(void) {
    if (!chat_input) return;
    copy_utf8(chat_draft, sizeof(chat_draft), lv_textarea_get_text(chat_input));
}

static void hide_chat_keyboard(void) {
    if (chat_keyboard) {
        lv_obj_add_flag(chat_keyboard, LV_OBJ_FLAG_HIDDEN);
        lv_keyboard_set_textarea(chat_keyboard, NULL);
    }
    if (chat_keyboard_close) lv_obj_add_flag(chat_keyboard_close, LV_OBJ_FLAG_HIDDEN);
    if (chat_send_button) lv_obj_clear_flag(chat_send_button, LV_OBJ_FLAG_HIDDEN);
    if (chat_input) {
        lv_obj_set_y(chat_input, 181);
        lv_obj_clear_state(chat_input, LV_STATE_FOCUSED);
    }
}

static void show_chat_keyboard(void) {
    if (!chat_keyboard || !chat_input) return;
    lv_keyboard_set_textarea(chat_keyboard, chat_input);
    lv_obj_set_style_text_font(chat_keyboard, &operit_font_zh_14, 0);
    lv_obj_set_y(chat_input, 48);
    if (chat_send_button) lv_obj_add_flag(chat_send_button, LV_OBJ_FLAG_HIDDEN);
    lv_obj_clear_flag(chat_keyboard, LV_OBJ_FLAG_HIDDEN);
    if (chat_keyboard_close) lv_obj_clear_flag(chat_keyboard_close, LV_OBJ_FLAG_HIDDEN);
}

static const char *sidebar_width_name(void) {
    if (sidebar_width <= 180) return "50%";
    if (sidebar_width >= 240) return "80%";
    return "67%";
}

static void cycle_sidebar_width(void) {
    if (sidebar_width <= 180) sidebar_width = 213;
    else if (sidebar_width <= 240) sidebar_width = 256;
    else sidebar_width = 160;
}

static lv_obj_t *sidebar_button(lv_obj_t *parent, const char *text, const char *action, int y, bool active) {
    int width = sidebar_width - 20;
    lv_obj_t *o = button(parent, text, action, 10, y, width, 30);
    lv_obj_set_style_radius(o, 10, 0);
    lv_obj_set_style_bg_color(o, lv_color_hex(active ? theme()->accent : theme()->surface), 0);
    return o;
}

static void sidebar_anim_exec(void *object, int32_t x) {
    lv_obj_set_x((lv_obj_t *)object, x);
}

static void sidebar_close_ready(lv_anim_t *animation) {
    (void)animation;
    if (!sidebar_open) {
        if (sidebar_panel) lv_obj_add_flag(sidebar_panel, LV_OBJ_FLAG_HIDDEN);
        if (sidebar_scrim) lv_obj_add_flag(sidebar_scrim, LV_OBJ_FLAG_HIDDEN);
    }
}

static void animate_drawer_object(lv_obj_t *object, int32_t from, int32_t to, uint32_t time,
                                  lv_anim_ready_cb_t ready_cb) {
    if (!object) return;
    (void)from;
    lv_anim_delete(object, sidebar_anim_exec);
    lv_obj_update_layout(object);
    from = lv_obj_get_x(object);
    lv_anim_t animation;
    lv_anim_init(&animation);
    lv_anim_set_var(&animation, object);
    lv_anim_set_values(&animation, from, to);
    lv_anim_set_time(&animation, time);
    lv_anim_set_path_cb(&animation, lv_anim_path_ease_out);
    lv_anim_set_exec_cb(&animation, sidebar_anim_exec);
    if (ready_cb) lv_anim_set_ready_cb(&animation, ready_cb);
    lv_anim_start(&animation);
}

static void draw_sidebar(void) {
    sidebar_scrim = box(root, sidebar_width, 0, 320 - sidebar_width, 240, 0x000000, 0);
    debug_set_id(sidebar_scrim, "sidebar_scrim");
    lv_obj_set_style_bg_opa(sidebar_scrim, LV_OPA_50, 0);
    lv_obj_add_flag(sidebar_scrim, LV_OBJ_FLAG_CLICKABLE);
    lv_obj_add_event_cb(sidebar_scrim, clicked, LV_EVENT_CLICKED, (void *)"sidebar_close");

    sidebar_panel = box(root, 0, 0, sidebar_width, 240, theme()->surface, 0);
    debug_set_id(sidebar_panel, "sidebar");
    lv_obj_add_flag(sidebar_panel, LV_OBJ_FLAG_SCROLLABLE);
    lv_obj_set_scrollbar_mode(sidebar_panel, LV_SCROLLBAR_MODE_OFF);
    lv_obj_set_scroll_dir(sidebar_panel, LV_DIR_VER);
    label(sidebar_panel, "聊天", 16, 6, sidebar_width - 32, theme()->accent);
    sidebar_button(sidebar_panel, "清空草稿", "edge_new", 28, false);
    label(sidebar_panel, "对话记录", 16, 64, sidebar_width - 32, theme()->muted);
    sidebar_button(sidebar_panel, "当前对话", "sidebar_close", 78, true);
    sidebar_preview_label = label(sidebar_panel, chat_preview, 20, 112, sidebar_width - 36, theme()->muted);
    debug_set_id(sidebar_preview_label, "chat_preview");
    lv_label_set_long_mode(sidebar_preview_label, LV_LABEL_LONG_WRAP);
    lv_obj_set_height(sidebar_preview_label, 30);
    sidebar_button(sidebar_panel, "配对设备", "builtin:Pairing", 144, false);
    sidebar_button(sidebar_panel, "任务", "builtin:Tasks", 176, false);
    sidebar_button(sidebar_panel, "设置", "builtin:Settings", 208, false);

    if (sidebar_open) {
        lv_obj_set_x(chat_content, sidebar_width);
        lv_obj_set_x(sidebar_panel, 0);
        lv_obj_set_x(sidebar_scrim, sidebar_width);
    } else {
        lv_obj_set_x(chat_content, 0);
        lv_obj_set_x(sidebar_panel, -sidebar_width);
        lv_obj_set_x(sidebar_scrim, 320);
        lv_obj_add_flag(sidebar_panel, LV_OBJ_FLAG_HIDDEN);
        lv_obj_add_flag(sidebar_scrim, LV_OBJ_FLAG_HIDDEN);
    }
}

static void set_sidebar_open(bool open, bool animate) {
    sidebar_open = open;
    if (!chat_content || !sidebar_panel || !sidebar_scrim) {
        page("Chat");
        return;
    }
    if (!animate) {
        if (open) {
            lv_obj_clear_flag(sidebar_panel, LV_OBJ_FLAG_HIDDEN);
            lv_obj_clear_flag(sidebar_scrim, LV_OBJ_FLAG_HIDDEN);
            lv_obj_set_x(chat_content, sidebar_width);
            lv_obj_set_x(sidebar_panel, 0);
            lv_obj_set_x(sidebar_scrim, sidebar_width);
        } else {
            lv_obj_set_x(chat_content, 0);
            lv_obj_set_x(sidebar_panel, -sidebar_width);
            lv_obj_set_x(sidebar_scrim, 320);
            lv_obj_add_flag(sidebar_panel, LV_OBJ_FLAG_HIDDEN);
            lv_obj_add_flag(sidebar_scrim, LV_OBJ_FLAG_HIDDEN);
        }
        return;
    }
    if (open) {
        hide_chat_keyboard();
        lv_obj_clear_flag(sidebar_panel, LV_OBJ_FLAG_HIDDEN);
        lv_obj_clear_flag(sidebar_scrim, LV_OBJ_FLAG_HIDDEN);
        animate_drawer_object(chat_content, 0, sidebar_width, 220, NULL);
        animate_drawer_object(sidebar_panel, -sidebar_width, 0, 220, NULL);
        animate_drawer_object(sidebar_scrim, 320, sidebar_width, 220, NULL);
    } else {
        animate_drawer_object(chat_content, sidebar_width, 0, 180, NULL);
        animate_drawer_object(sidebar_panel, 0, -sidebar_width, 180, sidebar_close_ready);
        animate_drawer_object(sidebar_scrim, sidebar_width, 320, 180, NULL);
    }
}

static void chat_keyboard_event(lv_event_t *event) {
    lv_event_code_t code = lv_event_get_code(event);
    if (code == LV_EVENT_READY) {
        remember_chat_draft();
        hide_chat_keyboard();
        queue_route("edge_send");
    } else if (code == LV_EVENT_CANCEL) {
        hide_chat_keyboard();
    }
}

static void chat_input_event(lv_event_t *event) {
    lv_event_code_t code = lv_event_get_code(event);
    if (code == LV_EVENT_FOCUSED) show_chat_keyboard();
    else if (code == LV_EVENT_DEFOCUSED) hide_chat_keyboard();
}

static void draw_chat(void) {
    chat_content = lv_obj_create(root);
    lv_obj_remove_style_all(chat_content);
    lv_obj_set_size(chat_content, 320, 240);
    lv_obj_clear_flag(chat_content, LV_OBJ_FLAG_SCROLLABLE);
    debug_register(chat_content, "screen", "chat", NULL);
    box(chat_content, 0, 0, 320, 42, theme()->surface, 0);
    button(chat_content, LV_SYMBOL_LIST, "sidebar_toggle", 8, 7, 32, 28);
    label(chat_content, "聊天", 50, 10, 130, 0xf4f8ff);
    connection_label = label(chat_content, edge_ready ? "已连接" : "离线", 224, 12, 82, edge_ready ? theme()->accent : theme()->muted);
    debug_set_id(connection_label, "connection_status");
    lv_obj_set_style_text_align(connection_label, LV_TEXT_ALIGN_RIGHT, 0);

    /* The ESP32 is intentionally a tiny message terminal, not a full AI chat UI:
     * one plain text output area, one input field, and one send button. */
    box(chat_content, 10, 50, 300, 124, theme()->surface, 16);
    chat_scroll = box(chat_content, 18, 58, 284, 110, theme()->surface, 0);
    debug_set_id(chat_scroll, "chat_text");
    lv_obj_add_flag(chat_scroll, LV_OBJ_FLAG_SCROLLABLE);
    lv_obj_set_scroll_dir(chat_scroll, LV_DIR_VER);
    lv_obj_set_scrollbar_mode(chat_scroll, LV_SCROLLBAR_MODE_AUTO);
    chat_label = label(chat_scroll, chat_screen, 2, 0, 272, 0xf4f8ff);
    debug_set_id(chat_label, "chat_screen");
    lv_label_set_long_mode(chat_label, LV_LABEL_LONG_WRAP);
    lv_obj_set_style_text_line_space(chat_label, 2, 0);

    chat_input = lv_textarea_create(chat_content);
    debug_register(chat_input, "textbox", "chat_input", NULL);
    lv_obj_set_pos(chat_input, 10, 181); lv_obj_set_size(chat_input, 252, 38);
    lv_textarea_set_one_line(chat_input, true);
    lv_textarea_set_max_length(chat_input, 120);
    lv_textarea_set_placeholder_text(chat_input, "输入消息…");
    lv_textarea_set_text(chat_input, chat_draft);
    lv_obj_set_style_bg_color(chat_input, lv_color_hex(theme()->surface), 0);
    lv_obj_set_style_text_color(chat_input, lv_color_hex(0xf4f8ff), 0);
    lv_obj_set_style_text_font(chat_input, &operit_font_zh_14, 0);
    lv_obj_add_event_cb(chat_input, chat_input_event, LV_EVENT_ALL, NULL);
    chat_send_button = button(chat_content, "发送", "edge_send", 270, 181, 40, 38);

    chat_keyboard = lv_keyboard_create(chat_content);
    debug_register(chat_keyboard, "keyboard", "chat_keyboard", NULL);
    lv_keyboard_set_textarea(chat_keyboard, chat_input);
    lv_obj_set_style_text_font(chat_keyboard, &operit_font_zh_14, 0);
    lv_keyboard_set_popovers(chat_keyboard, false);
    lv_obj_set_pos(chat_keyboard, 6, 88); lv_obj_set_size(chat_keyboard, 308, 147);
    lv_obj_add_flag(chat_keyboard, LV_OBJ_FLAG_HIDDEN);
    lv_obj_add_event_cb(chat_keyboard, chat_keyboard_event, LV_EVENT_ALL, NULL);
    chat_keyboard_close = button(chat_content, "X", "chat_keyboard_close", 270, 48, 40, 32);
    lv_obj_add_flag(chat_keyboard_close, LV_OBJ_FLAG_HIDDEN);

    draw_sidebar();
}

static void page(const char *name) {
    copy_utf8(page_name, sizeof(page_name), name);
    current_page = page_name;
    name = page_name;
    if (strcmp(name, "Chat")) sidebar_open = false;
    if (chat_input) remember_chat_draft();
    clear();
    if (!strcmp(name, "Chat")) {
        draw_chat();
    } else if (!strcmp(name, "Settings")) {
        button(root, LV_SYMBOL_LEFT, "edge_chat", 10, 8, 32, 30);
        label(root, "设置", 52, 14, 190, 0xf4f8ff);
        label(root, "连接状态", 18, 55, 280, theme()->muted);
        wifi_label = label(root, wifi_ready ? "Wi-Fi 已连接" : "需要设置 Wi-Fi", 18, 76, 280, 0xf4f8ff);
        space_label = label(root, edge_ready ? space_state : "等待 Core 连接", 18, 98, 280, theme()->muted);
        label(root, "侧栏宽度", 18, 132, 280, theme()->muted);
        char width_text[48]; snprintf(width_text, sizeof(width_text), "侧栏宽度：%s", sidebar_width_name());
        button(root, width_text, "sidebar_width_cycle", 18, 153, 284, 38);
        label(root, "从左侧边缘向右滑动打开", 18, 204, 284, theme()->muted);
    } else if (!strcmp(name, "Pairing")) {
        button(root, LV_SYMBOL_LEFT, "edge_chat", 10, 8, 32, 30);
        label(root, "配对设备", 52, 14, 190, 0xf4f8ff);
        wifi_label = label(root, wifi_ready ? "Wi-Fi 已连接" : "Wi-Fi 不可用", 18, 55, 284, theme()->muted);
        debug_set_id(wifi_label, "wifi_status");
        space_label = label(root, edge_ready ? "已连接 Operit" : "等待 Core 连接", 18, 82, 284, edge_ready ? theme()->accent : theme()->muted);
        debug_set_id(space_label, "space_state");
        pairing_label = label(root, pairing_code[0] ? pairing_code : "------", 18, 112, 284, theme()->accent);
        debug_set_id(pairing_label, "pairing_code");
        pairing_hint = label(root, pairing_code[0] ? "请在 Operit 中输入此配对码" : "打开 Operit > 设备 > 添加边缘设备", 18, 142, 284, theme()->muted);
        debug_set_id(pairing_hint, "pairing_hint");
        button(root, "刷新", "edge_pair", 18, 187, 92, 36);
        button(root, "解除配对", "edge_unpair", 116, 187, 92, 36);
        button(root, "返回", "edge_chat", 212, 187, 90, 36);
    } else if (!strcmp(name, "Tasks")) {
        button(root, LV_SYMBOL_LEFT, "edge_chat", 10, 8, 32, 30);
        label(root, "任务", 52, 14, 190, 0xf4f8ff);
        label(root, "当前任务", 18, 58, 284, theme()->muted);
        chat_task_label = label(root, chat_task, 18, 82, 284, theme()->accent);
        label(root, "任务状态也会显示在聊天页面。", 18, 119, 284, theme()->muted);
        button(root, "返回聊天", "edge_chat", 18, 181, 284, 36);
    } else {
        button(root, LV_SYMBOL_LEFT, "edge_chat", 12, 10, 32, 32);
        label(root, name, 56, 17, 246, theme()->accent);
        if (!strcmp(name, "Theme")) {
            label(root, "颜色主题", 20, 58, 250, theme()->muted);
            button(root, theme()->name, "Palette", 20, 84, 280, 44);
            label(root, "图标形状", 20, 144, 250, theme()->muted);
            button(root, round_icons ? "圆形" : "圆角方形", "Shape", 20, 170, 280, 44);
        } else if (!strcmp(name, "Face")) {
            lv_obj_t *eyes = label(root, "o   o", 20, 65, 280, theme()->accent);
            lv_obj_set_style_text_font(eyes, &lv_font_montserrat_48, 0);
            lv_obj_set_style_text_align(eyes, LV_TEXT_ALIGN_CENTER, 0);
            face_label = label(root, expression, 20, 136, 280, theme()->muted);
            lv_obj_set_style_text_align(face_label, LV_TEXT_ALIGN_CENTER, 0);
            button(root, "在线", "Online", 90, 180, 140, 40);
        } else if (!strcmp(name, "Network")) {
            wifi_label = label(root, wifi_ready ? "Wi-Fi 已连接" : "需要设置 Wi-Fi", 20, 66, 280, 0xf4f8ff);
            space_label = label(root, edge_ready ? space_state : "等待 Core 连接", 20, 98, 280, theme()->muted);
            pairing_label = label(root, pairing_code[0] ? pairing_code : "等待配对码", 20, 126, 280, theme()->accent);
            button(root, "搜索 Space", "edge_search", 20, 180, 88, 40);
            button(root, "配对", "edge_pair", 116, 180, 88, 40);
            button(root, "聊天", "edge_chat", 212, 180, 88, 40);
        } else if (!strcmp(name, "Terminal")) {
            label(root, "> Operit Edge 已就绪", 20, 70, 280, theme()->accent);
            label(root, "本地屏幕 · Wi-Fi", 20, 106, 280, theme()->muted);
            button(root, "运行节点", "Run", 20, 180, 280, 40);
        } else { label(root, "暂无已安装插件", 20, 92, 280, theme()->muted); }
    }
}

static void flush(lv_display_t *d, const lv_area_t *a, uint8_t *pixels) {
    operit_lvgl_area_t area = {a->x1, a->y1, a->x2, a->y2};
    flush_cb(&area, pixels, (a->x2-a->x1+1)*(a->y2-a->y1+1)*2, context);
    lv_display_flush_ready(d);
}
static void read_touch(lv_indev_t *dev, lv_indev_data_t *data) {
    (void)dev; data->point.x=touch_x; data->point.y=touch_y;
    data->state=touch_pressed ? LV_INDEV_STATE_PRESSED : LV_INDEV_STATE_RELEASED;
}
bool operit_lvgl_init(uint16_t w, uint16_t h, operit_lvgl_flush_cb_t f, operit_lvgl_touch_cb_t t, operit_lvgl_action_cb_t a, void *user) {
    (void)t; if (w!=320 || h!=240 || !f) return false;
    flush_cb=f; action_cb=a; context=user; lv_init();operit_store_init();
    display=lv_display_create(w,h); if (!display) return false;
    lv_display_set_color_format(display, LV_COLOR_FORMAT_RGB565);
    lv_display_set_buffers(display, draw_buffer, NULL, sizeof(draw_buffer), LV_DISPLAY_RENDER_MODE_PARTIAL);
    lv_display_set_flush_cb(display, flush);
    lv_indev_t *input=lv_indev_create(); if (!input) return false; pointer_input=input;
    lv_indev_set_type(input,LV_INDEV_TYPE_POINTER); lv_indev_set_read_cb(input,read_touch);
    root=lv_obj_create(NULL); lv_obj_remove_style_all(root);
    lv_obj_set_size(root,w,h); lv_obj_set_style_bg_opa(root,LV_OPA_COVER,0);
    lv_obj_clear_flag(root,LV_OBJ_FLAG_SCROLLABLE); lv_screen_load(root);
    home(); lv_timer_create(tick_clock,1000,NULL); last_tick=esp_timer_get_time(); return true;
}
void operit_lvgl_pump(uint32_t elapsed_ms) {
    if(operit_store_poll()){touch_pressed=false;lv_indev_reset(pointer_input,NULL);lv_indev_wait_release(pointer_input);home();}
    (void)elapsed_ms; int64_t now=esp_timer_get_time();
    uint32_t ms=(uint32_t)((now-last_tick)/1000); last_tick+=(int64_t)ms*1000;
    lv_tick_inc(ms); lv_timer_handler();
    if(*pending_route) { char action[32];memcpy(action,pending_route,32);pending_route[0]=0;lv_indev_reset(pointer_input,NULL);lv_indev_wait_release(pointer_input);execute_route(action); }
}
void operit_lvgl_set_touch(uint16_t x,uint16_t y,bool pressed) {
    if (pressed && !last_touch_pressed) {
        touch_start_x = x; touch_start_y = y;
    } else if (!pressed && last_touch_pressed) {
        int dx = (int)x - (int)touch_start_x;
        int dy = (int)y - (int)touch_start_y;
        if (abs(dx) > 35 && abs(dx) > abs(dy) && !strcmp(current_page, "Chat")) {
            if (!sidebar_open && touch_start_x <= 24 && dx > 35) queue_route("sidebar_open");
            else if (sidebar_open && dx < -35) queue_route("sidebar_close");
        }
    }
    last_touch_pressed = pressed;
    touch_x=x; touch_y=y; touch_pressed=pressed;
}
void operit_lvgl_navigate_home(void) {home();}
void operit_lvgl_set_connection(bool wifi,bool edge) {
    if(wifi==wifi_ready && edge==edge_ready) return;
    wifi_ready=wifi; edge_ready=edge;
    if(connection_label) {
        lv_label_set_text(connection_label, edge ? "已连接" : "离线");
        lv_obj_set_style_text_color(connection_label, lv_color_hex(edge ? theme()->accent : theme()->muted), 0);
    }
    if(wifi_label) lv_label_set_text(wifi_label, wifi ? "Wi-Fi 已连接" : "Wi-Fi 不可用");
    if(space_label) lv_label_set_text(space_label, edge ? space_state : "等待 Core 连接");
}
void operit_lvgl_set_pairing_code(const char *code) {
    const char *value = code ? code : "";
    if (!strcmp(pairing_code, value)) return;
    copy_utf8(pairing_code, sizeof(pairing_code), value);
    if (pairing_label) lv_label_set_text(pairing_label, pairing_code[0] ? pairing_code : "等待配对码");
    if (pairing_hint) lv_label_set_text(pairing_hint, pairing_code[0] ? "请在 Operit 中输入此配对码" : "打开 Operit > 设备 > 添加边缘设备");
}
void operit_lvgl_set_space_state(const char *state) {
    const char *value = state ? state : "等待连接 Operit";
    if (!strcmp(space_state, value)) return;
    copy_utf8(space_state, sizeof(space_state), value);
    if (space_label) lv_label_set_text(space_label, space_state);
}
void operit_lvgl_set_chat_preview(const char *preview) {
    const char *value = preview ? preview : "尚未连接对话";
    if (!strcmp(chat_preview, value)) return;
    copy_utf8(chat_preview, sizeof(chat_preview), value);
    if (sidebar_preview_label) lv_label_set_text(sidebar_preview_label, chat_preview);
}
void operit_lvgl_set_expression(const char *value) {
    if(!value || !strcmp(expression,value)) return;
    copy_utf8(expression, sizeof(expression), value);
    if(face_label) lv_label_set_text(face_label,expression);
}

/* Host controls shared by the firmware and the WebAssembly developer host. */
void operit_lvgl_set_chat_screen(const char *text) {
    if (!text || !strcmp(chat_screen, text)) return;
    bool at_bottom = !chat_scroll || lv_obj_get_scroll_bottom(chat_scroll) <= 4;
    copy_utf8(chat_screen, sizeof(chat_screen), text);
    if (chat_label) {
        lv_label_set_text(chat_label, chat_screen);
        lv_obj_update_layout(chat_scroll);
        if (at_bottom) lv_obj_scroll_to_y(chat_scroll, LV_COORD_MAX, LV_ANIM_OFF);
    }
}
void operit_lvgl_set_chat_task(const char *text) {
    if (!text || !strcmp(chat_task, text)) return;
    copy_utf8(chat_task, sizeof(chat_task), text);
    if (chat_task_label) lv_label_set_text(chat_task_label, chat_task);
}
const char *operit_lvgl_chat_draft(void) { remember_chat_draft(); return chat_draft; }
void operit_lvgl_set_chat_draft(const char *text) {
    copy_utf8(chat_draft, sizeof(chat_draft), text);
    if (chat_input) lv_textarea_set_text(chat_input, chat_draft);
}
void operit_lvgl_submit_chat(void) { queue_route("edge_send"); }
void operit_lvgl_chat_send_result(bool ok, const char *error) {
    if (!chat_send_pending) return;
    chat_send_pending = false;
    remember_chat_draft();
    if (ok && !strcmp(chat_draft, submitted_draft)) operit_lvgl_set_chat_draft("");
    if (!ok) operit_lvgl_set_chat_task(error && *error ? error : "发送失败，请重试");
}

void operit_lvgl_set_theme(unsigned index, bool circular) {
    theme_index = index % (sizeof(themes) / sizeof(themes[0]));
    round_icons = circular;
    home();
}
void operit_lvgl_navigate_apps(void) {
    sidebar_open = true;
    page("Chat");
}
static void home(void) {
    if (OPERIT_LAYOUT_ENABLED) document_home();
    else builtin_home();
}

unsigned operit_lvgl_theme_index(void) { return theme_index; }
bool operit_lvgl_round_icons(void) { return round_icons; }

const char *operit_lvgl_current_page(void) {
    if (tiles && lv_tileview_get_tile_active(tiles) == lv_obj_get_child(tiles, 1)) return "Apps";
    return current_page;
}

const char *operit_lvgl_debug_tree(void) {
    return debug_build_json(false);
}

const char *operit_lvgl_debug_snapshot(void) {
    return debug_build_json(true);
}

bool operit_lvgl_debug_tap(const char *id) {
    if (!id || !*id) return false;
    for (unsigned i = 0; i < debug_count; ++i) {
        operit_debug_node_t *node = &debug_nodes[i];
        if (strcmp(node->id, id) || !debug_visible(node) ||
            !lv_obj_has_flag(node->object, LV_OBJ_FLAG_CLICKABLE)) continue;
        lv_area_t area;
        lv_obj_get_coords(node->object, &area);
        uint16_t x = (uint16_t)((area.x1 + area.x2) / 2);
        uint16_t y = (uint16_t)((area.y1 + area.y2) / 2);
        operit_lvgl_set_touch(x, y, true);
        operit_lvgl_pump(0);
        operit_lvgl_set_touch(x, y, false);
        operit_lvgl_pump(0);
        /* A route is intentionally consumed by the next pump so callbacks
         * and page transitions follow the exact same path as a real tap. */
        operit_lvgl_pump(0);
        return true;
    }
    return false;
}

bool operit_lvgl_debug_swipe(const char *direction) {
    if (!direction || !*direction || strcmp(current_page, "Chat")) return false;
    bool right = !strcmp(direction, "right") || !strcmp(direction, "open");
    bool left = !strcmp(direction, "left") || !strcmp(direction, "close");
    if (!right && !left) return false;
    uint16_t start = right ? 4 : 300;
    uint16_t end = right ? 150 : 170;
    operit_lvgl_set_touch(start, 120, true);
    operit_lvgl_pump(0);
    operit_lvgl_set_touch(end, 120, false);
    operit_lvgl_pump(0);
    operit_lvgl_pump(0);
    return true;
}

/* The editor and firmware execute this same component factory. */
static lv_obj_t *editor_nodes[24];
static char editor_text[24][161], editor_action[24][24], editor_long_action[24][24];
static unsigned editor_count;
static void layout_action(lv_event_t *event) {
    unsigned index = (unsigned)(uintptr_t)lv_event_get_user_data(event);
    if(index >= editor_count) return;
    lv_event_code_t code = lv_event_get_code(event);
    const char *binding = NULL;
    if(code == LV_EVENT_LONG_PRESSED && *editor_long_action[index]) binding = editor_long_action[index];
    else if(code == (*editor_long_action[index] ? LV_EVENT_SHORT_CLICKED : LV_EVENT_CLICKED)) binding = editor_action[index];
    if(!binding || !*binding) return;
    queue_route(binding);
}
void operit_lvgl_layout_clear(uint32_t background) {
    pending_route[0]=0; swipe_left[0]=swipe_right[0]=0;
    clear(); current_page = "Layout"; editor_count = 0;
    memset(editor_nodes, 0, sizeof(editor_nodes));
    lv_obj_set_style_bg_color(root, lv_color_hex(document_color(background)), 0);
}
int operit_lvgl_layout_add(int type, int parent_index, int x, int y, int w, int h,
    uint32_t color, int radius, int value, const char *text, const char *action) {
    if (editor_count >= 24 || w < 8 || h < 8) return -1;
    color=document_color(color);
    if(type==2 && w==h && round_icons)radius=w/2;
    unsigned index = editor_count;
    lv_obj_t *parent = parent_index >= 0 && parent_index < (int)index ? editor_nodes[parent_index] : root;
    strncpy(editor_text[index], text ? text : "", 160); editor_text[index][160] = 0;
    strncpy(editor_action[index], action ? action : "", 23); editor_action[index][23] = 0;
    editor_long_action[index][0] = 0;
    const char *content = editor_text[index];
    lv_obj_t *obj = NULL;
    static const char *matrix[] = {"One", "Two", "\n", "Three", "", NULL};
    static const lv_point_precise_t points[] = {{0, 0}, {80, 10}};
    switch (type) {
    case 0: obj=lv_obj_create(parent); lv_obj_remove_style_all(obj); lv_obj_set_style_bg_opa(obj,255,0); break;
    case 1: obj=lv_label_create(parent); lv_label_set_text(obj,content); break;
    case 2: {obj=lv_button_create(parent);lv_obj_t *t=lv_label_create(obj);lv_label_set_text(t,content);lv_obj_center(t);break;}
    case 3: {obj=lv_label_create(parent);const char *symbol=LV_SYMBOL_EYE_OPEN;
        if(!strcmp(content,"wifi"))symbol=LV_SYMBOL_WIFI;else if(!strcmp(content,"settings"))symbol=LV_SYMBOL_SETTINGS;
        else if(!strcmp(content,"home"))symbol=LV_SYMBOL_HOME;else if(!strcmp(content,"play"))symbol=LV_SYMBOL_PLAY;
        else if(!strcmp(content,"folder")||!strcmp(content,"plugins"))symbol=LV_SYMBOL_DIRECTORY;
        else if(!strcmp(content,"theme"))symbol=LV_SYMBOL_TINT;else if(!strcmp(content,"terminal"))symbol=LV_SYMBOL_LIST;
        lv_label_set_text(obj,symbol);lv_obj_set_style_text_align(obj,LV_TEXT_ALIGN_CENTER,0);break;}
    case 4: obj=lv_arc_create(parent);lv_arc_set_value(obj,value);break;
    case 5: obj=lv_bar_create(parent);lv_bar_set_value(obj,value,LV_ANIM_OFF);break;
    case 6: obj=lv_slider_create(parent);lv_slider_set_value(obj,value,LV_ANIM_OFF);break;
    case 7: obj=lv_switch_create(parent);if(value>=50)lv_obj_add_state(obj,LV_STATE_CHECKED);break;
    case 8: obj=lv_checkbox_create(parent);lv_checkbox_set_text(obj,content);if(value>=50)lv_obj_add_state(obj,LV_STATE_CHECKED);break;
    case 9: obj=lv_dropdown_create(parent);lv_dropdown_set_options(obj,content);break;
    case 10: obj=lv_roller_create(parent);lv_roller_set_options(obj,content,LV_ROLLER_MODE_NORMAL);break;
    case 11: obj=lv_textarea_create(parent);lv_textarea_set_text(obj,content);break;
    case 12: obj=lv_spinbox_create(parent);lv_spinbox_set_range(obj,0,100);lv_spinbox_set_digit_format(obj,3,0);lv_spinbox_set_value(obj,value);break;
    case 13: obj=lv_led_create(parent);lv_led_set_color(obj,lv_color_hex(color));lv_led_set_brightness(obj,value*255/100);break;
    case 14: obj=lv_spinner_create(parent);break;
    case 15: obj=lv_line_create(parent);lv_line_set_points(obj,points,2);lv_obj_set_style_line_color(obj,lv_color_hex(color),0);break;
    case 16: {obj=lv_chart_create(parent);lv_chart_set_point_count(obj,8);lv_chart_series_t *series=lv_chart_add_series(obj,lv_color_hex(color),LV_CHART_AXIS_PRIMARY_Y);lv_chart_set_all_value(obj,series,value);break;}
    case 17: obj=lv_table_create(parent);lv_table_set_cell_value(obj,0,0,content);lv_table_set_cell_value(obj,1,0,"Value");break;
    case 18: obj=lv_buttonmatrix_create(parent);lv_buttonmatrix_set_map(obj,matrix);break;
    case 19: obj=lv_list_create(parent);lv_list_add_button(obj,LV_SYMBOL_DIRECTORY,content);break;
    case 20: obj=lv_calendar_create(parent);lv_calendar_set_showed_date(obj,2026,9);break;
    case 21: obj=lv_keyboard_create(parent);for(unsigned i=0;i<index;i++)if(lv_obj_check_type(editor_nodes[i],&lv_textarea_class)){lv_keyboard_set_textarea(obj,editor_nodes[i]);break;}break;
    case 22: {obj=lv_tabview_create(parent);lv_tabview_add_tab(obj,"One");lv_tabview_add_tab(obj,"Two");break;}
    case 23: obj=lv_tileview_create(parent);lv_tileview_add_tile(obj,0,0,LV_DIR_RIGHT);lv_tileview_add_tile(obj,1,0,LV_DIR_LEFT);break;
    case 24: obj=lv_scale_create(parent);lv_scale_set_mode(obj,LV_SCALE_MODE_HORIZONTAL_BOTTOM);lv_scale_set_range(obj,0,100);break;
    case 25: {obj=lv_spangroup_create(parent);lv_span_t *span=lv_spangroup_add_span(obj);lv_span_set_text(span,content);break;}
    case 26: {obj=lv_menu_create(parent);lv_obj_t *p=lv_menu_page_create(obj,NULL);lv_obj_t *l=lv_label_create(p);lv_label_set_text(l,content);lv_menu_set_page(obj,p);break;}
    case 27: obj=lv_msgbox_create(parent);lv_msgbox_add_title(obj,content);lv_msgbox_add_text(obj,"Message");break;
    case 28: obj=lv_win_create(parent);lv_win_add_title(obj,content);break;
    default: return -1;
    }
    if(!obj)return -1;
    editor_nodes[index]=obj;editor_count++;
    lv_obj_set_pos(obj,x,y);lv_obj_set_size(obj,w,h);
    if(type==0||type==2){lv_obj_set_style_bg_color(obj,lv_color_hex(color),0);lv_obj_set_style_radius(obj,radius,0);lv_obj_set_style_pad_all(obj,0,0);lv_obj_clear_flag(obj,LV_OBJ_FLAG_SCROLLABLE);}
    if(type==1||type==3||type==25)lv_obj_set_style_text_color(obj,lv_color_hex(color),0);
    lv_obj_add_event_cb(obj,layout_action,LV_EVENT_ALL,(void *)(uintptr_t)index);
    if(*editor_action[index])lv_obj_add_flag(obj,LV_OBJ_FLAG_CLICKABLE);
    return index;
}
void operit_lvgl_layout_bind(int index,const char *action,const char *long_action) {
    if(index<0 || index>=(int)editor_count)return;
    strncpy(editor_action[index],action ? action : "",23);editor_action[index][23]=0;
    strncpy(editor_long_action[index],long_action ? long_action : "",23);editor_long_action[index][23]=0;
    if(*editor_action[index] || *editor_long_action[index])lv_obj_add_flag(editor_nodes[index],LV_OBJ_FLAG_CLICKABLE);
}
void operit_lvgl_layout_geometry(int index,int x,int y,int w,int h) {
    if(index<0 || index>=(int)editor_count)return;
    lv_obj_set_pos(editor_nodes[index],x,y);lv_obj_set_size(editor_nodes[index],w,h);
}
static void document_gesture(lv_event_t *event) {
    (void)event;lv_dir_t direction=lv_indev_get_gesture_dir(pointer_input);
    const char *target=direction==LV_DIR_LEFT?swipe_left:direction==LV_DIR_RIGHT?swipe_right:"";
    if(*target){char action[32];snprintf(action,sizeof(action),"go:%s",target);queue_route(action);}
}
void operit_lvgl_layout_page_meta(const char *id,const char *left,const char *right) {
    strncpy(active_page,id?id:"home",23);active_page[23]=0;current_page=active_page;
    strncpy(swipe_left,left?left:"",23);swipe_left[23]=0;strncpy(swipe_right,right?right:"",23);swipe_right[23]=0;
    lv_obj_remove_event_cb(root,document_gesture);lv_obj_add_event_cb(root,document_gesture,LV_EVENT_GESTURE,NULL);
}
void operit_lvgl_layout_style(int index,const char *binding,int font_size) {
    if(index<0 || index>=(int)editor_count)return;lv_obj_t *obj=editor_nodes[index];
    if(font_size==48)lv_obj_set_style_text_font(obj,&lv_font_montserrat_48,0);
    if(!lv_obj_check_type(obj,&lv_label_class)||!binding)return;
    if(!strcmp(binding,"clock")){clock_label=obj;tick_clock(NULL);}
    else if(!strcmp(binding,"connection")){connection_label=obj;lv_label_set_text(obj,wifi_ready?"WIFI CONNECTED":"WIFI STARTING");}
    else if(!strcmp(binding,"expression")){face_label=obj;lv_label_set_text(obj,expression);}
    else if(!strcmp(binding,"pairing")){pairing_label=obj;lv_label_set_text(obj,pairing_code[0]?pairing_code:"等待配对码");}
    else if(!strcmp(binding,"space")){space_label=obj;lv_label_set_text(obj,space_state);}
    else if(!strcmp(binding,"chat")){chat_label=obj;lv_label_set_text(obj,chat_preview);}
}
static bool document_page(const char *id) {
    operit_packed_page_t packed;
    if(operit_store_page(id,&packed)){
        operit_lvgl_layout_clear(packed.color);const uint8_t *data=packed.nodes;
        for(unsigned i=0;i<packed.count;i++){
            operit_packed_node_t n;char strings[161+24+24+17];data=operit_store_node(data,&n,strings);
            int index=operit_lvgl_layout_add(n.type,n.parent==255?-1:n.parent,n.x,n.y,n.w,n.h,n.color,n.radius,n.value,n.text,n.action);
            operit_lvgl_layout_bind(index,n.action,n.hold);operit_lvgl_layout_style(index,n.binding,n.font);
        }
        operit_lvgl_layout_page_meta(packed.id,packed.left,packed.right);return true;
    }
    for(unsigned p=0;p<OPERIT_PAGE_COUNT;p++)if(!strcmp(layout_pages[p].id,id)){
        const operit_layout_page_t *page=&layout_pages[p];
        operit_lvgl_layout_clear(page->background);
        for(unsigned i=0;i<page->count;i++){
            const operit_layout_node_t *n=&page->nodes[i];
            int index=operit_lvgl_layout_add(n->type,n->parent,n->x,n->y,n->w,n->h,n->color,n->radius,n->value,n->text,n->action);
            operit_lvgl_layout_bind(index,n->action,n->long_action);
            operit_lvgl_layout_style(index,n->binding,n->font_size);
        }
        operit_lvgl_layout_page_meta(page->id,page->swipe_left,page->swipe_right);return true;
    }
    return false;
}
static void navigate_document(const char *id) {
#ifdef __EMSCRIPTEN__
    if(action_cb){char action[32];snprintf(action,sizeof(action),"navigate:%s",id);action_cb(action,context);return;}
#endif
    document_page(id);
}
static void execute_route(const char *action) {
    if(!strncmp(action,"go:",3)) navigate_document(action+3);
    else if(!strcmp(action,"home")) page("Chat");
    else if(!strcmp(action,"apps")) { sidebar_open = true; page("Chat"); }
    else if(!strncmp(action,"page:",5)) navigate_document(action+5);
    else if(!strcmp(action,"theme_next") || !strcmp(action,"shape_toggle")) {
        if(!strcmp(action,"theme_next")) theme_index=(theme_index+1)%3; else round_icons=!round_icons;
        page("Chat");
    }
    else if(!strncmp(action,"builtin:",8)) page(action+8);
    else if(!strcmp(action,"sidebar_open")) set_sidebar_open(true, true);
    else if(!strcmp(action,"sidebar_close")) set_sidebar_open(false, true);
    else if(!strcmp(action,"sidebar_toggle")) set_sidebar_open(!sidebar_open, true);
    else if(!strcmp(action,"sidebar_width_cycle")) {
        cycle_sidebar_width();
        if(!strcmp(current_page,"Settings")) page("Settings");
        else { bool was_open = sidebar_open; page("Chat"); if (was_open) set_sidebar_open(true, false); }
    }
    else if(!strcmp(action,"chat_keyboard_close")) hide_chat_keyboard();
    else if(!strcmp(action,"edge_new")) {
        chat_draft[0]=0;
        if(chat_input) lv_textarea_set_text(chat_input, "");
        sidebar_open=false;
        page("Chat");
        if(action_cb) action_cb("edge_new",context);
    }
    else if(!strcmp(action,"edge_chat")) { sidebar_open=false; page("Chat"); if(action_cb) action_cb(action,context); }
    else if(!strcmp(action,"edge_search") || !strcmp(action,"edge_pair")) { sidebar_open=false; page("Pairing"); if(action_cb) action_cb(action,context); }
    else if(!strcmp(action,"edge_send")) {
        if (chat_send_pending) return;
        remember_chat_draft();
        if (!chat_draft[0]) { operit_lvgl_set_chat_task("消息不能为空"); return; }
        copy_utf8(submitted_draft, sizeof(submitted_draft), chat_draft);
        chat_send_pending = true;
        hide_chat_keyboard();
        operit_lvgl_set_chat_task("发送中");
        if(action_cb) action_cb(action,context);
        else operit_lvgl_chat_send_result(false, "设备尚未连接");
    }
    else if((!strcmp(action,"face_online") || !strcmp(action,"run_node")) && action_cb) action_cb(action,context);
}
static void document_home(void) { page("Chat"); }
