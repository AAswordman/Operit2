/* Bounded UI state, host actions, touch handling and inspection. */
#include "mini_internal.h"
#include <stdio.h>
#include <stdarg.h>

mini_state_t mini_ui;

/* A bounded inspection buffer is included in the real device RAM budget. */
static char debug_json[3328];
static size_t json_at;
static bool json_ok;
_Static_assert(sizeof(mini_ui) + sizeof(mini_strip) + sizeof(debug_json) + sizeof(json_at) + sizeof(json_ok) <= 10 * 1024,
               "Mini UI static RAM must remain below 10 KiB");
size_t operit_mini_static_bytes(void) { return sizeof(mini_ui) + sizeof(mini_strip) + sizeof(debug_json) + sizeof(json_at) + sizeof(json_ok); }
size_t operit_mini_draw_bytes(void) { return sizeof(mini_strip); }
void mini_invalidate(void) {
    if (!mini_ui.dirty) mini_ui.render_y = 0;
    else if (mini_ui.render_y > 0) mini_ui.refresh_again = true;
    mini_ui.dirty = true; mini_ui.pressed_node = -1; mini_ui.scene_press = false;
}
/* Copy whole UTF-8 characters; never leave a truncated multibyte glyph. */
bool mini_copy(char *dst, size_t capacity, const char *src) {
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
static void follow_chat_tail(void);
static bool chat_swipe(const char *direction) {
    if(mini_page()==6&&mini_ui.plugin_dialog) {
        if(!strcmp(direction,"down"))mini_ui.plugin_info_scroll=mini_ui.plugin_info_scroll>3?mini_ui.plugin_info_scroll-3:0;
        else if(!strcmp(direction,"up")) {if(*mini_plugin_info_line(mini_ui.plugin_info_scroll+5))mini_ui.plugin_info_scroll+=3;}
        else return false;
        mini_invalidate();return true;
    }
    if(mini_page()!=2)return false;
    unsigned lines=mini_chat_lines(),last=lines>CHAT_ROWS?lines-CHAT_ROWS:0;
    if(!strcmp(direction,"down")) {
        if(mini_ui.chat_scroll){mini_ui.chat_scroll=mini_ui.chat_scroll>CHAT_ROWS?mini_ui.chat_scroll-CHAT_ROWS:0;mini_ui.chat_follow_tail=false;}
        else if(mini_ui.chat_has_older&&mini_ui.action){mini_ui.chat_follow_tail=true;mini_ui.action("edge_history_older",mini_ui.context);}
    } else if(!strcmp(direction,"up")) {
        if(mini_ui.chat_scroll<last){mini_ui.chat_scroll+=CHAT_ROWS;if(mini_ui.chat_scroll>last)mini_ui.chat_scroll=last;mini_ui.chat_follow_tail=mini_ui.chat_scroll==last;}
        else if(mini_ui.chat_has_newer&&mini_ui.action){mini_ui.chat_scroll=0;mini_ui.chat_follow_tail=false;mini_ui.action("edge_history_newer",mini_ui.context);}
    } else return false;
    mini_invalidate();return true;
}
static int hit(unsigned x,unsigned y) {
    for (unsigned i=0;i<mini_ui.node_count;++i) {
        const node_t *n=mini_ui.nodes+i;
        if (n->action&&n->enabled&&x>=(unsigned)n->x&&y>=(unsigned)n->y&&x<(unsigned)(n->x+n->w)&&y<(unsigned)(n->y+n->h)) return (int)i;
    }
    return -1;
}
static void activate(const char *action) {
    if(!strcmp(action,"plugin_dialog_close")){mini_ui.plugin_dialog=false;if(mini_ui.action)mini_ui.action("edge_plugin_close",mini_ui.context);mini_invalidate();return;}
    if(!strcmp(action,"plugin_info_up")||!strcmp(action,"plugin_info_down")){chat_swipe(!strcmp(action,"plugin_info_up")?"down":"up");return;}
    if(!strcmp(action,"plugin_test_connection")||!strcmp(action,"plugin_test_tool")) {
        char request[131];snprintf(request,sizeof(request),"%s%.108s",!strcmp(action,"plugin_test_tool")?"edge_plugin_tool_test:":"edge_plugin_probe:",mini_ui.plugins[mini_ui.plugin_selected].action+17);
        if(mini_ui.action)mini_ui.action(request,mini_ui.context);return;
    }
    if(!strncmp(action,"edge_plugin_open:",17)) {
        for(unsigned i=0;i<mini_ui.plugin_count;++i)if(!strcmp(action,mini_ui.plugins[i].action)) {
            mini_ui.plugin_selected=(uint8_t)i;mini_ui.plugin_dialog=true;mini_ui.plugin_tool_status=0;
            mini_copy(mini_ui.plugin_tool_text,sizeof(mini_ui.plugin_tool_text),"未测试");mini_ui.plugin_test_error[0]=0;
            mini_copy(mini_ui.plugin_connect_text,sizeof(mini_ui.plugin_connect_text),mini_ui.plugins[i].status==1?"测试中":mini_ui.plugins[i].status==2?"成功":mini_ui.plugins[i].status==3?"失败":"未测试");
            mini_ui.plugin_info_scroll=0;mini_ui.plugin_tool_total=0;mini_ui.plugin_details_loading=true;
            snprintf(mini_ui.plugin_info,sizeof(mini_ui.plugin_info),"包 ID: %.108s\n读取 Core 详情…",action+17);
            if(mini_ui.plugins[i].status==2||mini_ui.plugins[i].status==3)
                snprintf(mini_ui.plugin_connect_text,sizeof(mini_ui.plugin_connect_text),"%s %s",mini_ui.plugins[i].status==2?"成功":"失败",mini_ui.plugins[i].latency);
            if(mini_ui.action)mini_ui.action(action,mini_ui.context);mini_invalidate();return;
        }
        return;
    }
    if (!strcmp(action,"dismiss")) mini_ui.error[0]=0;
    else if (!strcmp(action,"sidebar")) mini_ui.sidebar=!mini_ui.sidebar;
    else if (!strcmp(action,"close_sidebar")) mini_ui.sidebar=false;
    else if (!strcmp(action,"settings")) {mini_ui.sidebar=false;mini_ui.page=1;}
    else if (!strcmp(action,"connection")) {mini_ui.sidebar=false;mini_ui.page=9;}
    else if (!strcmp(action,"pairing")) {mini_ui.sidebar=false;mini_ui.page=0;if(mini_ui.action)mini_ui.action("edge_pair",mini_ui.context);}
    else if (!strcmp(action,"space_settings")) {mini_ui.sidebar=false;mini_ui.page=10;}
    else if (!strcmp(action,"space_manage")) {mini_ui.page=16;}
    else if (!strcmp(action,"operations")) {mini_ui.sidebar=false;mini_ui.page=11;}
    else if (!strcmp(action,"edge_space_review")) {if(mini_ui.prompt[0])mini_ui.page=3;}
    else if (!strcmp(action,"edge_pair_resume")) {if(mini_ui.device_space_phase==3)mini_ui.page=15;}
    else if (!strcmp(action,"edge_pair_discover")) {mini_ui.page=14;mini_ui.pair_input[0]=0;if(mini_ui.action)mini_ui.action(action,mini_ui.context);}
    else if (!strncmp(action,"edge_pair_candidate:",20)) {mini_ui.page=15;mini_ui.pair_input[0]=0;if(mini_ui.action)mini_ui.action(action,mini_ui.context);}
    else if (!strncmp(action,"edge_pair_digit:",16)) {if(strlen(mini_ui.pair_input)<6){size_t n=strlen(mini_ui.pair_input);mini_ui.pair_input[n]=action[16];mini_ui.pair_input[n+1]=0;}}
    else if (!strcmp(action,"edge_pair_backspace")) {size_t n=strlen(mini_ui.pair_input);if(n)mini_ui.pair_input[n-1]=0;}
    else if (!strcmp(action,"edge_pair_submit")) {if(strlen(mini_ui.pair_input)==6&&mini_ui.action){char submit[48];snprintf(submit,sizeof(submit),"edge_pair_submit:%.6s",mini_ui.pair_input);mini_ui.device_space_phase=4;mini_ui.action(submit,mini_ui.context);}}
    else if (!strcmp(action,"edge_pair_cancel")) {if(mini_ui.device_space_phase==4)return;mini_ui.pair_input[0]=0;mini_ui.page=10;if(mini_ui.action)mini_ui.action(action,mini_ui.context);}
    else if (!strcmp(action,"plugins")) {mini_ui.sidebar=false;mini_ui.page=6;if(mini_ui.action&&mini_ui.connected)mini_ui.action("edge_plugins_refresh",mini_ui.context);}
    else if (!strcmp(action,"expression")) {mini_ui.sidebar=false;mini_ui.page=7;}
    else if (!strcmp(action,"chat")) {mini_ui.sidebar=false;mini_ui.page=2;if(!mini_ui.sending)mini_ui.submitted[0]=0;}
    else if (!strcmp(action,"conversations")||!strcmp(action,"conversations_refresh")||!strcmp(action,"conversations_prev")||!strcmp(action,"conversations_next")) {
        if(!mini_ui.connected||mini_ui.sending||mini_ui.speech_phase)return;
        if(!strcmp(action,"conversations")){mini_ui.sidebar=false;mini_ui.page=13;mini_ui.conversation_offset=0;}
        else if(!strcmp(action,"conversations_prev"))mini_ui.conversation_offset=mini_ui.conversation_offset>=3?mini_ui.conversation_offset-3:0;
        else if(!strcmp(action,"conversations_next")&&mini_ui.conversation_offset+3<mini_ui.conversation_total)mini_ui.conversation_offset+=3;
        memset(mini_ui.conversations,0,sizeof(mini_ui.conversations));mini_ui.conversation_count=0;mini_ui.conversations_loading=true;mini_ui.conversations_error_shown=false;mini_ui.error[0]=0;
        if(mini_ui.action)mini_ui.action("edge_conversations_refresh",mini_ui.context);
    }
    else if (!strncmp(action,"edge_select:",12)) {
        if(!mini_ui.connected||mini_ui.sending||mini_ui.speech_phase)return;
        if(!strcmp(action+12,mini_ui.chat_id)){mini_ui.page=2;mini_ui.submitted[0]=0;}
        else if(mini_ui.action)mini_ui.action(action,mini_ui.context);
    }
    else if (!strcmp(action,"space_leave")) mini_ui.page=5;
    else if (!strcmp(action,"confirm_space_leave")) { if(mini_ui.action) mini_ui.action("edge_space_leave",mini_ui.context); mini_ui.page=10; }
    else if (!strcmp(action,"edge_unpair")) mini_ui.page=4;
    else if (!strcmp(action,"confirm_unpair")) { if(mini_ui.action) mini_ui.action("edge_unpair",mini_ui.context); mini_ui.page=10; }
    else if (!strcmp(action,"unpair_cancel")) mini_ui.page=16;
    else if (!strcmp(action,"space_leave_cancel")) mini_ui.page=16;
    else if (!strcmp(action,"edge_new")) {mini_ui.page=2;if(mini_ui.action)mini_ui.action(action,mini_ui.context);}
    else if (!strcmp(action,"edge_send")) { operit_ui_submit_chat(); return; }
    else if (mini_ui.action) mini_ui.action(action,mini_ui.context);
    mini_invalidate();
}
bool operit_ui_init(uint16_t w,uint16_t h,operit_ui_flush_cb_t flush,operit_ui_touch_cb_t touch,operit_ui_action_cb_t action,void *context) {
    (void)touch;
    if(w!=WIDTH||h!=HEIGHT||!flush) return false;
    memset(&mini_ui,0,sizeof(mini_ui)); mini_ui.page=2; mini_ui.flush=flush; mini_ui.action=action; mini_ui.context=context; mini_ui.pressed_node=-1;mini_ui.chat_follow_tail=true;
    mini_copy(mini_ui.space,sizeof(mini_ui.space),"等待加入设备空间"); mini_invalidate(); mini_scene(); return true;
}
void operit_ui_scene_painter(operit_ui_scene_cb_t painter) { mini_ui.scene_painter=painter; }
void operit_ui_set_scene(bool active,uint32_t revision,uint32_t tick) {
    if(mini_ui.scene_active!=active||mini_ui.scene_revision!=revision) {
        mini_ui.scene_active=active;mini_ui.scene_revision=revision;mini_invalidate();
    }
    mini_ui.scene_tick=tick;
}
void operit_ui_set_touch(uint16_t x,uint16_t y,bool down) {
    mini_scene();
    if(down&&!mini_ui.pressed) { mini_ui.pressed_node=hit(x,y); mini_ui.press_x=x;mini_ui.press_y=y; mini_ui.scene_press=mini_page()==12&&y>=24; }
    if(!down&&mini_ui.pressed) {
        int found=hit(x,y),previous=mini_ui.pressed_node;
        int dx=(int)x-mini_ui.press_x,dy=(int)y-mini_ui.press_y;
        mini_ui.pressed=false;mini_ui.pressed_node=-1;
        if(mini_page()==12 && mini_ui.scene_press && y>=24 && mini_ui.press_y>=24 && dx<16&&dx>-16&&dy<16&&dy>-16) {
            char action[48];snprintf(action,sizeof(action),"edge_scene_touch:%u:%u",x,y);
            if(mini_ui.action) mini_ui.action(action,mini_ui.context);return;
        }
        if(mini_page()==6&&mini_ui.plugin_dialog&&mini_ui.press_y>=36&&mini_ui.press_y<144&&dx<48&&dx>-48&&(dy>24||dy<-24)){chat_swipe(dy>0?"down":"up");return;}
        if(mini_page()==2&&mini_ui.press_x>=CHAT_X&&x>=CHAT_X&&dx<48&&dx>-48&&(dy>24||dy<-24)){chat_swipe(dy>0?"down":"up");return;}
        if(previous>=0&&previous==found&&dx<16&&dx>-16&&dy<16&&dy>-16) activate(mini_ui.nodes[found].action);
        return;
    }
    mini_ui.pressed=down;
}
void operit_ui_navigate_home(void) { mini_ui.sidebar=false;mini_ui.page=mini_ui.remote_display?1:2;if(!mini_ui.sending)mini_ui.submitted[0]=0;mini_invalidate(); }
void operit_ui_navigate_apps(void) { mini_ui.sidebar=false;mini_ui.page=mini_ui.remote_display?1:6;mini_invalidate(); }
void operit_ui_set_remote_display(bool enabled) {
    if(mini_ui.remote_display!=enabled) {
        mini_ui.remote_display=enabled;mini_ui.sidebar=false;mini_ui.page=enabled?1:2;mini_invalidate();
    }
}
void operit_ui_set_connection(bool wifi,bool edge) { if(mini_ui.wifi!=wifi||mini_ui.connected!=edge){mini_ui.wifi=wifi;mini_ui.connected=edge;mini_invalidate();} }
void operit_ui_set_paired(bool paired) { if(mini_ui.paired!=paired){mini_ui.paired=paired;mini_invalidate();} }
void operit_ui_set_pairing_code(const char *code) {
    bool valid=code&&strlen(code)==6;
    if(valid) for(unsigned i=0;i<6;++i) if(code[i]<'0'||code[i]>'9') valid=false;
    bool hadCode=mini_ui.code[0];
    if(mini_copy(mini_ui.code,sizeof(mini_ui.code),valid?code:"")) {
        if(hadCode&&!mini_ui.code[0]&&mini_ui.page==0)mini_ui.page=10;
        mini_invalidate();
    }
}
void operit_ui_set_space_join_prompt(const char *prompt,bool busy) {
    bool changed=mini_ui.busy!=busy;mini_ui.busy=busy;
    if(!busy) changed=mini_copy(mini_ui.prompt,sizeof(mini_ui.prompt),prompt)||changed;
    if(!mini_ui.prompt[0]&&!busy&&mini_ui.page==3){mini_ui.page=10;changed=true;}
    if(changed) mini_invalidate();
}
void operit_ui_set_device_space_state(unsigned phase,const char *status,const char *first,const char *second,const char *third,bool pending) {
    bool changed=mini_ui.device_space_phase!=phase;mini_ui.device_space_phase=(uint8_t)(phase>4?0:phase);
    changed=(mini_ui.device_space_pending!=pending)||changed;mini_ui.device_space_pending=pending;
    changed=mini_copy(mini_ui.device_space_status,sizeof(mini_ui.device_space_status),status)||changed;
    changed=mini_copy(mini_ui.device_space_peers[0],sizeof(mini_ui.device_space_peers[0]),first)||changed;
    changed=mini_copy(mini_ui.device_space_peers[1],sizeof(mini_ui.device_space_peers[1]),second)||changed;
    changed=mini_copy(mini_ui.device_space_peers[2],sizeof(mini_ui.device_space_peers[2]),third)||changed;
    if(mini_ui.page==15&&mini_ui.device_space_phase!=3&&mini_ui.device_space_phase!=4){mini_ui.page=mini_ui.device_space_phase==2?14:16;mini_ui.pair_input[0]=0;changed=true;}
    if(changed)mini_invalidate();
}
#define SETTER(name,field) void operit_ui_set_##name(const char *value){if(mini_copy(mini_ui.field,sizeof(mini_ui.field),value))mini_invalidate();}
SETTER(space_state,space)
/* Message rows and the empty/error hint share one bounded display buffer. */
void operit_ui_set_chat_screen(const char *value){
    if(value&&*value) {if(mini_copy(mini_ui.chat,sizeof(mini_ui.chat),value)){follow_chat_tail();mini_invalidate();}}
    else if(!mini_ui.message_count&&mini_copy(mini_ui.chat,sizeof(mini_ui.chat),""))mini_invalidate();
}
SETTER(chat_task,task)
void operit_ui_set_speech_state(unsigned phase){
    if(phase>3)phase=0;if(mini_ui.speech_phase!=phase){mini_ui.speech_phase=(uint8_t)phase;mini_invalidate();}
}
void operit_ui_set_chat_draft(const char *value){
    if(value&&strlen(value)>=sizeof(mini_ui.draft)){
        mini_ui.draft[0]=0;
        mini_copy(mini_ui.error,sizeof(mini_ui.error),"输入超过设备草稿容量，请分段发送");mini_invalidate();return;
    }
    if(mini_copy(mini_ui.draft,sizeof(mini_ui.draft),value))mini_invalidate();
}
void operit_ui_action_error(const char *value){if(mini_copy(mini_ui.error,sizeof(mini_ui.error),value))mini_invalidate();}
void operit_ui_set_chat_preview(const char *value){(void)value;}
void operit_ui_set_chat_identity(const char *id,const char *character){
    (void)character;if(!id)id="";
    bool switched=mini_ui.chat_id[0]&&strcmp(mini_ui.chat_id,id);
    if(mini_copy(mini_ui.chat_id,sizeof(mini_ui.chat_id),id))mini_invalidate();
    if(switched){mini_ui.draft[0]=0;mini_ui.submitted[0]=0;mini_ui.conversation_count=0;if(mini_ui.page==13)mini_ui.page=2;mini_invalidate();}
}
void operit_ui_set_expression(const char *expression){
    uint8_t next=expression&&(!strcmp(expression,"sad")||!strcmp(expression,"error"))?1:
        expression&&(!strcmp(expression,"sleepy")||!strcmp(expression,"booting"))?2:0;
    if(next!=mini_ui.expression){mini_ui.expression=next;mini_invalidate();}
}
void operit_ui_set_theme(unsigned theme,bool circular){(void)theme;(void)circular;}
unsigned operit_ui_theme_index(void){return 0;}
bool operit_ui_round_icons(void){return false;}
bool operit_ui_set_emoji_style(unsigned style){return style==0;}
unsigned operit_ui_emoji_style(void){return 0;}
bool operit_emoji_color_available(void){return false;}
const char *operit_ui_chat_draft(void){return mini_ui.submitted;}
void operit_ui_submit_chat(void){
    if(mini_ui.page==13||!mini_ui.connected||!mini_ui.draft[0]||mini_ui.sending||mini_ui.speech_phase) return;
    mini_copy(mini_ui.submitted,sizeof(mini_ui.submitted),mini_ui.draft);mini_ui.sending=true;
    if(mini_ui.action) mini_ui.action("edge_send",mini_ui.context);mini_invalidate();
}
void operit_ui_chat_send_result(bool ok,const char *error){
    mini_ui.sending=false;
    if(ok) {mini_ui.draft[0]=0;if(mini_ui.page!=13)mini_ui.submitted[0]=0;} else mini_copy(mini_ui.error,sizeof(mini_ui.error),error&&*error?error:"发送失败");
    mini_invalidate();
}
/* One bounded, volatile window shared by painting and screen inspection. */
/** Appends one plain display message to the volatile window. */
void operit_ui_set_message(unsigned index,bool user,const char *text){
    if(index==0)mini_ui.chat[0]=0;
    size_t n=strlen(mini_ui.chat);
    if(n&&n+1<sizeof(mini_ui.chat))mini_ui.chat[n++]='\n';
    mini_ui.chat[n]=0;
    const char *prefix=user?"You: ":"AI: ";
    size_t count=strlen(prefix);
    if(n+count+1<sizeof(mini_ui.chat)){memcpy(mini_ui.chat+n,prefix,count);n+=count;mini_ui.chat[n]=0;}
    mini_copy(mini_ui.chat+n,sizeof(mini_ui.chat)-n,text);mini_invalidate();
}
void operit_ui_set_chat_history(bool older,bool newer){mini_ui.chat_has_older=older;mini_ui.chat_has_newer=newer;}
static void follow_chat_tail(void){
    unsigned lines=mini_chat_lines(),last=lines>CHAT_ROWS?lines-CHAT_ROWS:0;
    if(mini_ui.chat_follow_tail||mini_ui.chat_scroll>last)mini_ui.chat_scroll=last;
}
void operit_ui_finish_messages(unsigned count){
    if(!count){mini_ui.chat[0]=0;mini_ui.chat_scroll=0;mini_ui.chat_follow_tail=true;}
    follow_chat_tail();
    mini_ui.message_count=count;mini_invalidate();
}

/* Bounded, volatile display projection. No business objects persisted here. */
void operit_ui_set_plugin(unsigned index,const char *id,const char *name,unsigned status,unsigned ms){
    if(index>=6)return;
    plugin_t *p=mini_ui.plugins+index;
    char action[128]="",latency[16]="";
    snprintf(action,sizeof(action),"edge_plugin_open:%.108s",id?id:"");
    if(status==1)mini_copy(latency,sizeof(latency),"…");
    else if(status==2||status==3)snprintf(latency,sizeof(latency),"%u ms",ms>99999?99999:ms);
    else mini_copy(latency,sizeof(latency),"-- ms");
    bool changed=mini_copy(p->action,sizeof(p->action),action);
    if(changed&&mini_ui.plugin_dialog&&index==mini_ui.plugin_selected)mini_ui.plugin_dialog=false;
    changed=mini_copy(p->name,sizeof(p->name),name)||changed;
    changed=mini_copy(p->latency,sizeof(p->latency),latency)||changed;
    if(p->status!=status){p->status=status;changed=true;}
    if(mini_ui.plugin_dialog&&index==mini_ui.plugin_selected) {
        char result[24]="";
        if(status==2||status==3)snprintf(result,sizeof(result),"%s %u ms",status==2?"成功":"失败",ms>99999?99999:ms);
        else mini_copy(result,sizeof(result),status==1?"测试中":"未测试");
        changed=mini_copy(mini_ui.plugin_connect_text,sizeof(mini_ui.plugin_connect_text),result)||changed;
    }
    if(changed)mini_invalidate();
}
void operit_ui_set_plugin_test(unsigned index,unsigned status,unsigned ms,const char *error) {
    if(!mini_ui.plugin_dialog||index!=mini_ui.plugin_selected)return;
    char result[24]="";
    if(status==2||status==3)snprintf(result,sizeof(result),"%s %u ms",status==2?"成功":"失败",ms>99999?99999:ms);
    else mini_copy(result,sizeof(result),status==1?"测试中":"未测试");
    bool changed=mini_copy(mini_ui.plugin_tool_text,sizeof(mini_ui.plugin_tool_text),result);
    changed=mini_copy(mini_ui.plugin_test_error,sizeof(mini_ui.plugin_test_error),error)||changed;
    if(mini_ui.plugin_tool_status!=status){mini_ui.plugin_tool_status=(uint8_t)status;changed=true;}
    if(changed)mini_invalidate();
}
void operit_ui_set_plugin_category(bool exclusive) {
    if(mini_ui.plugins_exclusive!=exclusive){mini_ui.plugins_exclusive=exclusive;mini_ui.plugin_dialog=false;mini_invalidate();}
}
/* One bounded selected-package projection, shared by firmware and WASM. */
void operit_ui_set_plugin_details(const char *id,const char *description,const char *tools,unsigned offset,unsigned total,bool loading,const char *error) {
    if(!mini_ui.plugin_dialog||strcmp(id?id:"",mini_ui.plugins[mini_ui.plugin_selected].action+17))return;
    char info[600],heading[48]="";
    unsigned last=offset>=total?total:total-offset>3?offset+3:total;
    if(total)snprintf(heading,sizeof(heading),"\n工具: %u-%u / %u\n",offset+1,last,total);
    snprintf(info,sizeof(info),"包 ID: %.108s\n简介: %s%s%s",id,loading?"读取 Core 详情…":error&&error[0]?error:description&&description[0]?description:"暂无简介",
        heading,total&&tools?tools:"");
    bool changed=mini_copy(mini_ui.plugin_info,sizeof(mini_ui.plugin_info),info);
    if(mini_ui.plugin_tool_offset!=offset){mini_ui.plugin_info_scroll=0;changed=true;}
    if(mini_ui.plugin_tool_total!=total||mini_ui.plugin_details_loading!=loading)changed=true;
    mini_ui.plugin_tool_offset=offset;mini_ui.plugin_tool_total=total;mini_ui.plugin_details_loading=loading;
    if(changed)mini_invalidate();
}
void operit_ui_set_plugin_testing(bool testing){if(mini_ui.plugins_testing!=testing){mini_ui.plugins_testing=testing;mini_invalidate();}}
void operit_ui_finish_plugins(unsigned count,unsigned offset,unsigned total,bool loading,const char *error){
    if(count>6)count=6;
    if(mini_ui.plugin_dialog&&(mini_ui.plugin_selected>=count||!mini_ui.connected||loading))mini_ui.plugin_dialog=false;
    bool changed=mini_copy(mini_ui.plugins_error,sizeof(mini_ui.plugins_error),error);
    if(mini_ui.plugin_count!=count||mini_ui.plugin_offset!=offset||mini_ui.plugin_total!=total||mini_ui.plugins_loading!=loading)changed=true;
    mini_ui.plugin_count=count;mini_ui.plugin_offset=offset;mini_ui.plugin_total=total;mini_ui.plugins_loading=loading;
    if(changed)mini_invalidate();
}
void operit_ui_set_conversation(unsigned index,const char *id,const char *title,const char *character,bool selected){
    (void)character;
    if(mini_ui.page!=13||index<mini_ui.conversation_offset||index>=mini_ui.conversation_offset+3)return;
    conversation_t *p=mini_ui.conversations+(index-mini_ui.conversation_offset);
    char action[109]={0};
    /* Never truncate an identifier into a different selectable conversation. */
    if(id&&*id&&strlen(id)<=96)snprintf(action,sizeof(action),"edge_select:%s",id);
    bool changed=mini_copy(p->action,sizeof(p->action),action);
    changed=mini_copy(p->title,sizeof(p->title),title&&*title?title:"未命名对话")||changed;
    if(p->selected!=selected){p->selected=selected;changed=true;}
    if(changed)mini_invalidate();
}
void operit_ui_finish_conversations(unsigned count){
    if(mini_ui.page!=13)return;if(count>24)count=24;
    if(mini_ui.conversation_offset>=count&&mini_ui.conversation_offset){
        mini_ui.conversation_offset=count?((count-1)/3)*3:0;
        mini_ui.conversation_count=0;memset(mini_ui.conversations,0,sizeof(mini_ui.conversations));
        if(mini_ui.action)mini_ui.action("edge_conversations_refresh",mini_ui.context);
        mini_invalidate();return;
    }
    unsigned visible=count>mini_ui.conversation_offset?count-mini_ui.conversation_offset:0;if(visible>3)visible=3;
    if(mini_ui.conversation_count!=visible||mini_ui.conversation_total!=count){mini_ui.conversation_count=visible;mini_ui.conversation_total=count;mini_invalidate();}
}
void operit_ui_set_conversations_state(bool loading,const char *error){
    if(mini_ui.page!=13)return;
    if(mini_ui.conversations_loading!=loading){mini_ui.conversations_loading=loading;mini_invalidate();}
    if(loading||!error||!*error)mini_ui.conversations_error_shown=false;
    else if(!mini_ui.conversations_error_shown){mini_ui.conversations_error_shown=true;operit_ui_action_error(error);}
}
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
    mini_copy(bounded,sizeof(bounded),value);
    append("\"");
    for(unsigned i=0;bounded[i];++i){
        unsigned char c=(unsigned char)bounded[i];
        if(c=='"'||c=='\\')append("\\%c",c);
        else if(c<32)append("\\u%04x",c);
        else append("%c",c);
    }
    append("\"");
}
static const char *inspect_format(bool compact) {
    mini_scene();json_at=0;json_ok=true;
    append("{\"renderer\":\"mini\",\"page\":");string(operit_ui_current_page());
    append(",\"width\":320,\"height\":240,\"staticBytes\":%u,\"drawBytes\":%u,\"heapBytes\":0,\"pairingCode\":",(unsigned)operit_mini_static_bytes(),(unsigned)sizeof(mini_strip));string(mini_ui.code);
    append(",\"pluginDialogOpen\":%s,\"pluginsTesting\":%s,\"pluginCategory\":\"%s\",\"pluginInfoScroll\":%u",mini_ui.plugin_dialog?"true":"false",mini_ui.plugins_testing?"true":"false",mini_ui.plugins_exclusive?"exclusive":"general",mini_ui.plugin_info_scroll);
    append(",\"sidebarOpen\":%s,\"expression\":%u,\"chatWrapWidth\":%u,\"chatVisibleLines\":%u",mini_ui.sidebar?"true":"false",mini_ui.expression,CHAT_WRAP,CHAT_ROWS);
    append(",\"chatCachedBytes\":%u,\"chatCachedLines\":%u,\"chatScrollLine\":%u,\"nodes\":[",(unsigned)strlen(mini_ui.chat),mini_chat_lines(),mini_ui.chat_scroll);
    for(unsigned i=0;i<mini_ui.node_count;++i){
        node_t *n=mini_ui.nodes+i;
        append("%s{\"id\":",i?",":"");string(n->id);append(",\"text\":");string(n->text);
        if(!strncmp(n->id,"plugin_probe_",13)){plugin_t *p=mini_ui.plugins+(n->id[strlen(n->id)-1]-'0');append(",\"probeState\":%u,\"latency\":",p->status);string(p->latency);append(",\"latencyColor\":%u",(unsigned)rgb(p->status==2?0x71e5a2:p->status==3?0xf07878:0x849c8d));}
        if(!strncmp(n->id,"conversation_",13))append(",\"selected\":%s",mini_ui.conversations[n->id[13]-'0'].selected?"true":"false");
        if(same(n->id,"plugins_test_all"))append(",\"iconColor\":%u",(unsigned)rgb(0xffffff));
        append(",\"action\":");string(n->action);
        if(!compact)append(",\"role\":\"%s\",\"visible\":true,\"clickable\":%s",n->action?"button":"label",n->action?"true":"false");
        append(",\"rect\":{\"x\":%d,\"y\":%d,\"w\":%d,\"h\":%d},\"enabled\":%s}",n->x,n->y,n->w,n->h,n->enabled?"true":"false");
    }
    append("]}");
    return json_ok?debug_json:"{\"renderer\":\"mini\",\"error\":\"inspection capacity exceeded\",\"nodes\":[]}";
}
/* Very long package IDs may need the compact node schema; retain every hit
 * target and its real action rather than returning an empty inspection. */
static const char *inspect(void) {
    const char *result=inspect_format(false);
    return json_ok?result:inspect_format(true);
}
const char *operit_ui_debug_tree(void){return inspect();}
const char *operit_ui_debug_snapshot(void){return inspect();}
bool operit_ui_debug_tap(const char *id){
    if(!id)return false;mini_scene();
    for(unsigned i=0;i<mini_ui.node_count;++i){node_t *n=mini_ui.nodes+i;if(n->action&&n->enabled&&!strcmp(n->id,id)){
        unsigned x=n->x+n->w/2,y=n->y+n->h/2;operit_ui_set_touch(x,y,true);operit_ui_set_touch(x,y,false);return true;}}
    return false;
}
bool operit_ui_debug_swipe(const char *direction){return direction&&chat_swipe(direction);}
