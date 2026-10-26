#include "common/method_channel_utils.h"
#include "webview/webview_internal.h"

#include <cmath>
#include <cstdint>

// Forwards only Flutter-hit-tested events to the native WebKit input surface.
void dispatch_linux_webview_pointer(LinuxWebView *webview, FlValue *args,
                                    FlMethodCall *method_call) {
  static const GdkEventType types[] = {
      GDK_ENTER_NOTIFY,   GDK_LEAVE_NOTIFY, GDK_MOTION_NOTIFY, GDK_BUTTON_PRESS,
      GDK_BUTTON_RELEASE, GDK_SCROLL,       GDK_TOUCH_BEGIN,   GDK_TOUCH_UPDATE,
      GDK_TOUCH_END,      GDK_TOUCH_CANCEL};
  const gint type = map_lookup_int(args, "type", -1);
  const double x = map_lookup_double(args, "x", NAN);
  const double y = map_lookup_double(args, "y", NAN);
  if (type < 0 || type >= static_cast<gint>(G_N_ELEMENTS(types)) ||
      !std::isfinite(x) || !std::isfinite(y)) {
    respond(method_call,
            error_response("pointer_error", "Invalid pointer event."));
    return;
  }
  GtkWidget *widget = GTK_WIDGET(webview->web_view);
  // Queued pointer events can outlive an unmounted Flutter platform view.
  if (!webview->visible || !gtk_widget_get_mapped(widget)) {
    respond(method_call, success_response());
    return;
  }
  GdkWindow *window = gtk_widget_get_window(widget);
  GdkSeat *seat = gdk_display_get_default_seat(gtk_widget_get_display(widget));
  GdkDevice *device = gdk_seat_get_pointer(seat);
  const guint32 time = static_cast<guint32>(map_lookup_int(args, "time", 0));
  const guint state = static_cast<guint>(map_lookup_int(args, "modifiers", 0));
  gint root_x;
  gint root_y;
  gdk_window_get_origin(window, &root_x, &root_y);
  GdkEvent *event = gdk_event_new(types[type]);
  event->any.window = GDK_WINDOW(g_object_ref(window));
  event->any.send_event = TRUE;
  gdk_event_set_device(event, device);
  switch (event->type) {
  case GDK_ENTER_NOTIFY:
  case GDK_LEAVE_NOTIFY:
    event->crossing.time = time;
    event->crossing.x = x;
    event->crossing.y = y;
    event->crossing.x_root = root_x + x;
    event->crossing.y_root = root_y + y;
    event->crossing.state = state;
    event->crossing.mode = GDK_CROSSING_NORMAL;
    event->crossing.detail = GDK_NOTIFY_NONLINEAR;
    break;
  case GDK_MOTION_NOTIFY:
    event->motion.time = time;
    event->motion.x = x;
    event->motion.y = y;
    event->motion.x_root = root_x + x;
    event->motion.y_root = root_y + y;
    event->motion.state = state;
    break;
  case GDK_BUTTON_PRESS:
  case GDK_BUTTON_RELEASE:
    event->button.time = time;
    event->button.x = x;
    event->button.y = y;
    event->button.x_root = root_x + x;
    event->button.y_root = root_y + y;
    event->button.state = state;
    event->button.button = map_lookup_int(args, "button", 0);
    if (event->type == GDK_BUTTON_PRESS) {
      gtk_widget_grab_focus(widget);
    }
    break;
  case GDK_SCROLL:
    event->scroll.time = time;
    event->scroll.x = x;
    event->scroll.y = y;
    event->scroll.x_root = root_x + x;
    event->scroll.y_root = root_y + y;
    event->scroll.state = state;
    event->scroll.direction = GDK_SCROLL_SMOOTH;
    event->scroll.delta_x = map_lookup_double(args, "scrollX", 0) / 40.0;
    event->scroll.delta_y = map_lookup_double(args, "scrollY", 0) / 40.0;
    break;
  case GDK_TOUCH_BEGIN:
  case GDK_TOUCH_UPDATE:
  case GDK_TOUCH_END:
  case GDK_TOUCH_CANCEL:
    event->touch.time = time;
    event->touch.x = x;
    event->touch.y = y;
    event->touch.x_root = root_x + x;
    event->touch.y_root = root_y + y;
    event->touch.state = state;
    event->touch.sequence = reinterpret_cast<GdkEventSequence *>(
        static_cast<intptr_t>(map_lookup_int(args, "pointer", 0)) + 1);
    if (event->type == GDK_TOUCH_BEGIN) {
      gtk_widget_grab_focus(widget);
    }
    break;
  default:
    g_assert_not_reached();
  }
  gtk_widget_event(widget, event);
  gdk_event_free(event);
  respond(method_call, success_response());
}
