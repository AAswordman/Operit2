#ifndef OPERIT_WINDOW_CHROME_H_
#define OPERIT_WINDOW_CHROME_H_

#include <flutter_linux/flutter_linux.h>
#include <gtk/gtk.h>

// Keeps GTK's caption controls and window interactions on every desktop.
void prepare_operit_window_chrome(GtkWindow* window);

// Registers appearance updates for the window containing this Flutter engine.
void register_operit_window_chrome(FlView* view);

#endif  // OPERIT_WINDOW_CHROME_H_
