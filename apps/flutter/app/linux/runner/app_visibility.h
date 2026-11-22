#ifndef OPERIT_APP_VISIBILITY_H_
#define OPERIT_APP_VISIBILITY_H_

#include <flutter_linux/flutter_linux.h>
#include <gtk/gtk.h>

// Registers main UI visibility once; never install this on auxiliary engines.
void register_operit_app_visibility(FlView* view);

#endif  // OPERIT_APP_VISIBILITY_H_
