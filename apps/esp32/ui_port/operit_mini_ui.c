/* Allocator-free self-drawn UI. Fixed 320x240 RGB565, two-row strips.
 * Pairing and space decisions remain host actions, never local UI decisions.
 * SVG face primitives are compiled into flash; no runtime image decoder or UI heap.
 */
#include "operit_mini_ui.h"
#include "mini_font.h"
#include <stdio.h>
#include <stdarg.h>
#include <string.h>

enum { WIDTH = 320, HEIGHT = 240, ROWS = 2, NODE_LIMIT = 10,
       CHAT_X = 126, CHAT_W = 182, CHAT_WRAP = CHAT_W - 10, CHAT_ROWS = 6 };
enum { ICON_NONE, ICON_MENU, ICON_CLOSE, ICON_BACK, ICON_CHAT, ICON_FACE,
       ICON_PLUGIN, ICON_SETTINGS, ICON_LINK, ICON_SPACE, ICON_DEVICE, ICON_SEND };
typedef struct {
    const char *id, *text, *action;
    int16_t x, y, w, h;
    uint16_t color;
    uint8_t scale, icon;
    bool enabled;
} node_t;
typedef struct {
    operit_ui_flush_cb_t flush;
    operit_ui_action_cb_t action;
    void *context;
    bool wifi, connected, paired, busy, pressed, dirty, refresh_again, sending, sidebar, dimmed;
    uint8_t expression;
    int page, pressed_node, render_y;
    uint16_t press_x, press_y;
    unsigned node_count, message_count, chat_scroll;
    bool chat_follow_tail, chat_has_older, chat_has_newer;
    char code[7], prompt[256], space[96], chat[2048], task[128];
    char error[192], draft[512], submitted[512];
    node_t nodes[NODE_LIMIT];
} mini_state_t;
typedef struct {
    uint8_t kind;
    int16_t a, b, c, d;
    uint8_t width;
    uint32_t color;
} face_primitive_t;
#include "mini_face.h"
static mini_state_t ui;
static uint8_t strip[WIDTH * ROWS * 2];
/* A bounded inspection buffer is included in the real device RAM budget. */
static char debug_json[3072];
static size_t json_at;
static bool json_ok;
_Static_assert(sizeof(ui) + sizeof(strip) + sizeof(debug_json) + sizeof(json_at) + sizeof(json_ok) <= 10 * 1024,
               "Mini UI static RAM must remain below 10 KiB");
