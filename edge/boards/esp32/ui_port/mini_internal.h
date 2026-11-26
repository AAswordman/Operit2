/* Private renderer state; the public embedding ABI stays in operit_ui.h. */
#ifndef OPERIT_MINI_INTERNAL_H
#define OPERIT_MINI_INTERNAL_H
#include "operit_mini_ui.h"
#include <string.h>

enum { WIDTH = 320, HEIGHT = 240, ROWS = 2, NODE_LIMIT = 14,
       CHAT_X = 126, CHAT_W = 182, CHAT_WRAP = CHAT_W - 10, CHAT_ROWS = 6 };
enum { ICON_NONE, ICON_MENU, ICON_CLOSE, ICON_BACK, ICON_CHAT, ICON_FACE,
       ICON_SETTINGS, ICON_LINK, ICON_SPACE, ICON_DEVICE, ICON_SEND, ICON_BOLT, ICON_MIC };
typedef struct {
    const char *id, *text, *action;
    int16_t x, y, w, h;
    uint16_t color;
    uint8_t scale, icon;
    bool enabled;
} node_t;
typedef struct {
    operit_ui_scene_cb_t scene_painter;
    bool scene_active, scene_press;
    uint32_t scene_revision, scene_tick, frame_tick;
    operit_ui_flush_cb_t flush;
    operit_ui_action_cb_t action;
    void *context;
    bool wifi, connected, paired, busy, pressed, dirty, refresh_again, sending, sidebar, dimmed;
    uint8_t expression, speech_phase;
    int page, pressed_node, render_y;
    uint16_t press_x, press_y;
    unsigned node_count, message_count, chat_scroll;
    bool chat_follow_tail;
    char code[7], prompt[256], space[96], chat[1024], task[128];
    char error[192], draft[512], chat_id[97];
    uint8_t device_space_phase;
    bool device_space_pending;
    char device_space_status[112], device_space_peers[3][40], pair_input[7];
    char submitted[512];
    node_t nodes[NODE_LIMIT];
} mini_state_t;
extern mini_state_t mini_ui;
extern uint8_t mini_strip[WIDTH * ROWS * 2];
static inline uint16_t rgb(unsigned c) { return (uint16_t)(((c >> 8) & 0xf800) | ((c >> 5) & 0x07e0) | ((c >> 3) & 31)); }
static inline bool same(const char *a,const char *b){return !strcmp(a,b);}
void mini_invalidate(void);
bool mini_copy(char *dst, size_t capacity, const char *src);
int mini_page(void);
void mini_scene(void);
unsigned mini_chat_lines(void);
const char *mini_chat_line(unsigned wanted);
#endif
