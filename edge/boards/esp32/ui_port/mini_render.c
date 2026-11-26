/* Allocator-free RGB565 strip drawing, fonts and integer SVG primitives. */
#include "mini_internal.h"
#include "mini_font.h"

typedef struct {
    uint8_t kind;
    int16_t a, b, c, d;
    uint8_t width;
    uint32_t color;
} face_primitive_t;
#include "mini_face.h"
uint8_t mini_strip[WIDTH * ROWS * 2];
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
    if(mini_ui.dimmed)c=(c&0xf7de)>>1;
    if (x < 0 || x >= WIDTH || y < mini_ui.render_y || y >= mini_ui.render_y + ROWS || y >= HEIGHT) return;
    unsigned at = (unsigned)((y-mini_ui.render_y)*WIDTH+x)*2;
    mini_strip[at] = (uint8_t)c; mini_strip[at+1] = (uint8_t)(c >> 8);
}
static void rect(int x, int y, int w, int h, uint16_t color) {
    int top = y > mini_ui.render_y ? y : mini_ui.render_y;
    int end = y+h < mini_ui.render_y+ROWS ? y+h : mini_ui.render_y+ROWS;
    int left = x > 0 ? x : 0, right = x+w < WIDTH ? x+w : WIDTH;
    if (left >= right || top >= end) return;
    if (mini_ui.dimmed) color=(color&0xf7de)>>1;
    for (int row = top; row < end; ++row) {
        uint8_t *out=mini_strip+((row-mini_ui.render_y)*WIDTH+left)*2;
        for (int col = left; col < right; ++col) { *out++=(uint8_t)color; *out++=(uint8_t)(color>>8); }
    }
}
static void text(const node_t *node) {
    if(node->y+node->h+8<=mini_ui.render_y || node->y-8>=mini_ui.render_y+ROWS)return;
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
        int advance = digits ? MINI_DIGIT_ADVANCE : g->advance;
        if (x + advance > node->x+node->w-4) { x = node->x+6; y += line; }
        if (y+line > node->y+node->h+3) break;
        int gx = x + g->x, gy = y + g->top-MINI_GLYPH_TOP_BIAS;
        if (gy+g->h > mini_ui.render_y && gy < mini_ui.render_y+ROWS) {
            const uint8_t *bits = (digits ? mini_digits_bitmap : mini_text_bitmap) + g->offset;
            for (unsigned row=0; row<g->h; ++row) {
                int py=gy+(int)row;
                if (py < mini_ui.render_y || py >= mini_ui.render_y+ROWS) continue;
                for (unsigned col=0; col<g->w; ++col) {
                    unsigned bit=row*g->w+col;
                    unsigned value=bits[bit/8] & (128u>>(bit%8));
                    if (value) pixel(gx+(int)col,py,node->color);
                }
            }
        }
        x += advance;
    }
}
/* Display rows contain plain Core-projected text only. */
typedef struct { const char *text; unsigned bytes; } chat_row_t;
/** Advances one wrapped line without interpreting chat content. */
static bool next_chat_row(const char **cursor,chat_row_t *row) {
    const char *s=*cursor;if(!*s)return false;
    const char *start=s;int x=0;
    while(*s) {
        const char *before=s;
        uint32_t cp=decode(&s);
        if(cp=='\n'){*cursor=s;*row=(chat_row_t){start,(unsigned)(before-start)};return true;}
        const mini_glyph_t *g=text_glyph(cp);if(!g)continue;
        if(x+g->advance>CHAT_WRAP){s=before;break;}x+=g->advance;
    }
    *cursor=s;*row=(chat_row_t){start,(unsigned)(s-start)};return true;
}
/** Counts wrapped plain-text rows in the volatile chat window. */
unsigned mini_chat_lines(void) {
    const char *s=mini_ui.chat;chat_row_t row;unsigned lines=0;
    while(next_chat_row(&s,&row))++lines;
    return lines?lines:1;
}
/** Locates a wrapped text row without interpreting tool metadata. */
const char *mini_chat_line(unsigned wanted) {
    const char *s=mini_ui.chat;chat_row_t row;unsigned line=0;
    while(next_chat_row(&s,&row)){if(line++==wanted)return row.text;}
    return s;
}
const char *mini_plugin_info_line(unsigned wanted) {
    const char *s=mini_ui.plugin_info; if(!wanted)return s; unsigned line=0; int width=0;
    while(*s) {
        const char *before=s; uint32_t cp=decode(&s);
        const mini_glyph_t *g=text_glyph(cp); int advance=g?g->advance:8;
        if(cp=='\n'){++line;width=0;}
        else if(width+advance>294){++line;width=advance;if(line==wanted)return before;}
        else width+=advance;
        if(line==wanted)return s;
    }
    return s;
}
/* Small integer-only SVG painter. Geometry lives in flash, not a framebuffer. */
static void vector_line(int x,int y,int endX,int endY,int width,uint16_t color) {
    if ((y>endY?y:endY)+width<=mini_ui.render_y || (y<endY?y:endY)-width>=mini_ui.render_y+ROWS)return;
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
            if(mini_ui.expression==2)ry=ry/4+1;
            if(mini_ui.expression==1)ry=ry*3/4;
            int top=mini_ui.render_y-cy,bottom=mini_ui.render_y+ROWS-1-cy;
            if(top < -ry)top=-ry;if(bottom>ry)bottom=ry;
            for(int y=top;y<=bottom;++y)for(int x=-rx;x<=rx;++x)
                if(x*x*ry*ry+y*y*rx*rx<=rx*rx*ry*ry)pixel(cx+x,cy+y,color);
        } else {
            int y1=p->b,y2=p->d;
            if(mini_ui.expression==1){y1=146-y1;y2=146-y2;}
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
    {ICON_BOLT, 14, 2, 5, 13},
    {ICON_BOLT, 5, 13, 12, 13},
    {ICON_BOLT, 12, 13, 9, 22},
    {ICON_BOLT, 9, 22, 20, 10},
    {ICON_BOLT, 20, 10, 13, 10},
    {ICON_BOLT, 13, 10, 14, 2},
    {ICON_MIC, 10, 3, 14, 3},
    {ICON_MIC, 14, 3, 16, 5},
    {ICON_MIC, 16, 5, 16, 11},
    {ICON_MIC, 16, 11, 14, 13},
    {ICON_MIC, 14, 13, 10, 13},
    {ICON_MIC, 10, 13, 8, 11},
    {ICON_MIC, 8, 11, 8, 5},
    {ICON_MIC, 8, 5, 10, 3},
    {ICON_MIC, 5, 10, 5, 13},
    {ICON_MIC, 5, 13, 8, 17},
    {ICON_MIC, 8, 17, 16, 17},
    {ICON_MIC, 16, 17, 19, 13},
    {ICON_MIC, 19, 13, 19, 10},
    {ICON_MIC, 12, 17, 12, 21},
    {ICON_MIC, 8, 21, 16, 21},
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
static bool is_face(const node_t *n){return same(n->id,"home_face")||same(n->id,"sidebar_face")||same(n->id,"expression_face");}
/** Paints bounded display text without tool-specific state or cards. */
static void paint_chat(const node_t *item) {
    if(!mini_ui.chat[0]){text(item);return;}
    const char *s=mini_ui.chat;chat_row_t row;unsigned line=0;
    while(next_chat_row(&s,&row)) {
        unsigned at=line++;if(at<mini_ui.chat_scroll)continue;
        unsigned visible=at-mini_ui.chat_scroll;if(visible>=CHAT_ROWS)break;
        int y=item->y+(int)visible*21;
        node_t label=*item;label.y=y-1;label.h=22;
        char name[256];unsigned n=row.bytes<sizeof(name)-1?row.bytes:sizeof(name)-1;
        while(n&&((unsigned char)row.text[n]&0xc0)==0x80)--n;
        memcpy(name,row.text,n);name[n]=0;label.text=name;
        text(&label);
    }
}
static void paint_nodes(void) {
    for(unsigned i=0;i<mini_ui.node_count;++i) {
        const node_t *item=mini_ui.nodes+i;
        node_t label=*item;
        if(same(item->id,"chat_text")){paint_chat(item);continue;}
        if(same(item->id,"plugin_dialog_title")){draw_icon(ICON_PLUGIN,item->x+6,item->y+7,16,rgb(0xb1c7b8));label.x+=24;label.w-=24;}
        if(same(item->id,"plugin_info")){label.text=mini_plugin_info_line(mini_ui.plugin_info_scroll);if(mini_ui.plugin_test_error[0])label.h=84;text(&label);continue;}
        if(same(item->id,"plugins_exclusive")||same(item->id,"plugins_general")) {
            bool selected=same(item->id,"plugins_exclusive")==mini_ui.plugins_exclusive;
            if(selected){rect(item->x,item->y,item->w,item->h,rgb(0x263e31));rect(item->x,item->y+item->h-2,item->w,2,rgb(0xb1f2d8));}
        }
        if(!strncmp(item->id,"plugin_probe_",13)) {
            unsigned index=(unsigned)(item->id[strlen(item->id)-1]-'0');
            const plugin_t *p=mini_ui.plugins+index;
            rect(item->x,item->y,item->w,item->h,rgb(0x17271f));
            outline(item->x,item->y,item->w,item->h,rgb(0x344d3d));
            label.x+=1;label.y-=1;label.w-=8;label.h=22;
            text(&label);
            draw_icon(ICON_PLUGIN,item->x+6,item->y+23,14,rgb(0xb1c7b8));
            label.text=p->latency;label.x=item->x+25;label.y=item->y+18;label.w=item->w-32;label.h=22;
            label.color=rgb(p->status==2?0x71e5a2:p->status==3?0xf07878:0x849c8d);
            /* Right-align the numeric result in its own plugin cell. */
            int width=0;for(const char *t=label.text;*t;){uint32_t cp=decode(&t);const mini_glyph_t *g=text_glyph(cp);width+=g?g->advance:8;}
            width+=10;if(width<label.w){label.x+=label.w-width;label.w=width;}
            text(&label);continue;
        }
        if(same(item->id,"plugins_test_all")) {
            draw_icon(ICON_BOLT,item->x+12,item->y+9,20,rgb(0xffffff));continue;
        }
        if(same(item->id,"plugin_test_connection")||same(item->id,"plugin_test_tool")) {
            outline(item->x,item->y,item->w,item->h,rgb(item->enabled?0x50745c:0x344d3d));
        }
        if(same(item->id,"plugin_connection_result")||same(item->id,"plugin_tool_result")) {
            unsigned state=same(item->id,"plugin_connection_result")?mini_ui.plugins[mini_ui.plugin_selected].status:mini_ui.plugin_tool_status;
            label.color=rgb(state==2?0x71e5a2:state==3?0xf07878:state==1?0xe4c778:0x849c8d);
        }
        if(same(item->id,"plugin_test_error"))label.color=rgb(0xf07878);
        bool footer=same(item->id,"sidebar_plugins")||same(item->id,"sidebar_settings");
        bool row=item->icon&&!footer&&item->text[0];
        bool active=(same(item->id,"sidebar_chat")&&mini_ui.page==2)||(same(item->id,"sidebar_expression")&&mini_ui.page==7);
        bool confirm=same(item->id,"unpair_confirm")||same(item->id,"space_leave_confirm");
        bool filled=active||same(item->id,"edge_space_approve");
        if(!strncmp(item->id,"conversation_",13))filled=mini_ui.conversations[item->id[13]-'0'].selected;
        if(filled)rect(item->x,item->y,item->w,item->h,rgb(0x263e31));
        if(confirm||same(item->id,"edge_space_reject")||same(item->id,"unpair_cancel")||same(item->id,"space_leave_cancel"))
            outline(item->x,item->y,item->w,item->h,rgb(confirm?0x73534d:0x344d3d));
        if(confirm||same(item->id,"edge_unpair")||same(item->id,"space_leave"))label.color=rgb(item->enabled?0xd6a99f:0x647c6d);
        if(same(item->id,"link_status")) {
            label.color=rgb(mini_ui.connected?0xa6c5b3:0x84918c);
            if(mini_ui.connected)rect(item->x+1,item->y+11,3,3,rgb(0xb1f2d8));
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
    if(mini_page()==12 && mini_ui.scene_painter) mini_ui.scene_painter(mini_strip,sizeof(mini_strip),mini_ui.render_y,ROWS,mini_ui.frame_tick,mini_ui.context);
    if(mini_page()==2&&!mini_ui.error[0]) {
        rect(117,58,1,116,rgb(0x25332e));
        outline(170,187,88,40,rgb(0x344b3d));
    }
    paint_nodes();
    if(mini_page()==15&&mini_ui.device_space_phase==3) {
        node_t hint={"pair_code_hint",mini_ui.device_space_status,NULL,16,192,288,44,rgb(0xecf3f0),1,ICON_NONE,true};
        text(&hint);
    }
}
void operit_ui_pump(uint32_t elapsed) {
    (void)elapsed;
    if(mini_page()==12 && !mini_ui.dirty) { mini_ui.render_y=0;mini_ui.dirty=true; }
    if(mini_ui.render_y==0) mini_ui.frame_tick=mini_ui.scene_tick;
    if(!mini_ui.dirty||!mini_ui.flush) return;
    mini_scene();
    /* Bound SPI work per main-loop iteration; no whole-frame stack buffer. */
    for (unsigned n=0;n<8&&mini_ui.render_y<HEIGHT;++n) {
        memset(mini_strip,0,sizeof(mini_strip));
        if(mini_page()==6&&mini_ui.plugin_dialog&&!mini_ui.error[0]) {
            rect(0,0,WIDTH,HEIGHT,rgb(0x11171b));paint_nodes();
        } else if(mini_page()==8&&!mini_ui.error[0]) {
            // Paint the real current page underneath, then the full-height
            // drawer. Scrim is a fixed RGB565 darken, not an alpha buffer.
            mini_ui.sidebar=false;mini_ui.dimmed=true;mini_scene();paint_page();
            mini_ui.sidebar=true;mini_ui.dimmed=false;mini_scene();
            rect(0,0,180,HEIGHT,rgb(0x16201b));
            rect(179,0,1,HEIGHT,rgb(0x33463a));
            rect(12,185,156,1,rgb(0x2b3d31));
            paint_nodes();
        } else {paint_page();}
        operit_ui_area_t area={0,mini_ui.render_y,WIDTH-1,mini_ui.render_y+ROWS-1};
        mini_ui.flush(&area,mini_strip,sizeof(mini_strip),mini_ui.context); mini_ui.render_y+=ROWS;
    }
    if(mini_ui.render_y>=HEIGHT) {
        mini_ui.dirty=mini_ui.refresh_again;mini_ui.refresh_again=false;
        if(mini_ui.dirty)mini_ui.render_y=0;
    }
}
