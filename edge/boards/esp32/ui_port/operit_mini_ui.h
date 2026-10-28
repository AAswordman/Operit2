#pragma once
#include "operit_ui.h"
/* Host data/actions remain independent of the renderer.
 * This renderer performs no dynamic allocation. */
size_t operit_mini_static_bytes(void);
size_t operit_mini_draw_bytes(void);
