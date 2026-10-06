/* Allocator-free self-drawn UI. Fixed 320x240 RGB565, two-row strips.
 * Pairing and space decisions remain host actions, never local UI decisions.
 * No layout editor, image decoder, animation or on-device keyboard in this renderer.
 */
#include "operit_mini_ui.h"
#include "mini_font.h"
#include <stdio.h>
#include <stdarg.h>
#include <string.h>

enum { WIDTH = 320, HEIGHT = 240, ROWS = 2, NODE_LIMIT = 10 };
typedef struct {
    const char *id, *text, *action;
    int16_t x, y, w, h;
    uint16_t color;
    uint8_t scale;
    bool enabled;
} node_t;
typedef struct {
    operit_ui_flush_cb_t flush;
    operit_ui_action_cb_t action;
    void *context;
    bool wifi, connected, paired, busy, pressed, dirty, refresh_again, sending;
    int page, pressed_node, render_y;
    uint16_t press_x, press_y;
    unsigned node_count, message_count, chat_scroll;
    bool chat_follow_tail, chat_has_older, chat_has_newer;
    char code[7], prompt[256], space[96], chat[2048], task[128];
    char error[192], draft[512], submitted[512];
    node_t nodes[NODE_LIMIT];
} mini_state_t;
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
static unsigned chat_lines(void) {
    const char *s=ui.chat;unsigned lines=1;int x=0;
    while(*s) {uint32_t cp=decode(&s);if(cp=='\n'){++lines;x=0;continue;}
        const mini_glyph_t *g=text_glyph(cp);if(!g)continue;
        if(x+g->advance>290){++lines;x=0;}x+=g->advance;}
    return lines;
}
static const char *chat_line(unsigned wanted) {
    const char *s=ui.chat;unsigned line=0;int x=0;if(!wanted)return s;
    while(*s) {const char *before=s;uint32_t cp=decode(&s);
        if(cp=='\n'){++line;x=0;if(line==wanted)return s;continue;}
        const mini_glyph_t *g=text_glyph(cp);if(!g)continue;
        if(x+g->advance>290){++line;x=0;if(line==wanted)return before;}x+=g->advance;}
    return s;
}
static int page(void);
static bool chat_swipe(const char *direction) {
    if(page()!=2)return false;
    unsigned lines=chat_lines(),last=lines>5?lines-5:0;
    if(!strcmp(direction,"down")) {
        if(ui.chat_scroll){ui.chat_scroll=ui.chat_scroll>5?ui.chat_scroll-5:0;ui.chat_follow_tail=false;}
        else if(ui.chat_has_older&&ui.action){ui.chat_follow_tail=true;ui.action("edge_history_older",ui.context);}
    } else if(!strcmp(direction,"up")) {
        if(ui.chat_scroll<last){ui.chat_scroll+=5;if(ui.chat_scroll>last)ui.chat_scroll=last;ui.chat_follow_tail=ui.chat_scroll==last;}
        else if(ui.chat_has_newer&&ui.action){ui.chat_scroll=0;ui.chat_follow_tail=false;ui.action("edge_history_newer",ui.context);}
    } else return false;
    invalidate();return true;
}
static int page(void) { return ui.code[0] ? 0 : ui.prompt[0] ? 3 : ui.page; }
const char *operit_ui_current_page(void) {
    switch(page()) { case 0:return "Pairing"; case 1:return "Status"; case 2:return "Chat"; case 4:return "Unpair"; case 5:return "LeaveSpace"; default:return "Space"; }
}
static void node(const char *id,const char *caption,const char *action,int x,int y,int w,int h,bool enabled,int scale) {
    if (ui.node_count == NODE_LIMIT) return;
    ui.nodes[ui.node_count++] = (node_t){id,caption,action,x,y,w,h,rgb(enabled ? 0xe6edf3 : 0x73818d),scale,enabled};
}
static void scene(void) {
    ui.node_count=0;
    node("title",operit_ui_current_page(),NULL,8,4,180,28,true,1);
    node("link_status",ui.connected ? "已连接" : ui.paired ? "已配对 / 离线" : "未配对",NULL,185,4,130,28,true,1);
    if (ui.error[0] && !ui.code[0] && !ui.prompt[0]) {
        node("error",ui.error,NULL,12,42,296,130,true,1);
        node("dismiss_error","返回","dismiss",16,183,288,36,true,1);
    } else if (page()==0) {
        node("pairing_hint","USB 串口配对\n请在 Operit 中输入验证码",NULL,16,46,288,52,true,1);
        node("pairing_code",ui.code[0] ? ui.code : "等待配对请求",NULL,94,103,220,48,true,ui.code[0] ? 2 : 1);
        node("pair_listen","开始监听","edge_pair",16,184,138,36,true,1);
        node("open_status","状态","status",166,184,138,36,!ui.code[0],1);
    } else if (page()==3) {
        node("space_join_body",ui.busy ? "正在处理，请稍候…" : ui.prompt,NULL,16,46,288,ui.error[0]&&!ui.busy?82:123,true,1);
        if(ui.error[0]&&!ui.busy) node("space_join_error",ui.error,NULL,16,130,288,46,true,1);
        node("edge_space_reject","拒绝","edge_space_reject",16,184,138,36,!ui.busy,1);
        node("edge_space_approve","确认加入","edge_space_approve",166,184,138,36,!ui.busy,1);
    } else if (page()==4) {
        node("unpair_warning","解除配对后需重新连接。\n确认解除当前设备配对？",NULL,16,50,288,100,true,1);
        node("unpair_cancel","返回","status",16,184,138,36,true,1);
        node("unpair_confirm","确认解除","confirm_unpair",166,184,138,36,true,1);
    } else if (page()==5) {
        node("space_leave_warning","退出当前设备空间？\n配对和 Wi-Fi 将保留。\n新空间加入仍需审批。",NULL,16,50,288,100,true,1);
        node("space_leave_cancel","返回","status",16,184,138,36,true,1);
        node("space_leave_confirm","确认退出","confirm_space_leave",166,184,138,36,true,1);
    } else if (page()==1) {
        node("wifi",ui.wifi ? "Wi-Fi 已连接" : "Wi-Fi 未连接",NULL,16,44,288,28,true,1);
        node("space",ui.space,NULL,16,76,288,52,true,1);
        node("chat_open","对话","chat",16,142,138,34,ui.paired,1);
        node("pairing_open","配对","pairing",166,142,138,34,true,1);
        node("space_leave","退出空间","space_leave",16,184,138,36,!ui.busy,1);
        node("edge_unpair","解除配对","edge_unpair",166,184,138,36,ui.paired,1);
    } else {
        node("chat_text",ui.chat[0] ? chat_line(ui.chat_scroll) : "连接 Operit 后显示消息",NULL,10,42,300,112,true,1);
        node("chat_task",ui.task,NULL,10,157,300,25,true,1);
        node("open_status","状态","status",12,190,92,32,true,1);
        node("edge_new","新对话","edge_new",114,190,92,32,ui.connected&&!ui.sending,1);
        node("edge_send",ui.sending ? "发送中" : "发送","edge_send",216,190,92,32,ui.connected&&ui.draft[0]&&!ui.sending,1);
    }
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
    else if (!strcmp(action,"status")) ui.page=1;
    else if (!strcmp(action,"pairing")) ui.page=0;
    else if (!strcmp(action,"chat")) ui.page=2;
    else if (!strcmp(action,"space_leave")) ui.page=5;
    else if (!strcmp(action,"confirm_space_leave")) { if(ui.action) ui.action("edge_space_leave",ui.context); ui.page=1; }
    else if (!strcmp(action,"edge_unpair")) ui.page=4;
    else if (!strcmp(action,"confirm_unpair")) { if(ui.action) ui.action("edge_unpair",ui.context); ui.page=1; }
    else if (!strcmp(action,"edge_send")) { operit_ui_submit_chat(); return; }
    else if (ui.action) ui.action(action,ui.context);
    invalidate();
}
bool operit_ui_init(uint16_t w,uint16_t h,operit_ui_flush_cb_t flush,operit_ui_touch_cb_t touch,operit_ui_action_cb_t action,void *context) {
    (void)touch;
    if(w!=WIDTH||h!=HEIGHT||!flush) return false;
    memset(&ui,0,sizeof(ui)); ui.flush=flush; ui.action=action; ui.context=context; ui.pressed_node=-1;ui.chat_follow_tail=true;
    copy(ui.space,sizeof(ui.space),"等待加入设备空间"); invalidate(); scene(); return true;
}
void operit_ui_pump(uint32_t elapsed) {
    (void)elapsed;
    if(!ui.dirty||!ui.flush) return;
    scene();
    /* Bound SPI work per main-loop iteration; no whole-frame stack buffer. */
    for (unsigned n=0;n<8&&ui.render_y<HEIGHT;++n) {
        memset(strip,0,sizeof(strip)); rect(0,0,WIDTH,HEIGHT,rgb(0x101820));
        rect(0,34,WIDTH,1,rgb(0x344555));
        for (unsigned i=0;i<ui.node_count;++i) {
            node_t *item=ui.nodes+i;
            if(item->action) rect(item->x,item->y,item->w,item->h,rgb(item->enabled?0x263849:0x18212a));
            text(item);
        }
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
        if(page()==2&&dx<48&&dx>-48&&(dy>24||dy<-24)){chat_swipe(dy>0?"down":"up");return;}
        if(previous>=0&&previous==found&&dx<16&&dx>-16&&dy<16&&dy>-16) activate(ui.nodes[found].action);
        return;
    }
    ui.pressed=down;
}
void operit_ui_navigate_home(void) { ui.page=ui.paired?2:0;invalidate(); }
void operit_ui_navigate_apps(void) { ui.page=1;invalidate(); }
void operit_ui_set_connection(bool wifi,bool edge) { if(ui.wifi!=wifi||ui.connected!=edge){ui.wifi=wifi;ui.connected=edge;invalidate();} }
void operit_ui_set_paired(bool paired) { if(ui.paired!=paired){ui.paired=paired;ui.page=paired?2:0;invalidate();} }
void operit_ui_set_pairing_code(const char *code) {
    bool valid=code&&strlen(code)==6;
    if(valid) for(unsigned i=0;i<6;++i) if(code[i]<'0'||code[i]>'9') valid=false;
    if(copy(ui.code,sizeof(ui.code),valid?code:"")) invalidate();
}
void operit_ui_set_space_join_prompt(const char *prompt,bool busy) {
    bool changed=ui.busy!=busy;ui.busy=busy;
    if(!busy) changed=copy(ui.prompt,sizeof(ui.prompt),prompt)||changed;
    if(changed) invalidate();
}
#define SETTER(name,field) void operit_ui_set_##name(const char *value){if(copy(ui.field,sizeof(ui.field),value))invalidate();}
SETTER(space_state,space)
/* Message rows and the empty/error hint share one bounded display buffer. */
void operit_ui_set_chat_screen(const char *value){
    if(value&&*value) {if(copy(ui.chat,sizeof(ui.chat),value))invalidate();}
    else if(!ui.message_count&&copy(ui.chat,sizeof(ui.chat),""))invalidate();
}
SETTER(chat_task,task)
SETTER(chat_draft,draft)
void operit_ui_action_error(const char *value){if(copy(ui.error,sizeof(ui.error),value))invalidate();}
void operit_ui_set_chat_preview(const char *value){(void)value;}
void operit_ui_set_chat_identity(const char *id,const char *character){(void)id;(void)character;}
void operit_ui_set_expression(const char *expression){(void)expression;}
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
void operit_ui_finish_messages(unsigned count){
    if(!count){ui.chat[0]=0;ui.chat_scroll=0;ui.chat_follow_tail=true;}
    unsigned lines=chat_lines(),last=lines>5?lines-5:0;
    if(ui.chat_follow_tail||ui.chat_scroll>last)ui.chat_scroll=last;
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
