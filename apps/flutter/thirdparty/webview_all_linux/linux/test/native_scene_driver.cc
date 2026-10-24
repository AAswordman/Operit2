// Runs the real Flutter fixture through the patched Linux shell and WebKit.
#include <flutter_linux/flutter_linux.h>
#include <webview_all_linux/webview_all_linux_plugin.h>

#include <cmath>
#include <cstdio>
#include <cstring>
#include <initializer_list>

static GtkWidget *window;
static FlView *view;
static FlMethodChannel *probe;
static GtkWidget *input_box;
static const char *output_directory;
static bool ready = false;
static bool first_frame = false;
static bool pending = false;
static int stage = 0;
static int settle_ticks = 0;
static int browser_clicks = 0;
static int browser_double_clicks = 0;
static int browser_keys = 0;
static int browser_wheels = 0;
static int flutter_keys = 0;
static int result = 0;

// Records actual Dart and DOM acknowledgments from the fixture.
static void probe_call_cb(FlMethodChannel *, FlMethodCall *call, gpointer) {
  const char *method = fl_method_call_get_name(call);
  if (std::strcmp(method, "ready") == 0) {
    ready = true;
  } else if (std::strcmp(method, "browserDoubleClick") == 0) {
    ++browser_double_clicks;
    std::printf("Browser double click %d\n", browser_double_clicks);
  } else if (std::strcmp(method, "browserKey") == 0) {
    ++browser_keys;
  } else if (std::strcmp(method, "browserWheel") == 0) {
    ++browser_wheels;
  } else if (std::strcmp(method, "flutterKey") == 0) {
    ++flutter_keys;
  } else if (std::strcmp(method, "browserClick") == 0) {
    ++browser_clicks;
    std::printf("Browser click %d: %s\n", browser_clicks,
                fl_value_get_string(fl_method_call_get_args(call)));
  } else {
    fl_method_call_respond_not_implemented(call, nullptr);
    return;
  }
  fl_method_call_respond_success(call, nullptr, nullptr);
}

// Waits for Dart to complete a real route or widget lifecycle operation.
static void action_cb(GObject *object, GAsyncResult *response, gpointer) {
  GError *error = nullptr;
  FlMethodResponse *method_response = fl_method_channel_invoke_method_finish(
      FL_METHOD_CHANNEL(object), response, &error);
  if (error != nullptr || !FL_IS_METHOD_SUCCESS_RESPONSE(method_response)) {
    std::fprintf(stderr, "Fixture operation failed: %s\n",
                 error == nullptr ? "Dart method error" : error->message);
    result = 1;
    gtk_main_quit();
  }
  g_clear_object(&method_response);
  g_clear_error(&error);
  pending = false;
  settle_ticks = 0;
}

// Invokes one explicit fixture operation without timing-based dialog creation.
static void action(const char *method) {
  pending = true;
  fl_method_channel_invoke_method(probe, method, nullptr, nullptr, action_cb,
                                  nullptr);
}

// Checks a real window pixel after the embedder and GTK finish painting.
static void pixel(GdkPixbuf *image, int x, int y, int r, int g, int b) {
  const int scale = gdk_pixbuf_get_width(image) / 240;
  x *= scale;
  y *= scale;
  const auto *actual = gdk_pixbuf_get_pixels(image) +
                       y * gdk_pixbuf_get_rowstride(image) +
                       x * gdk_pixbuf_get_n_channels(image);
  std::printf("stage %d (%d,%d): %u,%u,%u expected %d,%d,%d\n", stage, x, y,
              actual[0], actual[1], actual[2], r, g, b);
  if (std::abs(actual[0] - r) > 5 || std::abs(actual[1] - g) > 5 ||
      std::abs(actual[2] - b) > 5) {
    result = 1;
  }
}

// Saves visual evidence from the actual Flutter window.
static GdkPixbuf *screenshot() {
  GdkPixbuf *image;
  if (g_strcmp0(g_getenv("OPERIT_SCENE_CAPTURE"), "gtk") == 0) {
    const int scale = gtk_widget_get_scale_factor(window);
    auto *surface = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, 240 * scale,
                                               160 * scale);
    cairo_surface_set_device_scale(surface, scale, scale);
    auto *cr = cairo_create(surface);
    gtk_widget_draw(window, cr);
    cairo_destroy(cr);
    image =
        gdk_pixbuf_get_from_surface(surface, 0, 0, 240 * scale, 160 * scale);
    cairo_surface_destroy(surface);
  } else {
    image = gdk_pixbuf_get_from_window(gtk_widget_get_window(window), 0, 0, 240,
                                       160);
  }
  g_assert(image != nullptr);
  auto *path =
      g_strdup_printf("%s/flutter-stage-%d.png", output_directory, stage);
  GError *error = nullptr;
  gdk_pixbuf_save(image, path, "png", &error, nullptr);
  g_assert_no_error(error);
  g_free(path);
  return image;
}

