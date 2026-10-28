#include "operit_mini_ui.h"
#include <emscripten.h>
#include <emscripten/stack.h>
#include <stdint.h>
static uint16_t framebuffer[320 * 240];
static unsigned generation;
EM_JS(void, report_action, (const char *value), {
    if (Module.onAction) Module.onAction(UTF8ToString(value));
});
static void flush(const operit_ui_area_t *a, const uint8_t *pixels, size_t length, void *user) {
    (void)user;
    size_t offset = 0;
    for(int y=a->y1;y<=a->y2;y++) for(int x=a->x1;x<=a->x2;x++) {
        if(offset + 1 >= length) return;
        if(x>=0 && x<320 && y>=0 && y<240) framebuffer[y*320+x]=pixels[offset]|((uint16_t)pixels[offset+1]<<8);
        offset+=2;
    }
    generation++;
}
static void action(const char *value, void *user) {
    (void)user; report_action(value);
}
EMSCRIPTEN_KEEPALIVE int simulator_init(void) {return operit_ui_init(320,240,flush,0,action,0);}
EMSCRIPTEN_KEEPALIVE const uint16_t *simulator_frame(void) {return framebuffer;}
EMSCRIPTEN_KEEPALIVE unsigned simulator_generation(void) {return generation;}
/* No dynamic UI allocator. Static RAM includes the strip and inspection buffer,
 * excludes this browser-only framebuffer and the host C stack. */
EMSCRIPTEN_KEEPALIVE unsigned simulator_heap_used(void) {return 0;}
EMSCRIPTEN_KEEPALIVE unsigned simulator_heap_total(void) {return 0;}
EMSCRIPTEN_KEEPALIVE unsigned simulator_heap_free(void) {return 0;}
EMSCRIPTEN_KEEPALIVE unsigned simulator_heap_largest(void) {return 0;}
EMSCRIPTEN_KEEPALIVE unsigned simulator_heap_peak(void) {return 0;}
/* Wasm's linear-memory C stack is checked, but is not an Xtensa/FreeRTOS watermark. */
EMSCRIPTEN_KEEPALIVE unsigned simulator_stack_size(void) {return emscripten_stack_get_base()-emscripten_stack_get_end();}
EMSCRIPTEN_KEEPALIVE unsigned simulator_stack_free(void) {return emscripten_stack_get_free();}
EMSCRIPTEN_KEEPALIVE void simulator_touch(int x,int y,int down) {operit_ui_set_touch(x,y,down!=0);}
