#include "app_visibility.h"

#include <cstring>

namespace {

struct AppVisibility {
  GWeakRef window;
  FlMethodChannel* channel;
  gboolean last;
  gboolean reported;
};

gboolean main_ui_visible(GtkWidget* window) {
  GdkWindow* native = gtk_widget_get_window(window);
  const auto state = native == nullptr ? GdkWindowState(0) : gdk_window_get_state(native);
  return gtk_widget_get_visible(window) && gtk_widget_get_mapped(window) &&
      !(state & (GDK_WINDOW_STATE_ICONIFIED | GDK_WINDOW_STATE_WITHDRAWN));
}

void publish_app_visibility(GtkWidget* window, gpointer data) {
  auto* visibility = static_cast<AppVisibility*>(data);
  const gboolean visible = main_ui_visible(window);
  if (visibility->reported && visibility->last == visible) return;
  visibility->reported = TRUE;
  visibility->last = visible;
  g_autoptr(FlValue) value = fl_value_new_bool(visible);
  fl_method_channel_invoke_method(visibility->channel, "visibilityChanged", value,
                                 nullptr, nullptr, nullptr);
}

gboolean app_window_state_changed(GtkWidget* window, GdkEventWindowState*, gpointer data) {
  publish_app_visibility(window, data);
  return FALSE;
}

void handle_app_visibility(FlMethodChannel*, FlMethodCall* call, gpointer data) {
  if (strcmp(fl_method_call_get_name(call), "getVisibility") != 0) {
    fl_method_call_respond_not_implemented(call, nullptr);
    return;
  }
  auto* visibility = static_cast<AppVisibility*>(data);
  g_autoptr(GtkWidget) window = GTK_WIDGET(g_weak_ref_get(&visibility->window));
  g_autoptr(FlValue) value = fl_value_new_bool(window != nullptr && main_ui_visible(window));
  fl_method_call_respond_success(call, value, nullptr);
}

void destroy_app_visibility(gpointer data) {
  auto* visibility = static_cast<AppVisibility*>(data);
  g_autoptr(GtkWidget) window = GTK_WIDGET(g_weak_ref_get(&visibility->window));
  if (window != nullptr) g_signal_handlers_disconnect_by_data(window, data);
  fl_method_channel_set_method_call_handler(visibility->channel, nullptr, nullptr, nullptr);
  g_object_unref(visibility->channel);
  g_weak_ref_clear(&visibility->window);
  g_free(visibility);
}

}  // namespace

void register_operit_app_visibility(FlView* view) {
  GtkWidget* window = gtk_widget_get_toplevel(GTK_WIDGET(view));
  auto* visibility = g_new0(AppVisibility, 1);
  g_weak_ref_init(&visibility->window, G_OBJECT(window));
  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  visibility->channel = fl_method_channel_new(
      fl_engine_get_binary_messenger(fl_view_get_engine(view)),
      "operit/app_visibility", FL_METHOD_CODEC(codec));
  fl_method_channel_set_method_call_handler(
      visibility->channel, handle_app_visibility, visibility, nullptr);
  g_signal_connect(window, "map", G_CALLBACK(publish_app_visibility), visibility);
  g_signal_connect(window, "unmap", G_CALLBACK(publish_app_visibility), visibility);
  g_signal_connect(window, "window-state-event", G_CALLBACK(app_window_state_changed), visibility);
  g_object_set_data_full(G_OBJECT(view), "operit-app-visibility", visibility, destroy_app_visibility);
}
