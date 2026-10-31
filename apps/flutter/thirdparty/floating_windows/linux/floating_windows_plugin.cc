#include "include/floating_windows/floating_windows_plugin.h"
#include <cmath>
#include <stdexcept>
#include <cstring>
#include <initializer_list>

namespace {
/// Keeps widget ownership and pointer state scoped to one engine.
struct WidgetHost {
  FlView* view;
  bool configured = false;
  bool pointer_down = false;
  guint button = 0;
  guint32 time = 0;
  double root_x = 0;
  double root_y = 0;
  bool draggable = false;
  bool dragging = false;
  int window_x = 0, window_y = 0;
  FlMethodChannel* channel = nullptr;
  guint motion_signal = 0;
  gulong motion_hook = 0;
  guint press_signal = 0;
  guint release_signal = 0;
  gulong press_hook = 0;
  gulong release_hook = 0;
};

/// Reads a validated logical dimension from the standard method codec.
int Dimension(FlValue* arguments, const char* name) {
  if (arguments == nullptr || fl_value_get_type(arguments) != FL_VALUE_TYPE_MAP) {
    throw std::invalid_argument("Expected widget dimensions");
  }
  FlValue* value = fl_value_lookup_string(arguments, name);
  if (value == nullptr) throw std::invalid_argument("Missing widget dimension");
  double number;
  switch (fl_value_get_type(value)) {
    case FL_VALUE_TYPE_INT: number = fl_value_get_int(value); break;
    case FL_VALUE_TYPE_FLOAT: number = fl_value_get_float(value); break;
    default: throw std::invalid_argument("Widget dimensions must be numeric");
  }
  if (!std::isfinite(number) || number < 16 || number > 4096) {
    throw std::invalid_argument("Widget dimensions must be between 16 and 4096");
  }
  return static_cast<int>(std::lround(number));
}

/// Requires real alpha composition instead of producing an opaque substitute.
void RequireComposition(GtkWindow* window) {
  GdkScreen* screen = gtk_window_get_screen(window);
  if (!gdk_screen_is_composited(screen) || gdk_screen_get_rgba_visual(screen) == nullptr) {
    throw std::runtime_error("The GTK display requires an active alpha compositor");
  }
}

/// Records the original press timestamp needed by the compositor's move protocol.
gboolean PointerEvent(GSignalInvocationHint*, guint, const GValue* values, gpointer data) {
  auto* host = static_cast<WidgetHost*>(data);
  auto* source = GTK_WIDGET(g_value_get_object(values));
  if (source != GTK_WIDGET(host->view) &&
      gtk_widget_get_ancestor(source, fl_view_get_type()) != GTK_WIDGET(host->view)) return TRUE;
  auto* event = static_cast<GdkEvent*>(g_value_get_boxed(values + 1));
  if (event->type == GDK_BUTTON_PRESS) {
    host->pointer_down = true;
    host->button = event->button.button;
    host->time = event->button.time;
    host->root_x = event->button.x_root;
    host->root_y = event->button.y_root;
    host->dragging = false;
    gtk_window_get_position(GTK_WINDOW(gtk_widget_get_toplevel(GTK_WIDGET(host->view))), &host->window_x, &host->window_y);
  }
  if (host->configured && host->draggable && host->pointer_down && host->button == 1 &&
      (event->type == GDK_MOTION_NOTIFY || event->type == GDK_BUTTON_RELEASE)) {
    double x, y; gdk_event_get_root_coords(event, &x, &y);
    const double dx = x - host->root_x, dy = y - host->root_y;
    if (!host->dragging && std::hypot(dx, dy) >= 4) {
      host->dragging = true;
      fl_method_channel_invoke_method(host->channel, "dragStarted", nullptr, nullptr, nullptr, nullptr);
    }
    if (host->dragging) gtk_window_move(GTK_WINDOW(gtk_widget_get_toplevel(GTK_WIDGET(host->view))),
      host->window_x + std::lround(dx), host->window_y + std::lround(dy));
  }
  if (event->type == GDK_BUTTON_RELEASE) { host->pointer_down = false; host->dragging = false; }
  return TRUE;
}

/// Releases the per-view host after GTK disconnects the owning view's signals.
void DestroyHost(gpointer data) {
  auto* host = static_cast<WidgetHost*>(data);
  g_signal_remove_emission_hook(host->motion_signal, host->motion_hook);
  g_signal_remove_emission_hook(host->press_signal, host->press_hook);
  g_signal_remove_emission_hook(host->release_signal, host->release_hook);
  delete host;
}

/// Closes after the method reply has been sent to the child engine.
gboolean CloseWindow(gpointer data) {
  gtk_window_close(GTK_WINDOW(data));
  return G_SOURCE_REMOVE;
}

/// Applies explicit geometry and input policy to this GTK surface.
void Apply(WidgetHost* host, GtkWindow* window, FlValue* args) {
  if (!args || fl_value_get_type(args) != FL_VALUE_TYPE_MAP) throw std::invalid_argument("Expected window options");
  if (FlValue* value = fl_value_lookup_string(args, "draggable")) {
    if (fl_value_get_type(value) != FL_VALUE_TYPE_BOOL) throw std::invalid_argument("draggable must be boolean");
    host->draggable = fl_value_get_bool(value);
    if (!host->draggable) { host->pointer_down = false; host->dragging = false; }
  }
  int width, height, x, y;
  gtk_window_get_size(window, &width, &height);
  gtk_window_get_position(window, &x, &y);
  if (fl_value_lookup_string(args,"width")) width = Dimension(args,"width");
  if (fl_value_lookup_string(args,"height")) height = Dimension(args,"height");
  gtk_window_resize(window, width, height);
  for (const auto* key : {"x", "y"}) {
    FlValue* value = fl_value_lookup_string(args, key);
    if (!value) continue;
    if (g_getenv("WAYLAND_DISPLAY")) throw std::runtime_error("Absolute positioning is unavailable on Wayland");
    double number = fl_value_get_type(value) == FL_VALUE_TYPE_FLOAT ? fl_value_get_float(value)
      : fl_value_get_type(value) == FL_VALUE_TYPE_INT ? fl_value_get_int(value) : NAN;
    if (!std::isfinite(number) || std::abs(number) > 1000000) throw std::invalid_argument("Invalid coordinate");
    if (strcmp(key,"x") == 0) x = static_cast<int>(number); else y = static_cast<int>(number);
  }
  if (fl_value_lookup_string(args,"x") || fl_value_lookup_string(args,"y")) gtk_window_move(window,x,y);
  for (const auto* key : {"alwaysOnTop", "focusable", "clickThrough"}) {
    FlValue* value = fl_value_lookup_string(args,key);
    if (!value) continue;
    if (fl_value_get_type(value) != FL_VALUE_TYPE_BOOL) throw std::invalid_argument("Window flag must be boolean");
    bool enabled = fl_value_get_bool(value);
    if (strcmp(key,"alwaysOnTop") == 0) gtk_window_set_keep_above(window,enabled);
    if (strcmp(key,"focusable") == 0) gtk_window_set_accept_focus(window,enabled);
    if (strcmp(key,"clickThrough") == 0) gdk_window_set_pass_through(gtk_widget_get_window(GTK_WIDGET(window)),enabled);
  }
}

/// Applies only content-window operations to this channel's owning GTK window.
void HandleMethod(FlMethodChannel*, FlMethodCall* call, gpointer data) {
  auto* host = static_cast<WidgetHost*>(data);
  GtkWidget* top = gtk_widget_get_toplevel(GTK_WIDGET(host->view));
  if (!GTK_IS_WINDOW(top)) {
    fl_method_call_respond_error(call, "WIDGET_WINDOW_CLOSED", "The engine has no window", nullptr, nullptr);
    return;
  }
  auto* window = GTK_WINDOW(top);
  const char* method = fl_method_call_get_name(call);
  FlValue* args = fl_method_call_get_args(call);
  try {
    if (strcmp(method, "capabilities") == 0) {
      g_autoptr(FlValue) result = fl_value_new_map();
      const bool supported = gdk_screen_is_composited(gtk_window_get_screen(window)) &&
          gdk_screen_get_rgba_visual(gtk_window_get_screen(window)) && !g_getenv("WAYLAND_DISPLAY");
      for (const auto* key : {"supported", "permissionGranted", "positioning", "alwaysOnTop", "clickThrough", "focusable"})
        fl_value_set_string_take(result,key,fl_value_new_bool(supported));
      fl_method_call_respond_success(call,result,nullptr); return;
    } else if (strcmp(method, "requestPermission") == 0) {
      g_autoptr(FlValue) yes = fl_value_new_bool(TRUE);
      fl_method_call_respond_success(call,yes,nullptr); return;
    } else if (strcmp(method, "getBounds") == 0) {
      if (!host->configured) throw std::runtime_error("Configure the floating window first");
      int x, y, width, height;
      gtk_window_get_position(window, &x, &y); gtk_window_get_size(window, &width, &height);
      g_autoptr(FlValue) bounds = fl_value_new_map();
      fl_value_set_string_take(bounds,"x",fl_value_new_int(x));
      fl_value_set_string_take(bounds,"y",fl_value_new_int(y));
      fl_value_set_string_take(bounds,"width",fl_value_new_int(width));
      fl_value_set_string_take(bounds,"height",fl_value_new_int(height));
      fl_method_call_respond_success(call,bounds,nullptr); return;
    } else if (strcmp(method, "configure") == 0) {
      const int width = Dimension(args, "width");
      const int height = Dimension(args, "height");
      FlValue* role = fl_value_lookup_string(args, "role");
      if (role == nullptr || fl_value_get_type(role) != FL_VALUE_TYPE_STRING ||
          strcmp(fl_value_get_string(role), "floating_windows.window") != 0) {
        throw std::invalid_argument("An explicit widget role is required");
      }
      RequireComposition(window);
      // GTK visuals must be chosen before realization. Never recreate a running engine.
      GdkVisual* rgba = gdk_screen_get_rgba_visual(gtk_window_get_screen(window));
      if (gtk_widget_get_visual(top) != rgba) {
        throw std::runtime_error("The window was created without an RGBA visual; transparent widgets require RGBA engine windows");
      }
      gtk_window_set_decorated(window, FALSE);
      GtkWidget* titlebar = gtk_window_get_titlebar(window);
      if (titlebar != nullptr) gtk_widget_hide(titlebar);
      gtk_window_set_skip_taskbar_hint(window, TRUE);
      gtk_window_set_skip_pager_hint(window, TRUE);
      gtk_window_set_keep_above(window, TRUE);
      gtk_window_set_accept_focus(window, FALSE);
      gtk_window_set_focus_on_map(window, FALSE);
      gtk_widget_set_app_paintable(top, TRUE);
      g_autoptr(GtkCssProvider) css = gtk_css_provider_new();
      g_autoptr(GError) error = nullptr;
      gtk_css_provider_load_from_data(css,
          "window { background-color: transparent; box-shadow: none; border: none; } decoration { box-shadow: none; border: none; }",
          -1, &error);
      if (error != nullptr) throw std::runtime_error(error->message);
      gtk_style_context_add_provider(gtk_widget_get_style_context(top), GTK_STYLE_PROVIDER(css), GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
      const GdkRGBA clear = {0, 0, 0, 0};
      fl_view_set_background_color(host->view, &clear);
      gtk_window_resize(window, width, height);
      Apply(host, window, args);
      host->configured = true;
    } else if (strcmp(method,"update") == 0 || strcmp(method,"show") == 0 ||
               strcmp(method,"hide") == 0 || strcmp(method,"close") == 0) {
      if (!host->configured) throw std::runtime_error("Configure the floating window first");
      if (strcmp(method,"update") == 0) Apply(host,window,args);
      else if (strcmp(method,"show") == 0) gtk_widget_show(top);
      else if (strcmp(method,"hide") == 0) gtk_widget_hide(top);
      else {
        fl_method_call_respond_success(call,nullptr,nullptr);
        g_idle_add_full(G_PRIORITY_DEFAULT_IDLE,CloseWindow,g_object_ref(window),g_object_unref); return;
      }
    } else {
      fl_method_call_respond_not_implemented(call,nullptr); return;
    }
    fl_method_call_respond_success(call, nullptr, nullptr);
  } catch (const std::exception& error) {
    fl_method_call_respond_error(call, "FLOATING_WINDOW_ERROR", error.what(), nullptr, nullptr);
  }
}

/// Disconnects the channel before releasing the per-view state.
void DestroyChannel(gpointer data) {
  auto* channel = FL_METHOD_CHANNEL(data);
  fl_method_channel_set_method_call_handler(channel, nullptr, nullptr, nullptr);
  g_object_unref(channel);
}
}  // namespace

/// Registers the same channel contract on each generated Flutter engine.
void floating_windows_plugin_register_with_registrar(FlPluginRegistrar* registrar) {
  FlView* view = fl_plugin_registrar_get_view(registrar);
  auto* host = new WidgetHost{view};
  g_object_set_data_full(G_OBJECT(view), "floating-windows-host", host, DestroyHost);
  gtk_widget_add_events(GTK_WIDGET(view), GDK_POINTER_MOTION_MASK | GDK_BUTTON_PRESS_MASK | GDK_BUTTON_RELEASE_MASK);
  host->motion_signal = g_signal_lookup("motion-notify-event", GTK_TYPE_WIDGET);
  host->motion_hook = g_signal_add_emission_hook(host->motion_signal, 0, PointerEvent, host, nullptr);
  host->press_signal = g_signal_lookup("button-press-event", GTK_TYPE_WIDGET);
  host->release_signal = g_signal_lookup("button-release-event", GTK_TYPE_WIDGET);
  host->press_hook = g_signal_add_emission_hook(host->press_signal, 0, PointerEvent, host, nullptr);
  host->release_hook = g_signal_add_emission_hook(host->release_signal, 0, PointerEvent, host, nullptr);
  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  FlMethodChannel* channel = fl_method_channel_new(fl_plugin_registrar_get_messenger(registrar),
      "floating_windows/window", FL_METHOD_CODEC(codec));
  host->channel = channel;
  fl_method_channel_set_method_call_handler(channel, HandleMethod, host, nullptr);
  g_object_set_data_full(G_OBJECT(view), "floating-windows-channel", channel, DestroyChannel);
}
