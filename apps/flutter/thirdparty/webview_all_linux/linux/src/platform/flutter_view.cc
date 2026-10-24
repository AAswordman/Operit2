#include "plugin/webview_all_linux_plugin_private.h"

#if !defined(FL_NATIVE_VIEW_COMPOSITION_API_VERSION) ||                        \
    FL_NATIVE_VIEW_COMPOSITION_API_VERSION != 1
#error                                                                         \
    "Linux WebView requires the Operit Linux native-composition engine patch."
#endif

namespace {

// Detaches weak host references when the Flutter window is destroyed.
void overlay_destroy_cb(GtkWidget *, gpointer user_data) {
  detach_linux_webview_host(static_cast<WebviewAllLinuxPlugin *>(user_data));
}

} // namespace

// Obtains the engine-owned host without reparenting Flutter's GL surface.
GtkOverlay *ensure_overlay(WebviewAllLinuxPlugin *self) {
  if (self->overlay != nullptr) {
    return self->overlay;
  }
  FlView *view = fl_plugin_registrar_get_view(self->registrar);
  if (view == nullptr) {
    return nullptr;
  }
  self->flutter_view = GTK_WIDGET(view);
  self->overlay = fl_view_get_native_view_overlay(view);
  g_object_add_weak_pointer(G_OBJECT(self->flutter_view),
                            reinterpret_cast<gpointer *>(&self->flutter_view));
  g_object_add_weak_pointer(G_OBJECT(self->overlay),
                            reinterpret_cast<gpointer *>(&self->overlay));
  g_signal_connect(self->overlay, "destroy", G_CALLBACK(overlay_destroy_cb),
                   self);
  return self->overlay;
}

// Clears window references while preserving normal Flutter input routing.
void detach_linux_webview_host(WebviewAllLinuxPlugin *self) {
  if (self->overlay != nullptr) {
    g_signal_handlers_disconnect_by_data(self->overlay, self);
    g_object_remove_weak_pointer(G_OBJECT(self->overlay),
                                 reinterpret_cast<gpointer *>(&self->overlay));
    self->overlay = nullptr;
  }
  if (self->flutter_view != nullptr) {
    g_object_remove_weak_pointer(
        G_OBJECT(self->flutter_view),
        reinterpret_cast<gpointer *>(&self->flutter_view));
    self->flutter_view = nullptr;
  }
}

// Gives keyboard and IME ownership back to Flutter when native focus ends.
void release_linux_webview_focus(LinuxWebView *webview) {
  if (webview->web_view == nullptr ||
      webview->plugin->flutter_view == nullptr) {
    return;
  }
  GtkWidget *widget = GTK_WIDGET(webview->web_view);
  GtkWidget *toplevel = gtk_widget_get_toplevel(widget);
  if (!GTK_IS_WINDOW(toplevel)) {
    return;
  }
  GtkWidget *focus = gtk_window_get_focus(GTK_WINDOW(toplevel));
  if (focus == widget ||
      (focus != nullptr && gtk_widget_is_ancestor(focus, widget))) {
    gtk_widget_grab_focus(webview->plugin->flutter_view);
  }
}