size_t operit_mini_static_bytes(void) { return sizeof(ui) + sizeof(strip) + sizeof(debug_json) + sizeof(json_at) + sizeof(json_ok); }
size_t operit_mini_draw_bytes(void) { return sizeof(strip); }
static uint16_t rgb(unsigned c) { return (uint16_t)(((c >> 8) & 0xf800) | ((c >> 5) & 0x07e0) | ((c >> 3) & 31)); }
static void invalidate(void) {
    if (!ui.dirty) ui.render_y = 0;
    else if (ui.render_y > 0) ui.refresh_again = true;
    ui.dirty = true; ui.pressed_node = -1;
}
/* Copy whole UTF-8 characters; never leave a truncated multibyte glyph. */
static bool copy(char *dst, size_t capacity, const char *src) {
    if (!src) src = "";
    size_t n = 0;
    while (src[n] && n + 1 < capacity) {
        unsigned char c = (unsigned char)src[n];
        size_t count = c < 128 ? 1 : (c & 0xe0) == 0xc0 ? 2 : (c & 0xf0) == 0xe0 ? 3 : (c & 0xf8) == 0xf0 ? 4 : 1;
        if (n + count >= capacity) break;
        bool valid = true;
        for (size_t i = 1; i < count; ++i) if (!src[n+i] || ((unsigned char)src[n+i] & 0xc0) != 0x80) { valid = false; break; }
        n += valid ? count : 1;
    }
    if (strlen(dst) == n && !memcmp(dst, src, n)) return false;
    memcpy(dst, src, n); dst[n] = 0; return true;
}
static uint32_t decode(const char **text) {
    const unsigned char *s = (const unsigned char *)*text;
    unsigned n; uint32_t cp;
    if (*s < 128) { *text += 1; return *s; }
    if ((*s & 0xe0) == 0xc0) { n = 2; cp = *s & 31; }
    else if ((*s & 0xf0) == 0xe0) { n = 3; cp = *s & 15; }
    else if ((*s & 0xf8) == 0xf0) { n = 4; cp = *s & 7; }
    else { *text += 1; return '?'; }
    for (unsigned i = 1; i < n; ++i) {
        if (!s[i] || (s[i] & 0xc0) != 0x80) { *text += 1; return '?'; }
        cp = (cp << 6) | (s[i] & 63);
    }
    *text += n;
    if (cp > 0x10ffff || (cp >= 0xd800 && cp <= 0xdfff) ||
        (n == 2 && cp < 128) || (n == 3 && cp < 2048) || (n == 4 && cp < 65536)) return '?';
    return cp;
}
static const mini_glyph_t *lookup(uint32_t cp, bool digits) {
    const mini_glyph_t *table = digits ? mini_digits_glyphs : mini_text_glyphs;
    size_t count = digits ? sizeof(mini_digits_glyphs)/sizeof(*table) : sizeof(mini_text_glyphs)/sizeof(*table);
    size_t lo = 0, hi = count;
    while (lo < hi) { size_t mid = (lo + hi)/2; if (table[mid].codepoint < cp) lo = mid+1; else hi = mid; }
    if (lo < count && table[lo].codepoint == cp) return table+lo;
    return NULL;
}
static const mini_glyph_t *text_glyph(uint32_t cp) {
    const mini_glyph_t *g = lookup(cp, false);
    return g ? g : lookup('?', false);
}
static void pixel(int x, int y, uint16_t c) {
    if(ui.dimmed)c=(c&0xf7de)>>1;
    if (x < 0 || x >= WIDTH || y < ui.render_y || y >= ui.render_y + ROWS || y >= HEIGHT) return;
    unsigned at = (unsigned)((y-ui.render_y)*WIDTH+x)*2;
    strip[at] = (uint8_t)c; strip[at+1] = (uint8_t)(c >> 8);
}
static void rect(int x, int y, int w, int h, uint16_t color) {
    int top = y > ui.render_y ? y : ui.render_y;
    int end = y+h < ui.render_y+ROWS ? y+h : ui.render_y+ROWS;
    int left = x > 0 ? x : 0, right = x+w < WIDTH ? x+w : WIDTH;
    for (int row = top; row < end; ++row) for (int col = left; col < right; ++col) pixel(col,row,color);
}
static void text(const node_t *node) {
    const char *s = node->text;
    int x = node->x+6, y = node->y+4, scale = node->scale ? node->scale : 1;
    bool digits = scale == 2;
    while (s && *s) {
        uint32_t cp = decode(&s);
        if (cp == 0x200d || cp == 0xfe0e || cp == 0xfe0f) continue;
        const mini_glyph_t *g = digits ? lookup(cp,true) : text_glyph(cp);
        int line = digits ? 36 : 21;
        if (cp == '\n') { x = node->x+6; y += line; continue; }
        if (!g) continue;
        int advance = g->advance;
        if (x + advance > node->x+node->w-4) { x = node->x+6; y += line; }
        if (y+line > node->y+node->h+3) break;
        int gx = x + g->x, gy = y + g->line-g->baseline-g->h-g->y;
        if (gy+g->h > ui.render_y && gy < ui.render_y+ROWS) {
            const uint8_t *bits = (digits ? mini_digits_bitmap : mini_text_bitmap) + g->offset;
            for (unsigned row=0; row<g->h; ++row) {
                int py=gy+(int)row;
                if (py < ui.render_y || py >= ui.render_y+ROWS) continue;
                for (unsigned col=0; col<g->w; ++col) {
                    unsigned bit=(row*g->w+col)*g->bpp;
                    unsigned value=(bits[bit/8] >> (8-g->bpp-bit%8)) & ((1u<<g->bpp)-1);
                    if (value) pixel(gx+(int)col,py,node->color);
                }
            }
        }
        x += advance;
    }
}
/* Match the actual painter's wrapping, not message count or byte estimates. */
static void follow_chat_tail(void);
static unsigned chat_lines(void) {
    const char *s=ui.chat;unsigned lines=1;int x=0;
    while(*s) {uint32_t cp=decode(&s);if(cp=='\n'){++lines;x=0;continue;}
        const mini_glyph_t *g=text_glyph(cp);if(!g)continue;
        if(x+g->advance>CHAT_WRAP){++lines;x=0;}x+=g->advance;}
    return lines;
}
static const char *chat_line(unsigned wanted) {
    const char *s=ui.chat;unsigned line=0;int x=0;if(!wanted)return s;
    while(*s) {const char *before=s;uint32_t cp=decode(&s);
        if(cp=='\n'){++line;x=0;if(line==wanted)return s;continue;}
        const mini_glyph_t *g=text_glyph(cp);if(!g)continue;
        if(x+g->advance>CHAT_WRAP){++line;x=0;if(line==wanted)return before;}x+=g->advance;}
    return s;
}
static int page(void);
static bool chat_swipe(const char *direction) {
    if(page()!=2)return false;
    unsigned lines=chat_lines(),last=lines>CHAT_ROWS?lines-CHAT_ROWS:0;
    if(!strcmp(direction,"down")) {
        if(ui.chat_scroll){ui.chat_scroll=ui.chat_scroll>CHAT_ROWS?ui.chat_scroll-CHAT_ROWS:0;ui.chat_follow_tail=false;}
        else if(ui.chat_has_older&&ui.action){ui.chat_follow_tail=true;ui.action("edge_history_older",ui.context);}
    } else if(!strcmp(direction,"up")) {
        if(ui.chat_scroll<last){ui.chat_scroll+=CHAT_ROWS;if(ui.chat_scroll>last)ui.chat_scroll=last;ui.chat_follow_tail=ui.chat_scroll==last;}
        else if(ui.chat_has_newer&&ui.action){ui.chat_scroll=0;ui.chat_follow_tail=false;ui.action("edge_history_newer",ui.context);}
    } else return false;
    invalidate();return true;
}
static int page(void) { return ui.code[0] ? 0 : ui.prompt[0] ? 3 : ui.sidebar ? 8 : ui.page; }
const char *operit_ui_current_page(void) {
    switch(page()) {
        case 0:return "Pairing"; case 1:return "Settings"; case 2:return "Chat";
        case 3:return "Space"; case 4:return "Unpair"; case 5:return "LeaveSpace";
        case 6:return "Plugins"; case 7:return "Expression"; case 8:return "Sidebar";
        case 9:return "Connection"; case 10:return "SpaceSettings"; default:return "DeviceSettings";
    }
}
static const char *page_title(void) {
    switch(page()) {
        case 0:return "配对码"; case 1:return "设置"; case 2:return "Operit";
        case 3:return "空间审批"; case 4:return "解除配对"; case 5:return "退出空间";
        case 6:return "插件"; case 7:return "表情"; case 8:return "Operit";
        case 9:return "连接与配对"; case 10:return "设备空间"; default:return "设备操作";
    }
}
static void node(const char *id,const char *caption,const char *action,int x,int y,int w,int h,bool enabled,int scale) {
    if (ui.node_count == NODE_LIMIT) return;
    ui.nodes[ui.node_count++] = (node_t){id,caption,action,x,y,w,h,rgb(enabled ? 0xecf3f0 : 0x647c6d),scale,ICON_NONE,enabled};
}
static void icon_node(const char *id,const char *caption,const char *action,int x,int y,int w,int h,bool enabled,unsigned icon) {
    node(id,caption,action,x,y,w,h,enabled,1);
    ui.nodes[ui.node_count-1].icon=icon;
}
static void scene(void) {
    ui.node_count=0;
    if(page()==8 && !ui.error[0]) {
        node("title","Operit",NULL,10,8,118,26,true,1);
        icon_node("sidebar_toggle","","close_sidebar",136,0,44,40,true,ICON_CLOSE);
        node("sidebar_face","","expression",49,43,82,51,true,1);
        icon_node("sidebar_expression","表情","expression",12,102,156,36,true,ICON_FACE);
        icon_node("sidebar_chat","表情 / 对话","chat",12,143,156,36,true,ICON_CHAT);
        icon_node("sidebar_plugins","插件","plugins",12,190,48,44,true,ICON_PLUGIN);
        icon_node("sidebar_settings","设置","settings",120,190,48,44,true,ICON_SETTINGS);
        node("sidebar_backdrop","","close_sidebar",180,0,140,240,true,1);
        return;
    }
    bool detail=page()==9||page()==10||page()==11||(page()==0&&!ui.code[0]);
    bool menuEnabled=!ui.code[0]&&!ui.prompt[0]&&page()!=4&&page()!=5;
    if(detail)icon_node("page_back","","settings",0,0,44,42,true,ICON_BACK);
    else icon_node("sidebar_toggle","","sidebar",0,0,44,42,menuEnabled,ICON_MENU);
    node("title",page_title(),NULL,44,6,142,30,true,1);
    node("link_status",ui.connected ? "已连接" : ui.paired ? "已配对 / 离线" : "未配对",NULL,188,6,130,30,true,1);
    if (ui.error[0] && !ui.code[0] && !ui.prompt[0]) {
        node("error",ui.error,NULL,16,50,288,122,true,1);
        node("dismiss_error","返回","dismiss",16,184,288,40,true,1);
    } else if (page()==0) {
        node("pairing_hint","请在 Operit 中输入验证码",NULL,16,54,288,46,true,1);
        node("pairing_code",ui.code[0] ? ui.code : "等待配对请求",NULL,90,112,224,48,true,ui.code[0] ? 2 : 1);
        node("pairing_settings","连接与配对",NULL,106,186,176,30,true,1);
    } else if (page()==3) {
        node("space_join_body",ui.busy ? "正在处理，请稍候…" : ui.prompt,NULL,16,46,288,ui.error[0]&&!ui.busy?82:123,true,1);
        if(ui.error[0]&&!ui.busy) node("space_join_error",ui.error,NULL,16,130,288,46,true,1);
        node("edge_space_reject","拒绝","edge_space_reject",16,184,138,40,!ui.busy,1);
        node("edge_space_approve","确认加入","edge_space_approve",166,184,138,40,!ui.busy,1);
    } else if (page()==4) {
        node("unpair_warning","解除配对后需重新连接。\n确认解除当前设备配对？",NULL,16,56,288,108,true,1);
        node("unpair_cancel","返回","operations",16,184,138,40,true,1);
        node("unpair_confirm","确认解除","confirm_unpair",166,184,138,40,true,1);
    } else if (page()==5) {
        node("space_leave_warning","退出当前设备空间？\n配对和 Wi-Fi 将保留。\n新空间加入仍需审批。",NULL,16,56,288,108,true,1);
        node("space_leave_cancel","返回","space_settings",16,184,138,40,true,1);
        node("space_leave_confirm","确认退出","confirm_space_leave",166,184,138,40,true,1);
    } else if (page()==1) {
        node("settings_label","设备设置",NULL,16,45,288,24,true,1);
        icon_node("settings_connection","连接与配对","connection",14,75,292,44,true,ICON_LINK);
        icon_node("settings_space","设备空间","space_settings",14,122,292,44,true,ICON_SPACE);
        icon_node("settings_device","设备操作","operations",14,169,292,44,true,ICON_DEVICE);
    } else if (page()==9) {
        node("wifi",ui.wifi ? "Wi-Fi 已连接" : "Wi-Fi 未连接",NULL,16,50,288,26,true,1);
        icon_node("pairing_open","查看配对码","pairing",14,90,292,44,true,ICON_LINK);
        icon_node("pair_listen","开始监听","edge_pair",14,140,292,44,true,ICON_DEVICE);
    } else if (page()==10) {
        node("space",ui.space,NULL,16,50,288,66,true,1);
        icon_node("space_leave","退出设备空间","space_leave",14,136,292,44,!ui.busy,ICON_SPACE);
    } else if (page()==11) {
        icon_node("edge_new","新建对话","edge_new",14,66,292,44,ui.connected&&!ui.sending,ICON_CHAT);
        icon_node("edge_unpair","解除配对","edge_unpair",14,122,292,44,ui.paired,ICON_LINK);
    } else if (page()==6) {
        icon_node("plugins_icon","",NULL,145,95,30,30,true,ICON_PLUGIN);
        node("plugins_empty","暂无插件",NULL,126,138,94,26,true,1);
    } else if (page()==7) {
        node("expression_face","",NULL,70,68,180,112,true,1);
        node("expression_caption","Operit",NULL,134,191,100,26,true,1);
    } else {
        node("home_face","",NULL,8,92,104,65,true,1);
        node("chat_task",ui.task[0]?ui.task:ui.connected?"就绪":"离线",NULL,14,170,98,50,true,1);
        node("chat_text",ui.chat[0] ? chat_line(ui.chat_scroll) : "连接 Operit 后\n开始聊天",NULL,CHAT_X,44,CHAT_W,132,true,1);
        node("chat_draft",ui.draft[0]?ui.draft:"电脑输入…",NULL,CHAT_X,191,132,34,true,1);
        icon_node("edge_send","","edge_send",268,187,40,40,ui.connected&&ui.draft[0]&&!ui.sending,ICON_SEND);
    }
}
/* Small integer-only SVG painter. Geometry lives in flash, not a framebuffer. */
static void vector_line(int x,int y,int endX,int endY,int width,uint16_t color) {
    int dx=endX>x?endX-x:x-endX, sx=x<endX?1:-1;
    int dy=endY>y?y-endY:endY-y, sy=y<endY?1:-1, error=dx+dy;
    for (;;) {
        rect(x-width/2,y-width/2,width,width,color);
        if(x==endX&&y==endY)break;
        int twice=2*error;
        if(twice>=dy){error+=dy;x+=sx;}
        if(twice<=dx){error+=dx;y+=sy;}
    }
}
static void face(const node_t *box) {
    for(unsigned i=0;i<sizeof(face_primitives)/sizeof(*face_primitives);++i) {
        const face_primitive_t *p=face_primitives+i;
        uint16_t color=rgb(p->color);
        if(p->kind==0) {
            int cx=box->x+p->a*box->w/160,cy=box->y+p->b*box->h/100;
            int rx=p->c*box->w/160,ry=p->d*box->h/100;
            if(ui.expression==2)ry=ry/4+1;
            if(ui.expression==1)ry=ry*3/4;
            int top=ui.render_y-cy,bottom=ui.render_y+ROWS-1-cy;
            if(top < -ry)top=-ry;if(bottom>ry)bottom=ry;
            for(int y=top;y<=bottom;++y)for(int x=-rx;x<=rx;++x)
                if(x*x*ry*ry+y*y*rx*rx<=rx*rx*ry*ry)pixel(cx+x,cy+y,color);
        } else {
            int y1=p->b,y2=p->d;
            if(ui.expression==1){y1=146-y1;y2=146-y2;}
            int width=p->width*box->w/160;if(width<1)width=1;
            vector_line(box->x+p->a*box->w/160,box->y+y1*box->h/100,
                box->x+p->c*box->w/160,box->y+y2*box->h/100,width,color);
        }
    }
}
typedef struct { uint8_t icon, x1, y1, x2, y2; } icon_line_t;
/* Shared flash-resident vectors, not font symbols or allocated SVG images. */
static const icon_line_t icon_lines[] = {
    {ICON_MENU, 4, 7, 20, 7},
    {ICON_MENU, 4, 12, 20, 12},
    {ICON_MENU, 4, 17, 20, 17},
    {ICON_CLOSE, 7, 7, 17, 17},
    {ICON_CLOSE, 17, 7, 7, 17},
    {ICON_BACK, 14, 5, 7, 12},
    {ICON_BACK, 7, 12, 14, 19},
    {ICON_CHAT, 4, 4, 20, 4},
    {ICON_CHAT, 20, 4, 20, 17},
    {ICON_CHAT, 20, 17, 9, 17},
    {ICON_CHAT, 9, 17, 4, 21},
    {ICON_CHAT, 4, 21, 4, 4},
    {ICON_CHAT, 8, 9, 16, 9},
    {ICON_CHAT, 8, 13, 13, 13},
    {ICON_FACE, 8, 8, 8, 10},
    {ICON_FACE, 16, 8, 16, 10},
    {ICON_FACE, 7, 14, 10, 17},
    {ICON_FACE, 10, 17, 14, 17},
    {ICON_FACE, 14, 17, 17, 14},
    {ICON_FACE, 7, 3, 17, 3},
    {ICON_FACE, 17, 3, 21, 7},
    {ICON_FACE, 21, 7, 21, 17},
    {ICON_FACE, 21, 17, 17, 21},
    {ICON_FACE, 17, 21, 7, 21},
    {ICON_FACE, 7, 21, 3, 17},
    {ICON_FACE, 3, 17, 3, 7},
    {ICON_FACE, 3, 7, 7, 3},
    {ICON_PLUGIN, 3, 3, 9, 3},
    {ICON_PLUGIN, 9, 3, 9, 1},
    {ICON_PLUGIN, 9, 1, 15, 1},
    {ICON_PLUGIN, 15, 1, 15, 3},
    {ICON_PLUGIN, 15, 3, 21, 3},
    {ICON_PLUGIN, 21, 3, 21, 9},
    {ICON_PLUGIN, 21, 9, 18, 9},
    {ICON_PLUGIN, 18, 9, 18, 15},
    {ICON_PLUGIN, 18, 15, 21, 15},
    {ICON_PLUGIN, 21, 15, 21, 21},
    {ICON_PLUGIN, 21, 21, 15, 21},
    {ICON_PLUGIN, 15, 21, 15, 18},
    {ICON_PLUGIN, 15, 18, 9, 18},
    {ICON_PLUGIN, 9, 18, 9, 21},
    {ICON_PLUGIN, 9, 21, 3, 21},
    {ICON_PLUGIN, 3, 21, 3, 15},
    {ICON_PLUGIN, 3, 15, 6, 15},
    {ICON_PLUGIN, 6, 15, 6, 9},
    {ICON_PLUGIN, 6, 9, 3, 9},
    {ICON_PLUGIN, 3, 9, 3, 3},
    {ICON_SETTINGS, 9, 2, 15, 2},
    {ICON_SETTINGS, 15, 2, 16, 5},
    {ICON_SETTINGS, 16, 5, 19, 5},
    {ICON_SETTINGS, 19, 5, 22, 10},
    {ICON_SETTINGS, 22, 10, 20, 12},
    {ICON_SETTINGS, 20, 12, 22, 14},
    {ICON_SETTINGS, 22, 14, 19, 19},
    {ICON_SETTINGS, 19, 19, 16, 19},
    {ICON_SETTINGS, 16, 19, 15, 22},
    {ICON_SETTINGS, 15, 22, 9, 22},
    {ICON_SETTINGS, 9, 22, 8, 19},
    {ICON_SETTINGS, 8, 19, 5, 19},
    {ICON_SETTINGS, 5, 19, 2, 14},
    {ICON_SETTINGS, 2, 14, 4, 12},
    {ICON_SETTINGS, 4, 12, 2, 10},
    {ICON_SETTINGS, 2, 10, 5, 5},
    {ICON_SETTINGS, 5, 5, 8, 5},
    {ICON_SETTINGS, 8, 5, 9, 2},
    {ICON_SETTINGS, 10, 8, 14, 8},
    {ICON_SETTINGS, 14, 8, 16, 10},
    {ICON_SETTINGS, 16, 10, 16, 14},
    {ICON_SETTINGS, 16, 14, 14, 16},
    {ICON_SETTINGS, 14, 16, 10, 16},
    {ICON_SETTINGS, 10, 16, 8, 14},
    {ICON_SETTINGS, 8, 14, 8, 10},
    {ICON_SETTINGS, 8, 10, 10, 8},
    {ICON_LINK, 10, 14, 14, 10},
    {ICON_LINK, 6, 16, 4, 18},
    {ICON_LINK, 4, 18, 1, 15},
    {ICON_LINK, 1, 15, 1, 12},
    {ICON_LINK, 1, 12, 6, 7},
    {ICON_LINK, 6, 7, 9, 7},
    {ICON_LINK, 9, 7, 12, 10},
    {ICON_LINK, 12, 14, 15, 17},
    {ICON_LINK, 15, 17, 18, 17},
    {ICON_LINK, 18, 17, 23, 12},
    {ICON_LINK, 23, 12, 23, 9},
    {ICON_LINK, 23, 9, 20, 6},
    {ICON_LINK, 20, 6, 18, 8},
    {ICON_SPACE, 12, 3, 22, 8},
    {ICON_SPACE, 22, 8, 12, 13},
    {ICON_SPACE, 12, 13, 2, 8},
    {ICON_SPACE, 2, 8, 12, 3},
    {ICON_SPACE, 2, 13, 12, 18},
    {ICON_SPACE, 12, 18, 22, 13},
    {ICON_SPACE, 2, 18, 12, 23},
    {ICON_SPACE, 12, 23, 22, 18},
    {ICON_DEVICE, 5, 2, 19, 2},
    {ICON_DEVICE, 19, 2, 19, 22},
    {ICON_DEVICE, 19, 22, 5, 22},
    {ICON_DEVICE, 5, 22, 5, 2},
    {ICON_DEVICE, 9, 18, 15, 18},
    {ICON_SEND, 3, 11, 21, 3},
    {ICON_SEND, 21, 3, 13, 21},
    {ICON_SEND, 13, 21, 11, 11},
    {ICON_SEND, 11, 11, 3, 11},
    {ICON_SEND, 11, 11, 21, 3},
};
static void draw_icon(unsigned kind,int x,int y,int size,uint16_t color) {
    for(unsigned i=0;i<sizeof(icon_lines)/sizeof(*icon_lines);++i) {
        const icon_line_t *line=icon_lines+i;if(line->icon!=kind)continue;
        vector_line(x+line->x1*size/24,y+line->y1*size/24,
            x+line->x2*size/24,y+line->y2*size/24,1,color);
    }
}
static void outline(int x,int y,int w,int h,uint16_t color) {
    rect(x+3,y,w-6,1,color);rect(x+3,y+h-1,w-6,1,color);
    rect(x,y+3,1,h-6,color);rect(x+w-1,y+3,1,h-6,color);
    pixel(x+1,y+1,color);pixel(x+2,y,color);pixel(x+1,y+h-2,color);pixel(x+2,y+h-1,color);
    pixel(x+w-2,y+1,color);pixel(x+w-3,y,color);pixel(x+w-2,y+h-2,color);pixel(x+w-3,y+h-1,color);
}
static bool same(const char *a,const char *b){return !strcmp(a,b);}
static bool is_face(const node_t *n){return same(n->id,"home_face")||same(n->id,"sidebar_face")||same(n->id,"expression_face");}
static void paint_nodes(void) {
    for(unsigned i=0;i<ui.node_count;++i) {
        const node_t *item=ui.nodes+i;
        node_t label=*item;
        bool footer=same(item->id,"sidebar_plugins")||same(item->id,"sidebar_settings");
        bool row=item->icon&&!footer&&item->text[0];
        bool active=(same(item->id,"sidebar_chat")&&ui.page==2)||(same(item->id,"sidebar_expression")&&ui.page==7);
        bool confirm=same(item->id,"unpair_confirm")||same(item->id,"space_leave_confirm");
        bool filled=active||same(item->id,"edge_space_approve");
        if(filled)rect(item->x,item->y,item->w,item->h,rgb(0x263e31));
        if(confirm||same(item->id,"edge_space_reject")||same(item->id,"unpair_cancel")||same(item->id,"space_leave_cancel"))
            outline(item->x,item->y,item->w,item->h,rgb(confirm?0x73534d:0x344d3d));
        if(confirm||same(item->id,"edge_unpair")||same(item->id,"space_leave"))label.color=rgb(item->enabled?0xd6a99f:0x647c6d);
        if(same(item->id,"link_status")) {
            label.color=rgb(ui.connected?0xa6c5b3:0x84918c);
            if(ui.connected)rect(item->x+1,item->y+11,3,3,rgb(0xb1f2d8));
        }
        if(same(item->id,"settings_label")||same(item->id,"wifi")||same(item->id,"space")||same(item->id,"chat_task")||same(item->id,"plugins_empty")||same(item->id,"expression_caption"))
            label.color=rgb(0x849c8d);
        if(same(item->id,"sidebar_backdrop"))continue;
        if(is_face(item)){face(item);continue;}
        if(item->icon) {
            int size=same(item->id,"plugins_icon")?30:18;
            int x=row?item->x+6:item->x+(item->w-size)/2;
            int y=footer?item->y:item->y+(item->h-size)/2;
            draw_icon(item->icon,x,y,size,item->enabled?rgb(0xb1c7b8):rgb(0x647c6d));
            if(!item->text[0])continue;
            if(footer){label.x=item->x+4;label.y=item->y+20;label.h=24;}
            else {label.x+=30;label.w-=42;label.y+=(item->h-28)/2;label.h=28;}
            if(row&&!same(item->id,"sidebar_chat")&&!same(item->id,"sidebar_expression")) {
                vector_line(item->x+item->w-14,item->y+item->h/2-4,item->x+item->w-10,item->y+item->h/2,1,rgb(0x647c6d));
                vector_line(item->x+item->w-10,item->y+item->h/2,item->x+item->w-14,item->y+item->h/2+4,1,rgb(0x647c6d));
                rect(item->x,item->y+item->h-1,item->w,1,rgb(0x25332e));
            }
        } else if(item->action && !same(item->id,"dismiss_error")) {
            label.y+=(item->h-28)/2;label.h=28;
        }
        text(&label);
    }
}
static void paint_page(void) {
    rect(0,0,WIDTH,HEIGHT,rgb(0x11171b));
    if(page()==2&&!ui.error[0]) {
        rect(117,58,1,116,rgb(0x25332e));
        outline(CHAT_X,187,132,40,rgb(0x344b3d));
    }
    paint_nodes();
}
static int hit(unsigned x,unsigned y) {
    for (unsigned i=0;i<ui.node_count;++i) {
        const node_t *n=ui.nodes+i;
        if (n->action&&n->enabled&&x>=(unsigned)n->x&&y>=(unsigned)n->y&&x<(unsigned)(n->x+n->w)&&y<(unsigned)(n->y+n->h)) return (int)i;
    }
    return -1;
}
static void activate(const char *action) {
    if (!strcmp(action,"dismiss")) ui.error[0]=0;
    else if (!strcmp(action,"sidebar")) ui.sidebar=!ui.sidebar;
    else if (!strcmp(action,"close_sidebar")) ui.sidebar=false;
    else if (!strcmp(action,"settings")) {ui.sidebar=false;ui.page=1;}
    else if (!strcmp(action,"connection")) {ui.sidebar=false;ui.page=9;}
    else if (!strcmp(action,"space_settings")) {ui.sidebar=false;ui.page=10;}
    else if (!strcmp(action,"operations")) {ui.sidebar=false;ui.page=11;}
    else if (!strcmp(action,"plugins")) {ui.sidebar=false;ui.page=6;}
    else if (!strcmp(action,"expression")) {ui.sidebar=false;ui.page=7;}
    else if (!strcmp(action,"pairing")) ui.page=0;
    else if (!strcmp(action,"chat")) {ui.sidebar=false;ui.page=2;}
    else if (!strcmp(action,"space_leave")) ui.page=5;
    else if (!strcmp(action,"confirm_space_leave")) { if(ui.action) ui.action("edge_space_leave",ui.context); ui.page=1; }
    else if (!strcmp(action,"edge_unpair")) ui.page=4;
    else if (!strcmp(action,"confirm_unpair")) { if(ui.action) ui.action("edge_unpair",ui.context); ui.page=1; }
    else if (!strcmp(action,"edge_new")) {ui.page=2;if(ui.action)ui.action(action,ui.context);}
    else if (!strcmp(action,"edge_send")) { operit_ui_submit_chat(); return; }
    else if (ui.action) ui.action(action,ui.context);
    invalidate();
}
bool operit_ui_init(uint16_t w,uint16_t h,operit_ui_flush_cb_t flush,operit_ui_touch_cb_t touch,operit_ui_action_cb_t action,void *context) {
    (void)touch;
    if(w!=WIDTH||h!=HEIGHT||!flush) return false;
    memset(&ui,0,sizeof(ui)); ui.page=2; ui.flush=flush; ui.action=action; ui.context=context; ui.pressed_node=-1;ui.chat_follow_tail=true;
    copy(ui.space,sizeof(ui.space),"等待加入设备空间"); invalidate(); scene(); return true;
}
void operit_ui_pump(uint32_t elapsed) {
    (void)elapsed;
    if(!ui.dirty||!ui.flush) return;
    scene();
    /* Bound SPI work per main-loop iteration; no whole-frame stack buffer. */
    for (unsigned n=0;n<8&&ui.render_y<HEIGHT;++n) {
        memset(strip,0,sizeof(strip));
        if(page()==8&&!ui.error[0]) {
            // Paint the real current page underneath, then the full-height
            // drawer. Scrim is a fixed RGB565 darken, not an alpha buffer.
            ui.sidebar=false;ui.dimmed=true;scene();paint_page();
            ui.sidebar=true;ui.dimmed=false;scene();
            rect(0,0,180,HEIGHT,rgb(0x16201b));
            rect(179,0,1,HEIGHT,rgb(0x33463a));
            rect(12,185,156,1,rgb(0x2b3d31));
            paint_nodes();
        } else {paint_page();}
        operit_ui_area_t area={0,ui.render_y,WIDTH-1,ui.render_y+ROWS-1};
        ui.flush(&area,strip,sizeof(strip),ui.context); ui.render_y+=ROWS;
    }
    if(ui.render_y>=HEIGHT) {
        ui.dirty=ui.refresh_again;ui.refresh_again=false;
        if(ui.dirty)ui.render_y=0;
    }
}
void operit_ui_set_touch(uint16_t x,uint16_t y,bool down) {
    scene();
    if(down&&!ui.pressed) { ui.pressed_node=hit(x,y); ui.press_x=x;ui.press_y=y; }
    if(!down&&ui.pressed) {
        int found=hit(x,y),previous=ui.pressed_node;
        int dx=(int)x-ui.press_x,dy=(int)y-ui.press_y;
        ui.pressed=false;ui.pressed_node=-1;
        if(page()==2&&ui.press_x>=CHAT_X&&x>=CHAT_X&&dx<48&&dx>-48&&(dy>24||dy<-24)){chat_swipe(dy>0?"down":"up");return;}
        if(previous>=0&&previous==found&&dx<16&&dx>-16&&dy<16&&dy>-16) activate(ui.nodes[found].action);
        return;
    }
    ui.pressed=down;
}
void operit_ui_navigate_home(void) { ui.sidebar=false;ui.page=2;invalidate(); }
void operit_ui_navigate_apps(void) { ui.sidebar=false;ui.page=6;invalidate(); }
void operit_ui_set_connection(bool wifi,bool edge) { if(ui.wifi!=wifi||ui.connected!=edge){ui.wifi=wifi;ui.connected=edge;invalidate();} }
void operit_ui_set_paired(bool paired) { if(ui.paired!=paired){ui.paired=paired;ui.sidebar=false;if(ui.page==0)ui.page=2;invalidate();} }
void operit_ui_set_pairing_code(const char *code) {
    bool valid=code&&strlen(code)==6;
    if(valid) for(unsigned i=0;i<6;++i) if(code[i]<'0'||code[i]>'9') valid=false;
    if(valid)ui.sidebar=false;
    if(copy(ui.code,sizeof(ui.code),valid?code:"")) invalidate();
}
void operit_ui_set_space_join_prompt(const char *prompt,bool busy) {
    bool changed=ui.busy!=busy;ui.busy=busy;
    if(!busy) changed=copy(ui.prompt,sizeof(ui.prompt),prompt)||changed;
    if(ui.prompt[0])ui.sidebar=false;
    if(changed) invalidate();
}
#define SETTER(name,field) void operit_ui_set_##name(const char *value){if(copy(ui.field,sizeof(ui.field),value))invalidate();}
SETTER(space_state,space)
/* Message rows and the empty/error hint share one bounded display buffer. */
void operit_ui_set_chat_screen(const char *value){
    if(value&&*value) {if(copy(ui.chat,sizeof(ui.chat),value)){follow_chat_tail();invalidate();}}
    else if(!ui.message_count&&copy(ui.chat,sizeof(ui.chat),""))invalidate();
}
SETTER(chat_task,task)
SETTER(chat_draft,draft)
void operit_ui_action_error(const char *value){if(copy(ui.error,sizeof(ui.error),value))invalidate();}
void operit_ui_set_chat_preview(const char *value){(void)value;}
void operit_ui_set_chat_identity(const char *id,const char *character){(void)id;(void)character;}
void operit_ui_set_expression(const char *expression){
    uint8_t next=expression&&(!strcmp(expression,"sad")||!strcmp(expression,"error"))?1:
        expression&&(!strcmp(expression,"sleepy")||!strcmp(expression,"booting"))?2:0;
    if(next!=ui.expression){ui.expression=next;invalidate();}
}
void operit_ui_set_theme(unsigned theme,bool circular){(void)theme;(void)circular;}
unsigned operit_ui_theme_index(void){return 0;}
bool operit_ui_round_icons(void){return false;}
bool operit_ui_set_emoji_style(unsigned style){return style==0;}
unsigned operit_ui_emoji_style(void){return 0;}
bool operit_emoji_color_available(void){return false;}
const char *operit_ui_chat_draft(void){return ui.submitted;}
void operit_ui_submit_chat(void){
    if(!ui.connected||!ui.draft[0]||ui.sending) return;
    copy(ui.submitted,sizeof(ui.submitted),ui.draft);ui.sending=true;
    if(ui.action) ui.action("edge_send",ui.context);invalidate();
}
void operit_ui_chat_send_result(bool ok,const char *error){
    ui.sending=false;
    if(ok) {ui.draft[0]=0;ui.submitted[0]=0;} else copy(ui.error,sizeof(ui.error),error&&*error?error:"发送失败");
    invalidate();
}
/* One bounded, volatile window shared by painting and screen inspection. */
void operit_ui_set_message(unsigned index,bool user,const char *text){
    if(index==0)ui.chat[0]=0;
    size_t n=strlen(ui.chat);
    if(n&&n+1<sizeof(ui.chat))ui.chat[n++]='\n';
    ui.chat[n]=0;
    const char *prefix=user?"You: ":"AI: ";
    size_t count=strlen(prefix);
    if(n+count+1<sizeof(ui.chat)){memcpy(ui.chat+n,prefix,count);n+=count;ui.chat[n]=0;}
    copy(ui.chat+n,sizeof(ui.chat)-n,text);invalidate();
}
void operit_ui_set_chat_history(bool older,bool newer){ui.chat_has_older=older;ui.chat_has_newer=newer;}
static void follow_chat_tail(void){
    unsigned lines=chat_lines(),last=lines>CHAT_ROWS?lines-CHAT_ROWS:0;
    if(ui.chat_follow_tail||ui.chat_scroll>last)ui.chat_scroll=last;
}
void operit_ui_finish_messages(unsigned count){
    if(!count){ui.chat[0]=0;ui.chat_scroll=0;ui.chat_follow_tail=true;}
    follow_chat_tail();
    ui.message_count=count;invalidate();
}

