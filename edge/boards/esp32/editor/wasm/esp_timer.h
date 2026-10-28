#pragma once
#include <stdint.h>
#include <emscripten.h>
static inline int64_t esp_timer_get_time(void) { return (int64_t)(emscripten_get_now() * 1000.0); }