// Finds the engine-owned input surface above the interleaved paint scene.
static GtkWidget *find_input_box() {
  GList *roots = gtk_container_get_children(GTK_CONTAINER(view));
  g_assert_cmpint(g_list_length(roots), ==, 1);
  GList *children = gtk_container_get_children(GTK_CONTAINER(roots->data));
  GtkWidget *found = nullptr;
  for (GList *item = children; item != nullptr; item = item->next) {
    if (GTK_IS_EVENT_BOX(item->data))
      found = GTK_WIDGET(item->data);
  }
  g_list_free(children);
  g_list_free(roots);
  g_assert(found != nullptr);
  return found;
}

// Enters Flutter's real GDK input boundary and lets Flutter route the click.
static void click_browser() {
  auto *area = gtk_bin_get_child(GTK_BIN(input_box));
  auto *target = gtk_widget_get_window(area);
  auto *seat = gdk_display_get_default_seat(gtk_widget_get_display(input_box));
  for (auto type : {GDK_BUTTON_PRESS, GDK_BUTTON_RELEASE}) {
    GdkEvent *event = gdk_event_new(type);
    event->button.window = GDK_WINDOW(g_object_ref(target));
    event->button.send_event = TRUE;
    event->button.time = static_cast<guint32>(g_get_monotonic_time() / 1000);
    event->button.x = 30;
    event->button.y = 50;
    event->button.button = 1;
    event->button.state = type == GDK_BUTTON_RELEASE ? GDK_BUTTON1_MASK : 0;
    gdk_event_set_device(event, gdk_seat_get_pointer(seat));
    gtk_widget_event(input_box, event);
    gdk_event_free(event);
  }
}

// Delivers keyboard events through GTK window focus and normal IME ownership.
static void type_key(guint keyval) {
  auto *seat = gdk_display_get_default_seat(gtk_widget_get_display(window));
  GdkKeymapKey *keys = nullptr;
  gint count = 0;
  g_assert(gdk_keymap_get_entries_for_keyval(
      gdk_keymap_get_for_display(gtk_widget_get_display(window)), keyval, &keys,
      &count));
  for (auto type : {GDK_KEY_PRESS, GDK_KEY_RELEASE}) {
    auto *event = gdk_event_new(type);
    event->key.window = GDK_WINDOW(g_object_ref(gtk_widget_get_window(window)));
    event->key.send_event = TRUE;
    event->key.time = static_cast<guint32>(g_get_monotonic_time() / 1000);
    event->key.keyval = keyval;
    event->key.hardware_keycode = keys[0].keycode;
    event->key.group = keys[0].group;
    gdk_event_set_device(event, gdk_seat_get_keyboard(seat));
    gtk_widget_event(window, event);
    gdk_event_free(event);
  }
  g_free(keys);
}

// Sends a real wheel event through Flutter's pointer-signal resolver.
static void scroll_browser() {
  auto *event = gdk_event_new(GDK_SCROLL);
  auto *area = gtk_bin_get_child(GTK_BIN(input_box));
  event->scroll.window = GDK_WINDOW(g_object_ref(gtk_widget_get_window(area)));
  event->scroll.send_event = TRUE;
  event->scroll.time = static_cast<guint32>(g_get_monotonic_time() / 1000);
  event->scroll.x = 30;
  event->scroll.y = 50;
  event->scroll.direction = GDK_SCROLL_SMOOTH;
  event->scroll.delta_y = 1;
  auto *seat = gdk_display_get_default_seat(gtk_widget_get_display(window));
  gdk_event_set_device(event, gdk_seat_get_pointer(seat));
  gtk_widget_event(input_box, event);
  gdk_event_free(event);
}

