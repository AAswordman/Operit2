#include "operit_mini_ui.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <time.h>
static unsigned char frame[320*240*2];
static char last_action[256];
static unsigned calls;
static void flush(const operit_ui_area_t *a,const uint8_t *p,size_t n,void *ctx){
    (void)ctx; if(a->x1!=0||a->x2!=319||a->y1<0||a->y2>=240||n!=(size_t)(a->y2-a->y1+1)*640)abort();
    memcpy(frame+a->y1*640,p,n);++calls;
}
static void action(const char *s,void *ctx){(void)ctx;snprintf(last_action,sizeof(last_action),"%s",s);}
static void scene(uint8_t *p,size_t n,unsigned y,unsigned rows,uint32_t tick,void *ctx){
    (void)ctx;for(unsigned row=0;row<rows;++row){if(y+row<24)continue;
    for(unsigned x=0;x<320;++x){uint16_t c=((x/16+(y+row)/16+tick)%2)?0x07e0:0x001f;
    p[row*640+x*2]=c;p[row*640+x*2+1]=c>>8;}}
}
static void tap(const char *id){if(!operit_ui_debug_tap(id)){fprintf(stderr,"missing node: %s\n%s\n",id,operit_ui_debug_tree());abort();}}
static void open_space(void){operit_ui_navigate_home();tap("sidebar_toggle");tap("sidebar_settings");tap("settings_space");}
static void snapshot(const char *dir,const char *name){
    unsigned before=calls;for(unsigned i=0;i<32;i++)operit_ui_pump(20);
    if(calls==before)abort();char path[1024];snprintf(path,sizeof(path),"%s/%s.rgb565",dir,name);
    FILE *f=fopen(path,"wb");if(!f)abort();fwrite(frame,1,sizeof(frame),f);fclose(f);
    snprintf(path,sizeof(path),"%s/%s.json",dir,name);f=fopen(path,"wb");if(!f)abort();
    fputs(operit_ui_debug_tree(),f);fclose(f);
}
int main(int argc,char **argv){
    if(argc!=2)return 2;
    if(!operit_ui_init(320,240,flush,NULL,action,NULL))return 3;
    snapshot(argv[1],"chat-start");
    operit_ui_set_remote_display(true);snapshot(argv[1],"remote-home");
    operit_ui_set_connection(true,true);operit_ui_set_paired(true);snapshot(argv[1],"paired");
    tap("sidebar_toggle");snapshot(argv[1],"drawer");
    operit_ui_navigate_home();operit_ui_set_pairing_code("001234");
    open_space();tap("space_pair_code");snapshot(argv[1],"pairing");
    operit_ui_set_pairing_code("");operit_ui_set_space_join_prompt("申请加入空间\n测试设备",false);
    open_space();tap("space_review");snapshot(argv[1],"approval");
    operit_ui_action_error("测试错误 Error 123");snapshot(argv[1],"error");
    operit_ui_action_error("");operit_ui_set_space_join_prompt("",false);operit_ui_navigate_home();
    operit_ui_scene_painter(scene);operit_ui_set_scene(true,1,3);snapshot(argv[1],"scene");
    operit_ui_set_scene(false,2,0);operit_ui_set_remote_display(false);operit_ui_navigate_home();
    operit_ui_set_chat_screen("第一行\nHello 中文 123\nTask completed");snapshot(argv[1],"chat-text");
    tap("sidebar_toggle");tap("sidebar_face");snapshot(argv[1],"face-default");
    operit_ui_set_expression("sad");snapshot(argv[1],"face-sad");
    operit_ui_set_expression("sleepy");snapshot(argv[1],"face-sleepy");
    operit_ui_navigate_home();
    clock_t start=clock();unsigned before=calls;
    for(unsigned i=0;i<100;i++){operit_ui_set_connection(i&1,i&1);for(unsigned j=0;j<15;j++)operit_ui_pump(20);}
    printf("{\"staticBytes\":%zu,\"drawBytes\":%zu,\"flushes\":%u,\"benchmarkFlushes\":%u,\"cpuSeconds\":%.6f,\"action\":\"%s\"}\n",
      operit_mini_static_bytes(),operit_mini_draw_bytes(),calls,calls-before,(double)(clock()-start)/CLOCKS_PER_SEC,last_action);
    return 0;
}
