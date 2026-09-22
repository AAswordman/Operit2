#include "operit_lvgl.h"
#include "lvgl.h"
#include "esp_timer.h"
#include "layout_store.h"
#include <string.h>
#include <stdio.h>
#include <time.h>

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
static const char *current_page = "Home";
static bool round_icons;
static lv_display_t *display;
static lv_obj_t *root, *tiles, *clock_label, *connection_label, *face_label;
static lv_obj_t *pairing_label, *space_label, *chat_label;
static uint16_t touch_x, touch_y;
static bool touch_pressed, wifi_ready, edge_ready;
static char expression[24] = "neutral";
static char pairing_code[20] = "";
static char space_state[40] = "Waiting for Space";
static char chat_preview[96] = "No chat session";
static operit_lvgl_flush_cb_t flush_cb;
static operit_lvgl_action_cb_t action_cb;
static void *context;
static uint8_t draw_buffer[320 * 10 * 2] __attribute__((aligned(4)));
static int64_t last_tick;
static void home(void);
static void builtin_home(void);
static void page(const char *name);
static const theme_t *theme(void) { return &themes[theme_index]; }
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
    return o;
}
static lv_obj_t *label(lv_obj_t *p, const char *text, int x, int y, int w, uint32_t color) {
    lv_obj_t *o = lv_label_create(p);
    lv_label_set_text(o, text); lv_obj_set_pos(o, x, y); lv_obj_set_width(o, w);
    lv_obj_set_style_text_color(o, lv_color_hex(color), 0);
    lv_obj_set_style_text_font(o, LV_FONT_DEFAULT, 0);
    lv_label_set_long_mode(o, LV_LABEL_LONG_CLIP);
    lv_obj_clear_flag(o, LV_OBJ_FLAG_CLICKABLE);
    return o;
}
static void clicked(lv_event_t *e) {
    const char *name = lv_event_get_user_data(e);
    if (!strcmp(name,"Home")) queue_route("home");
    else if (!strcmp(name,"Palette")) queue_route("theme_next");
    else if (!strcmp(name,"Shape")) queue_route("shape_toggle");
    else if (!strcmp(name,"Online")) queue_route("face_online");
    else if (!strcmp(name,"Run")) queue_route("run_node");
    else if (!strcmp(name,"edge_search") || !strcmp(name,"edge_pair") || !strcmp(name,"edge_chat") || !strcmp(name,"edge_send")) queue_route(name);
    else { char route[32];snprintf(route,sizeof(route),"builtin:%s",name);queue_route(route); }
}
static lv_obj_t *button(lv_obj_t *p, const char *text, const char *action, int x, int y, int w, int h) {
    lv_obj_t *o = box(p, x, y, w, h, theme()->surface, 12);
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
    pairing_label = space_label = chat_label = NULL;
    lv_obj_clean(root);
    lv_obj_set_style_bg_color(root, lv_color_hex(theme()->bg), 0);
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
    current_page = "Home";
    clear();
    tiles = lv_tileview_create(root); lv_obj_remove_style_all(tiles);
    lv_obj_set_size(tiles, 320, 240); lv_obj_set_scrollbar_mode(tiles, LV_SCROLLBAR_MODE_OFF);
    lv_obj_set_style_anim_duration(tiles, 240, 0);
    lv_obj_t *watch = lv_tileview_add_tile(tiles, 0, 0, LV_DIR_RIGHT);
    lv_obj_t *apps = lv_tileview_add_tile(tiles, 1, 0, LV_DIR_LEFT);
    lv_obj_set_style_pad_all(watch, 0, 0); lv_obj_set_style_pad_all(apps, 0, 0);
    label(watch, "OPERIT / EDGE", 18, 16, 190, theme()->accent);
    label(watch, theme()->name, 242, 16, 74, theme()->muted);
    box(watch, 18, 48, 284, 111, theme()->surface, 22);
    clock_label = label(watch, "00:00", 31, 56, 262, 0xf4f8ff);
    lv_obj_set_style_text_font(clock_label, &lv_font_montserrat_48, 0);
    lv_obj_set_style_text_align(clock_label, LV_TEXT_ALIGN_CENTER, 0);
    label(watch, "DEVICE TIME", 73, 119, 220, theme()->muted);
    connection_label = label(watch, wifi_ready ? "WIFI CONNECTED" : "WIFI STARTING", 24, 177, 190, theme()->accent);
    label(watch, "Swipe left  >", 195, 207, 110, theme()->muted);
    box(watch, 141, 225, 20, 3, theme()->accent, 2);
    box(watch, 166, 225, 7, 3, theme()->surface, 2);
    label(apps, "YOUR SPACE", 18, 13, 180, theme()->accent);
    label(apps, "<  swipe back", 203, 13, 112, theme()->muted);
    icon(apps, LV_SYMBOL_EYE_OPEN, "Face", 30, 46, 0x216c73);
    icon(apps, LV_SYMBOL_DIRECTORY, "Plugins", 134, 46, 0x435989);
    icon(apps, LV_SYMBOL_TINT, "Theme", 238, 46, 0x895573);
    icon(apps, LV_SYMBOL_SETTINGS, "Settings", 30, 139, 0x516344);
    icon(apps, LV_SYMBOL_LIST, "Terminal", 134, 139, 0x795636);
    icon(apps, LV_SYMBOL_WIFI, "Network", 238, 139, 0x285f8a);
    tick_clock(NULL);
}
static void page(const char *name) {
    current_page = name;
    clear();
    button(root, LV_SYMBOL_LEFT, "Home", 12, 10, 32, 32);
    label(root, name, 56, 17, 246, theme()->accent);
    if (!strcmp(name, "Theme")) {
        label(root, "COLOR PALETTE", 20, 58, 250, theme()->muted);
        button(root, theme()->name, "Palette", 20, 84, 280, 44);
        label(root, "ICON GEOMETRY", 20, 144, 250, theme()->muted);
        button(root, round_icons ? "Circle" : "Rounded square", "Shape", 20, 170, 280, 44);
    } else if (!strcmp(name, "Face")) {
        lv_obj_t *eyes = label(root, "o   o", 20, 65, 280, theme()->accent);
        lv_obj_set_style_text_font(eyes, &lv_font_montserrat_48, 0);
        lv_obj_set_style_text_align(eyes, LV_TEXT_ALIGN_CENTER, 0);
        face_label = label(root, expression, 20, 136, 280, theme()->muted);
        lv_obj_set_style_text_align(face_label, LV_TEXT_ALIGN_CENTER, 0);
        button(root, "Online", "Online", 90, 180, 140, 40);
    } else if (!strcmp(name, "Settings") || !strcmp(name, "Network")) {
        label(root, wifi_ready ? "Wi-Fi connected" : "Wi-Fi setup needed", 20, 66, 280, 0xf4f8ff);
        space_label = label(root, edge_ready ? space_state : "Edge Link waiting", 20, 98, 280, theme()->muted);
        pairing_label = label(root, pairing_code[0] ? pairing_code : "Pairing: open Space on a nearby Core", 20, 126, 280, theme()->accent);
        chat_label = label(root, chat_preview, 20, 151, 280, theme()->muted);
        button(root, "Search Space", "edge_search", 20, 180, 88, 40);
        button(root, "Pair", "edge_pair", 116, 180, 88, 40);
        button(root, "Chat", "edge_chat", 212, 180, 88, 40);
    } else if (!strcmp(name, "Chat")) {
        label(root, "SPACE CHAT", 20, 60, 280, theme()->accent);
        chat_label = label(root, chat_preview, 20, 98, 280, theme()->muted);
        button(root, "Network", "Network", 20, 188, 88, 36);
        button(root, "Refresh", "edge_chat", 116, 188, 88, 36);
    } else if (!strcmp(name, "Terminal")) {
        label(root, "> Operit Edge ready", 20, 70, 280, theme()->accent);
        label(root, "Local display + Wi-Fi", 20, 106, 280, theme()->muted);
        button(root, "Run node", "Run", 20, 180, 280, 40);
    } else { label(root, "No plugins installed", 20, 92, 280, theme()->muted); }
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
void operit_lvgl_set_touch(uint16_t x,uint16_t y,bool pressed) {touch_x=x;touch_y=y;touch_pressed=pressed;}
void operit_lvgl_navigate_home(void) {home();}
void operit_lvgl_set_connection(bool wifi,bool edge) {
    if(wifi==wifi_ready && edge==edge_ready) return;
    wifi_ready=wifi; edge_ready=edge;
    if(connection_label) lv_label_set_text(connection_label,wifi ? "WIFI CONNECTED" : "WIFI STARTING");
}
void operit_lvgl_set_pairing_code(const char *code) {
    const char *value = code ? code : "";
    if (!strcmp(pairing_code, value)) return;
    strncpy(pairing_code, value, sizeof(pairing_code)-1); pairing_code[sizeof(pairing_code)-1]=0;
    if (pairing_label) lv_label_set_text(pairing_label, pairing_code[0] ? pairing_code : "Pairing: open Space on a nearby Core");
}
void operit_lvgl_set_space_state(const char *state) {
    const char *value = state ? state : "Waiting for Space";
    if (!strcmp(space_state, value)) return;
    strncpy(space_state, value, sizeof(space_state)-1); space_state[sizeof(space_state)-1]=0;
    if (space_label) lv_label_set_text(space_label, space_state);
}
void operit_lvgl_set_chat_preview(const char *preview) {
    const char *value = preview ? preview : "No chat session";
    if (!strcmp(chat_preview, value)) return;
    strncpy(chat_preview, value, sizeof(chat_preview)-1); chat_preview[sizeof(chat_preview)-1]=0;
    if (chat_label) lv_label_set_text(chat_label, chat_preview);
}
void operit_lvgl_set_expression(const char *value) {
    if(!value || !strcmp(expression,value)) return;
    strncpy(expression,value,sizeof(expression)-1); expression[sizeof(expression)-1]=0;
    if(face_label) lv_label_set_text(face_label,expression);
}

/* Host controls shared by the firmware and the WebAssembly developer host. */
void operit_lvgl_set_theme(unsigned index, bool circular) {
    theme_index = index % (sizeof(themes) / sizeof(themes[0]));
    round_icons = circular;
    home();
}
void operit_lvgl_navigate_apps(void) {
    if(OPERIT_LAYOUT_ENABLED && document_page("apps"))return;
    builtin_home();
    lv_tileview_set_tile_by_index(tiles, 1, 0, LV_ANIM_ON);
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
    else if(!strcmp(binding,"pairing")){pairing_label=obj;lv_label_set_text(obj,pairing_code[0]?pairing_code:"Pairing code: waiting");}
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
    if(!strncmp(action,"go:",3))navigate_document(action+3);
    else if(!strcmp(action,"home"))navigate_document(operit_store_entry()?operit_store_entry():OPERIT_LAYOUT_ENTRY);
    else if(!strcmp(action,"apps"))navigate_document("apps");
    else if(!strncmp(action,"page:",5))navigate_document(action+5);
    else if(!strcmp(action,"theme_next")||!strcmp(action,"shape_toggle")){
        if(!strcmp(action,"theme_next"))theme_index=(theme_index+1)%3;else round_icons=!round_icons;
        if(*active_page){char id[24];strcpy(id,active_page);navigate_document(id);}else home();
    }
    else if(!strncmp(action,"builtin:",8))page(action+8);
    else if(!strcmp(action,"edge_chat")){page("Chat");if(action_cb)action_cb(action,context);}
    else if(!strcmp(action,"edge_search")||!strcmp(action,"edge_pair")||!strcmp(action,"edge_send")){if(action_cb)action_cb(action,context);}
    else if((!strcmp(action,"face_online")||!strcmp(action,"run_node"))&&action_cb)action_cb(action,context);
}
static void document_home(void) { const char *entry=operit_store_entry();if(!document_page(entry?entry:OPERIT_LAYOUT_ENTRY))builtin_home(); }