void operit_ui_set_conversation(unsigned index,const char *id,const char *title,const char *character,bool selected){(void)index;(void)id;(void)title;(void)character;(void)selected;}
void operit_ui_finish_conversations(unsigned count){(void)count;}
/* The renderer explicitly rejects dynamic layout editing. Never silently
 * turn a deployed layout into a different set of touch targets. */
void operit_ui_layout_clear(uint32_t color){(void)color;operit_ui_action_error("自绘 UI 暂不支持布局编辑");}
int operit_ui_layout_add(int type,int parent,int x,int y,int w,int h,uint32_t color,int radius,int value,const char *text,const char *action){(void)type;(void)parent;(void)x;(void)y;(void)w;(void)h;(void)color;(void)radius;(void)value;(void)text;(void)action;return -1;}
void operit_ui_layout_geometry(int index,int x,int y,int w,int h){(void)index;(void)x;(void)y;(void)w;(void)h;}
void operit_ui_layout_bind(int index,const char *action,const char *hold){(void)index;(void)action;(void)hold;}
void operit_ui_layout_page_meta(const char *id,const char *left,const char *right){(void)id;(void)left;(void)right;}
void operit_ui_layout_style(int index,const char *binding,int font){(void)index;(void)binding;(void)font;}
static void append(const char *format,...) {
    if(!json_ok)return;
    va_list args;va_start(args,format);
    int count=vsnprintf(debug_json+json_at,sizeof(debug_json)-json_at,format,args);va_end(args);
    if(count<0||(size_t)count>=sizeof(debug_json)-json_at){json_ok=false;return;}
    json_at+=(size_t)count;
}
static void string(const char *value) {
    char bounded[193] = "";
    copy(bounded,sizeof(bounded),value);
    append("\"");
    for(unsigned i=0;bounded[i];++i){
        unsigned char c=(unsigned char)bounded[i];
        if(c=='"'||c=='\\')append("\\%c",c);
        else if(c<32)append("\\u%04x",c);
        else append("%c",c);
    }
    append("\"");
}
static const char *inspect(void) {
    scene();json_at=0;json_ok=true;
    append("{\"renderer\":\"mini\",\"page\":");string(operit_ui_current_page());
    append(",\"width\":320,\"height\":240,\"staticBytes\":%u,\"drawBytes\":%u,\"heapBytes\":0,\"pairingCode\":",(unsigned)operit_mini_static_bytes(),(unsigned)sizeof(strip));string(ui.code);
    append(",\"sidebarOpen\":%s,\"expression\":%u,\"chatWrapWidth\":%u,\"chatVisibleLines\":%u",ui.sidebar?"true":"false",ui.expression,CHAT_WRAP,CHAT_ROWS);
    append(",\"chatCachedBytes\":%u,\"chatCachedLines\":%u,\"chatScrollLine\":%u,\"nodes\":[",(unsigned)strlen(ui.chat),chat_lines(),ui.chat_scroll);
    for(unsigned i=0;i<ui.node_count;++i){
        node_t *n=ui.nodes+i;
        append("%s{\"id\":",i?",":"");string(n->id);append(",\"text\":");string(n->text);
        append(",\"action\":");string(n->action);append(",\"role\":\"%s\",\"rect\":{\"x\":%d,\"y\":%d,\"w\":%d,\"h\":%d},\"visible\":true,\"enabled\":%s,\"clickable\":%s}",n->action?"button":"label",n->x,n->y,n->w,n->h,n->enabled?"true":"false",n->action?"true":"false");
    }
    append("]}");
    return json_ok?debug_json:"{\"renderer\":\"mini\",\"error\":\"inspection capacity exceeded\",\"nodes\":[]}";
}
const char *operit_ui_debug_tree(void){return inspect();}
const char *operit_ui_debug_snapshot(void){return inspect();}
bool operit_ui_debug_tap(const char *id){
    if(!id)return false;scene();
    for(unsigned i=0;i<ui.node_count;++i){node_t *n=ui.nodes+i;if(n->action&&n->enabled&&!strcmp(n->id,id)){
        unsigned x=n->x+n->w/2,y=n->y+n->h/2;operit_ui_set_touch(x,y,true);operit_ui_set_touch(x,y,false);return true;}}
    return false;
}
bool operit_ui_debug_swipe(const char *direction){return direction&&chat_swipe(direction);}