// Advances only after the real browsers and Flutter operation acknowledgments.
static gboolean advance_cb(gpointer) {
  if (!ready || !first_frame || pending || ++settle_ticks < 3) {
    return G_SOURCE_CONTINUE;
  }
  settle_ticks = 0;
  if (stage == 0) {
    auto *image = screenshot();
    pixel(image, 10, 10, 255, 0, 0);
    pixel(image, 30, 50, 127, 128, 0);
    pixel(image, 70, 50, 191, 192, 128);
    pixel(image, 90, 50, 95, 96, 192);
    g_object_unref(image);
    if (g_getenv("OPERIT_SCENE_CLOSE_LIVE") != nullptr) {
      gtk_main_quit();
      return G_SOURCE_REMOVE;
    }
    input_box = find_input_box();
    click_browser();
    stage = 1;
  } else if (stage == 1) {
    if (browser_clicks != 1)
      return G_SOURCE_CONTINUE;
    type_key(GDK_KEY_a);
    scroll_browser();
    stage = 10;
  } else if (stage == 10) {
    if (browser_keys != 1 || browser_wheels != 1)
      return G_SOURCE_CONTINUE;
    action("showDialog");
    stage = 2;
  } else if (stage == 2) {
    auto *image = screenshot();
    pixel(image, 30, 50, 63, 64, 0);
    pixel(image, 70, 50, 95, 96, 64);
    pixel(image, 90, 50, 47, 48, 96);
    pixel(image, 130, 85, 255, 255, 255);
    g_object_unref(image);
    click_browser();
    type_key(GDK_KEY_b);
    scroll_browser();
    stage = 3;
  } else if (stage == 3) {
    if (flutter_keys != 1)
      return G_SOURCE_CONTINUE;
    g_assert_cmpint(browser_clicks, ==, 1);
    g_assert_cmpint(browser_keys, ==, 1);
    g_assert_cmpint(browser_wheels, ==, 1);
    action("dismissDialog");
    stage = 4;
  } else if (stage == 4) {
    auto *image = screenshot();
    pixel(image, 30, 50, 127, 128, 0);
    pixel(image, 90, 50, 95, 96, 192);
    g_object_unref(image);
    click_browser();
    stage = 5;
  } else if (stage == 5) {
    if (browser_clicks != 2)
      return G_SOURCE_CONTINUE;
    action("removeSecond");
    stage = 6;
  } else if (stage == 6) {
    auto *image = screenshot();
    pixel(image, 90, 50, 191, 192, 128);
    pixel(image, 200, 50, 255, 0, 0);
    g_object_unref(image);
    click_browser();
    click_browser();
    stage = 7;
  } else if (stage == 7) {
    if (browser_double_clicks != 1)
      return G_SOURCE_CONTINUE;
    g_assert_cmpint(browser_clicks, ==, 4);
    action("dispose");
    stage = 8;
  } else {
    gtk_main_quit();
    return G_SOURCE_REMOVE;
  }
  return G_SOURCE_CONTINUE;
}

// Reports enough state to diagnose a failed native or Dart startup.
static gboolean timeout_cb(gpointer) {
  std::fprintf(stderr,
               "Flutter fixture timeout: ready=%d first_frame=%d pending=%d "
               "stage=%d clicks=%d keys=%d wheels=%d flutter_keys=%d\n",
               ready, first_frame, pending, stage, browser_clicks, browser_keys,
               browser_wheels, flutter_keys);
  result = 1;
  gtk_main_quit();
  return G_SOURCE_REMOVE;
}

// Marks the first scene submitted by the actual Flutter rendering thread.
static void first_frame_cb(FlView *, gpointer) { first_frame = true; }

// Starts the patched Linux shell, real plugin, and compiled Flutter fixture.
int main(int argc, char **argv) {
  gtk_init(&argc, &argv);
  if (argc != 4) {
    std::fprintf(
        stderr,
        "Usage: native_scene_driver ASSETS ICU_DATA OUTPUT_DIRECTORY\n");
    return 2;
  }
  output_directory = argv[3];
  g_mkdir_with_parents(output_directory, 0755);
  g_log_set_always_fatal(
      static_cast<GLogLevelFlags>(G_LOG_LEVEL_ERROR | G_LOG_LEVEL_CRITICAL));
  auto *project = fl_dart_project_new();
  fl_dart_project_set_assets_path(project, argv[1]);
  fl_dart_project_set_icu_data_path(project, argv[2]);
  view = fl_view_new(project);
  g_object_unref(project);
  auto *registrar = fl_plugin_registry_get_registrar_for_plugin(
      FL_PLUGIN_REGISTRY(view), "WebviewAllLinuxPlugin");
  webview_all_linux_plugin_register_with_registrar(registrar);
  g_object_unref(registrar);
  auto *messenger = fl_engine_get_binary_messenger(fl_view_get_engine(view));
  auto *codec = fl_standard_method_codec_new();
  probe = fl_method_channel_new(messenger, "operit/linux_native_scene_probe",
                                FL_METHOD_CODEC(codec));
  g_object_unref(codec);
  fl_method_channel_set_method_call_handler(probe, probe_call_cb, nullptr,
                                            nullptr);
  g_signal_connect(view, "first-frame", G_CALLBACK(first_frame_cb), nullptr);
  window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
  gtk_window_set_decorated(GTK_WINDOW(window), FALSE);
  gtk_window_set_default_size(GTK_WINDOW(window), 240, 160);
  gtk_container_add(GTK_CONTAINER(window), GTK_WIDGET(view));
  gtk_widget_show_all(window);
  g_timeout_add(100, advance_cb, nullptr);
  g_timeout_add_seconds(45, timeout_cb, nullptr);
  gtk_main();
  g_object_unref(probe);
  gtk_widget_destroy(window);
  std::printf("Flutter native scene integration: %s\n",
              result == 0 ? "PASS" : "FAIL");
  return result;
}
