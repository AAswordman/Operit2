/* Page layouts and hit targets; pairing and approval remain host actions. */
#include "mini_internal.h"
#include <stdio.h>

int mini_page(void) { return mini_ui.scene_active&&!mini_ui.error[0] ? 12 : mini_ui.sidebar ? 8 : mini_ui.page; }
const char *operit_ui_current_page(void) {
    switch(mini_page()) {
        case 14: return "PairDiscovery"; case 15: return "PairCodeEntry"; case 16: return "SpaceManage";
        case 12: return "edge_scene";
        case 0:return "Pairing"; case 1:return "Settings"; case 2:return "Chat";
        case 3:return "Space"; case 4:return "Unpair"; case 5:return "LeaveSpace";
        case 7:return "Expression"; case 8:return "Sidebar";
        case 9:return "Connection"; case 10:return "SpaceSettings"; default:return "DeviceSettings";
    }
}
static const char *page_title(void) {
    switch(mini_page()) {
        case 14: return "发现节点"; case 15: return "输入配对码"; case 16: return "空间管理";
        case 12: return "edge_scene";
        case 0:return "配对码"; case 1:return "设置"; case 2:return "Operit";
        case 3:return "空间审批"; case 4:return "解除配对"; case 5:return "退出空间";
        case 7:return "表情"; case 8:return "Operit";
        case 9:return "网络连接"; case 10:return "设备空间"; default:return "设备操作";
    }
}
static void node(const char *id,const char *caption,const char *action,int x,int y,int w,int h,bool enabled,int scale) {
    if (mini_ui.node_count == NODE_LIMIT) return;
    mini_ui.nodes[mini_ui.node_count++] = (node_t){id,caption,action,x,y,w,h,rgb(enabled ? 0xecf3f0 : 0x647c6d),scale,ICON_NONE,enabled};
}
static void icon_node(const char *id,const char *caption,const char *action,int x,int y,int w,int h,bool enabled,unsigned icon) {
    node(id,caption,action,x,y,w,h,enabled,1);
    mini_ui.nodes[mini_ui.node_count-1].icon=icon;
}
void mini_scene(void) {
    mini_ui.node_count=0;
    if(mini_page()==12) {
        node("scene_title","EDGE SCENE",NULL,8,0,250,24,true,1);
        icon_node("scene_exit","","edge_scene_exit",276,0,44,24,true,ICON_CLOSE);
        return;
    }
    if(mini_page()==8 && !mini_ui.error[0]) {
        node("title","Operit",NULL,10,8,118,26,true,1);
        icon_node("sidebar_toggle","","close_sidebar",136,0,44,40,true,ICON_CLOSE);
        node("sidebar_face","","expression",49,43,82,51,true,1);
        icon_node("sidebar_expression","表情","expression",12,102,156,36,true,ICON_FACE);
        icon_node("sidebar_chat","当前对话","chat",12,143,156,36,mini_ui.connected&&!mini_ui.sending&&!mini_ui.speech_phase,ICON_CHAT);
        icon_node("sidebar_settings","设置","settings",120,190,48,44,true,ICON_SETTINGS);
        node("sidebar_backdrop","","close_sidebar",180,0,140,240,true,1);
        return;
    }
    bool detail=mini_page()==9||mini_page()==10||mini_page()==11||mini_page()==14||mini_page()==15||mini_page()==16||mini_page()==0||mini_page()==3;
    bool menuEnabled=mini_page()!=4&&mini_page()!=5;
    if(mini_page()==15) icon_node("pair_input_back","","edge_pair_cancel",0,0,44,42,mini_ui.device_space_phase!=4,ICON_BACK);
    else if(detail)icon_node("page_back","",(mini_page()==14||mini_page()==16||mini_page()==0||mini_page()==3)?"space_settings":"settings",0,0,44,42,true,ICON_BACK);
    else icon_node("sidebar_toggle","","sidebar",0,0,44,42,menuEnabled,ICON_MENU);
    if(mini_page()!=15) node("title",page_title(),NULL,44,6,142,30,true,1);
    // The large digit font needs 36 pixels plus the text inset to avoid clipping.
    if(mini_page()==15) node("pair_input",mini_ui.pair_input[0]?mini_ui.pair_input:"输入Core六位配对码",NULL,48,1,270,40,true,mini_ui.pair_input[0]?2:1);
    else node("link_status",mini_ui.connected ? "已连接" : mini_ui.paired ? "已配对 / 离线" : "未配对",NULL,188,6,130,30,true,1);
    if (mini_ui.error[0] && mini_page()!=0 && mini_page()!=3) {
        node("error",mini_ui.error,NULL,16,50,288,122,true,1);
        node("dismiss_error","返回","dismiss",16,184,288,40,true,1);
    } else if (mini_page()==0) {
        node("pairing_hint","请在 Operit 中输入验证码",NULL,16,54,288,46,true,1);
        node("pairing_code",mini_ui.code[0] ? mini_ui.code : "等待配对请求",NULL,90,112,224,48,true,mini_ui.code[0] ? 2 : 1);
        node("pairing_settings","返回设备空间","space_settings",106,186,176,30,true,1);
    } else if (mini_page()==3) {
        node("space_join_body",mini_ui.busy ? "正在处理，请稍候…" : mini_ui.prompt,NULL,16,46,288,mini_ui.error[0]&&!mini_ui.busy?82:123,true,1);
        if(mini_ui.error[0]&&!mini_ui.busy) node("space_join_error",mini_ui.error,NULL,16,130,288,46,true,1);
        node("edge_space_reject","拒绝","edge_space_reject",16,184,138,40,mini_ui.prompt[0]&&!mini_ui.busy,1);
        node("edge_space_approve","确认加入","edge_space_approve",166,184,138,40,mini_ui.prompt[0]&&!mini_ui.busy,1);
    } else if (mini_page()==4) {
        node("unpair_warning","解除配对后需重新连接。\n确认解除当前设备配对？",NULL,16,56,288,108,true,1);
        node("unpair_cancel","返回","unpair_cancel",16,184,138,40,true,1);
        node("unpair_confirm","确认解除","confirm_unpair",166,184,138,40,true,1);
    } else if (mini_page()==5) {
        node("space_leave_warning","退出当前设备空间？\n配对和 Wi-Fi 将保留。\n新空间加入仍需审批。",NULL,16,56,288,108,true,1);
        node("space_leave_cancel","返回","space_leave_cancel",16,184,138,40,true,1);
        node("space_leave_confirm","确认退出","confirm_space_leave",166,184,138,40,true,1);
    } else if (mini_page()==1) {
        node("settings_label","设备设置",NULL,16,45,288,24,true,1);
        icon_node("settings_connection","网络连接","connection",14,75,292,44,true,ICON_LINK);
        icon_node("settings_space",mini_ui.prompt[0]?"设备空间（待审批）":mini_ui.code[0]?"设备空间（待配对）":"设备空间","space_settings",14,122,292,44,true,ICON_SPACE);
        icon_node("settings_device","设备操作","operations",14,169,292,44,true,ICON_DEVICE);
    } else if (mini_page()==9) {
        node("wifi",mini_ui.wifi ? "Wi-Fi 已连接" : "Wi-Fi 未连接",NULL,16,50,288,36,true,1);
        icon_node("connection_space","配对和空间管理已移至设备空间","space_settings",14,108,292,48,true,ICON_SPACE);
    } else if (mini_page()==10) {
        node("space",mini_ui.space,NULL,16,45,288,42,true,1);
        icon_node("space_pair",mini_ui.device_space_phase==3?"继续输入配对码":"发现节点并发起配对",mini_ui.device_space_phase==3?"edge_pair_resume":"edge_pair_discover",14,91,292,34,mini_ui.device_space_phase!=1&&mini_ui.device_space_phase!=4&&!mini_ui.device_space_pending,ICON_LINK);
        node("space_pair_code",mini_ui.code[0]?"查看本机配对码":"接收配对：查看本机配对码","pairing",14,130,292,34,true,1);
        node("space_review",mini_ui.prompt[0]?"审批加入申请：点此查看":"暂无待审批的空间申请",mini_ui.prompt[0]?"edge_space_review":NULL,14,169,292,34,mini_ui.prompt[0]&&!mini_ui.busy,1);
        icon_node("space_manage","空间与配对管理","space_manage",14,208,292,28,true,ICON_SETTINGS);
    } else if (mini_page()==11) {
        icon_node("edge_new","新建对话","edge_new",14,66,292,44,mini_ui.connected&&!mini_ui.sending&&!mini_ui.speech_phase,ICON_CHAT);
        icon_node("operations_space","设备空间与配对","space_settings",14,122,292,44,true,ICON_SPACE);
    } else if (mini_page()==14) {
        static const char *ids[]={"space_peer_0","space_peer_1","space_peer_2"};
        static const char *actions[]={"edge_pair_candidate:0","edge_pair_candidate:1","edge_pair_candidate:2"};
        node("pair_discovery_status",mini_ui.device_space_status,NULL,14,44,292,42,true,1);
        for(unsigned i=0;i<3;++i) if(mini_ui.device_space_peers[i][0])
            node(ids[i],mini_ui.device_space_peers[i],actions[i],14,89+(int)i*34,292,30,mini_ui.device_space_phase==2,1);
        icon_node("pair_scan_again","重新发现","edge_pair_discover",14,198,138,30,mini_ui.device_space_phase==2,ICON_LINK);
        icon_node("pair_cancel","返回","edge_pair_cancel",166,198,138,30,mini_ui.device_space_phase!=4,ICON_BACK);
    } else if (mini_page()==15) {
        if(mini_ui.device_space_phase==3) {
            static const char *ids[]={"pair_digit_1","pair_digit_2","pair_digit_3","pair_digit_4","pair_digit_5","pair_digit_6","pair_digit_7","pair_digit_8","pair_digit_9"};
            static const char *labels[]={"1","2","3","4","5","6","7","8","9"};
            static const char *actions[]={"edge_pair_digit:1","edge_pair_digit:2","edge_pair_digit:3","edge_pair_digit:4","edge_pair_digit:5","edge_pair_digit:6","edge_pair_digit:7","edge_pair_digit:8","edge_pair_digit:9"};
            for(unsigned i=0;i<9;++i)
                node(ids[i],labels[i],actions[i],24+(int)(i%3)*94,48+(int)(i/3)*36,78,31,true,1);
            node("pair_digit_0","0","edge_pair_digit:0",24,156,78,31,true,1);
            node("pair_digit_delete","删除","edge_pair_backspace",118,156,78,31,true,1);
            node("pair_digit_submit","确认配对","edge_pair_submit",212,156,92,31,strlen(mini_ui.pair_input)==6,1);
            // Paint the hint below the keypad without consuming a touch node.
        } else node("pairing_progress",mini_ui.device_space_status,NULL,16,72,288,92,true,1);
    } else if (mini_page()==16) {
        node("space_manage_status",mini_ui.device_space_status[0]?mini_ui.device_space_status:mini_ui.paired?"本机保存有配对凭据":"本机未保存配对凭据",NULL,16,46,288,62,true,1);
        node("space_join",mini_ui.device_space_phase==3?"取消当前配对":mini_ui.device_space_pending?"取消加入申请":"申请加入已配对节点的空间",(mini_ui.device_space_pending||mini_ui.device_space_phase==3)?"edge_pair_cancel":"edge_space_join",14,114,292,32,mini_ui.device_space_phase==0||mini_ui.device_space_phase==3,1);
        icon_node("space_leave","退出当前设备空间","space_leave",14,154,292,32,!mini_ui.busy&&mini_ui.device_space_phase==0&&!mini_ui.device_space_pending,ICON_SPACE);
        icon_node("edge_unpair","解除本机配对","edge_unpair",14,194,292,32,mini_ui.paired&&mini_ui.device_space_phase==0&&!mini_ui.device_space_pending,ICON_LINK);
    } else if (mini_page()==7) {
        node("expression_face","",NULL,70,68,180,112,true,1);
        node("expression_caption","Operit",NULL,134,191,100,26,true,1);
    } else {
        node("home_face","",NULL,8,92,104,65,true,1);
        node("chat_task",mini_ui.task[0]?mini_ui.task:mini_ui.connected?"就绪":"离线",NULL,14,170,98,50,true,1);
        node("chat_text",mini_ui.chat[0] ? mini_chat_line(mini_ui.chat_scroll) : "连接 Operit 后\n开始聊天",NULL,CHAT_X,44,CHAT_W,132,true,1);
        icon_node("edge_voice",mini_ui.speech_phase==1?"结束":"","edge_voice",CHAT_X,187,40,40,mini_ui.connected&&!mini_ui.sending&&(mini_ui.speech_phase==0||mini_ui.speech_phase==1),mini_ui.speech_phase==1?ICON_CLOSE:ICON_MIC);
        node("chat_draft",mini_ui.draft[0]?mini_ui.draft:"电脑输入…",NULL,170,191,88,34,true,1);
        icon_node("edge_send","","edge_send",268,187,40,40,mini_ui.connected&&mini_ui.draft[0]&&!mini_ui.sending&&!mini_ui.speech_phase,ICON_SEND);
    }
}
