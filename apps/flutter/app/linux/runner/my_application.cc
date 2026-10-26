#include "my_application.h"

#include <flutter_linux/flutter_linux.h>
#include <desktop_multi_window/desktop_multi_window_plugin.h>
#include <file_selector_linux/file_selector_plugin.h>
#ifdef GDK_WINDOWING_X11
#include <gdk/gdkx.h>
#endif

#include "flutter/generated_plugin_registrant.h"
#include "crash_channel.h"
#include "operit_runtime_channel.h"

struct _MyApplication {
  GtkApplication parent_instance;
  char** dart_entrypoint_arguments;
};

G_DEFINE_TYPE(MyApplication, my_application, GTK_TYPE_APPLICATION)

/// Quotes one executable path according to the freedesktop desktop-entry Exec grammar.
static gchar* quote_desktop_exec_argument(const gchar* executable) {
  GString* quoted = g_string_new("\"");
  for (const gchar* character = executable; *character != '\0'; ++character) {
    switch (*character) {
      case '\\': g_string_append(quoted, "\\\\"); break;
      case '"': case '`': case '$':
        g_string_append(quoted, "\\\\");
        g_string_append_c(quoted, *character);
        break;
      case '%': g_string_append(quoted, "%%"); break;
      case '\n': g_string_append(quoted, "\\n"); break;
      case '\r': g_string_append(quoted, "\\r"); break;
      case '\t': g_string_append(quoted, "\\t"); break;
      default: g_string_append_c(quoted, *character);
    }
  }
  g_string_append_c(quoted, '"');
  return g_string_free(quoted, FALSE);
}

/// Registers only the file picker needed by the standalone rescue Flutter screen.
static void register_data_rescue_plugins(FlPluginRegistry* registry) {
  g_autoptr(FlPluginRegistrar) registrar =
      fl_plugin_registry_get_registrar_for_plugin(registry, "FileSelectorPlugin");
  file_selector_plugin_register_with_registrar(registrar);
}

/// Publishes the app preference through the GTK appearance consumed by WebKit.
static void handle_webview_theme(FlMethodChannel*, FlMethodCall* call, gpointer) {
  if (strcmp(fl_method_call_get_name(call), "setPreferredColorScheme") != 0) {
    fl_method_call_respond_not_implemented(call, nullptr);
    return;
  }
  FlValue* arguments = fl_method_call_get_args(call);
  if (fl_value_get_type(arguments) != FL_VALUE_TYPE_STRING) {
    fl_method_call_respond_error(call, "invalid_color_scheme", "Expected dark or light", nullptr, nullptr);
    return;
  }
  const gchar* scheme = fl_value_get_string(arguments);
  if (strcmp(scheme, "dark") != 0 && strcmp(scheme, "light") != 0) {
    fl_method_call_respond_error(call, "invalid_color_scheme", "Expected dark or light", nullptr, nullptr);
    return;
  }
  g_object_set(gtk_settings_get_default(), "gtk-application-prefer-dark-theme",
               strcmp(scheme, "dark") == 0, nullptr);
  fl_method_call_respond_success(call, nullptr, nullptr);
}

/// Registers the common WebView appearance channel for a Flutter engine.
static void register_webview_theme(FlView* view) {
  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  g_autoptr(FlMethodChannel) channel = fl_method_channel_new(
      fl_engine_get_binary_messenger(fl_view_get_engine(view)),
      "operit/webview_theme", FL_METHOD_CODEC(codec));
  fl_method_channel_set_method_call_handler(channel, handle_webview_theme, nullptr, nullptr);
}

/// Registers this installed executable for session-bus Plugin SDK activation.
static void register_plugin_sdk_activation() {
  g_autoptr(GError) error = nullptr;
  g_autofree gchar* executable = g_file_read_link("/proc/self/exe", &error);
  if (executable == nullptr) {
    g_error("Cannot resolve Operit activation executable: %s", error->message);
  }
  g_autofree gchar* directory = g_build_filename(
      g_get_user_data_dir(), "dbus-1", "services", nullptr);
  if (g_mkdir_with_parents(directory, 0700) != 0) {
    g_error("Cannot create Operit D-Bus activation directory");
  }
  g_autofree gchar* quoted = g_shell_quote(executable);
  g_autofree gchar* content = g_strdup_printf(
      "[D-BUS Service]\nName=org.operit.PluginSdk\nExec=%s --plugin-sdk\n", quoted);
  g_autofree gchar* path = g_build_filename(directory, "org.operit.PluginSdk.service", nullptr);
  if (!g_file_set_contents(path, content, -1, &error)) {
    g_error("Cannot register Operit D-Bus activation: %s", error->message);
  }
}

/// Publishes a desktop entry that starts the standalone data rescue route.
static void register_data_rescue_desktop_entry() {
  g_autoptr(GError) error = nullptr;
  g_autofree gchar* executable = g_file_read_link("/proc/self/exe", &error);
  if (executable == nullptr) {
    g_error("Cannot resolve Operit data rescue executable: %s", error->message);
  }
  g_autofree gchar* directory = g_build_filename(
      g_get_user_data_dir(), "applications", nullptr);
  if (g_mkdir_with_parents(directory, 0700) != 0) {
    g_error("Cannot create Operit desktop entry directory");
  }
  g_autofree gchar* quoted = quote_desktop_exec_argument(executable);
  g_autofree gchar* content = g_strdup_printf(
      "[Desktop Entry]\nType=Application\nName=Operit2 Data Rescue\nExec=%s --data-rescue\nTerminal=false\nCategories=Utility;\n",
      quoted);
  g_autofree gchar* path = g_build_filename(
      directory, "operit2-data-rescue.desktop", nullptr);
  if (!g_file_set_contents(path, content, -1, &error)) {
    g_error("Cannot register Operit data rescue desktop entry: %s", error->message);
  }
}

/// Checks the runner arguments before any normal activation registration.
static gboolean has_data_rescue_argument(MyApplication* self) {
  for (gchar** argument = self->dart_entrypoint_arguments;
       argument != nullptr && *argument != nullptr; ++argument) {
    if (g_strcmp0(*argument, "--data-rescue") == 0) return TRUE;
  }
  return FALSE;
}

// Called when first Flutter frame received.
static void first_frame_cb(MyApplication* self, FlView* view) {
  gtk_widget_show(gtk_widget_get_toplevel(GTK_WIDGET(view)));
}

// Implements GApplication::activate.
static void my_application_activate(GApplication* application) {
  MyApplication* self = MY_APPLICATION(application);
  GtkWindow* window =
      GTK_WINDOW(gtk_application_window_new(GTK_APPLICATION(application)));

  // Use a header bar when running in GNOME as this is the common style used
  // by applications and is the setup most users will be using (e.g. Ubuntu
  // desktop).
  // If running on X and not using GNOME then just use a traditional title bar
  // in case the window manager does more exotic layout, e.g. tiling.
  // If running on Wayland assume the header bar will work (may need changing
  // if future cases occur).
  gboolean use_header_bar = TRUE;
#ifdef GDK_WINDOWING_X11
  GdkScreen* screen = gtk_window_get_screen(window);
  if (GDK_IS_X11_SCREEN(screen)) {
    const gchar* wm_name = gdk_x11_screen_get_window_manager_name(screen);
    if (g_strcmp0(wm_name, "GNOME Shell") != 0) {
      use_header_bar = FALSE;
    }
  }
#endif
  if (use_header_bar) {
    GtkHeaderBar* header_bar = GTK_HEADER_BAR(gtk_header_bar_new());
    gtk_widget_show(GTK_WIDGET(header_bar));
    gtk_header_bar_set_title(header_bar, "Operit2");
    gtk_header_bar_set_show_close_button(header_bar, TRUE);
    gtk_window_set_titlebar(window, GTK_WIDGET(header_bar));
  } else {
    gtk_window_set_title(window, "Operit2");
  }

  gtk_window_set_default_size(window, 1280, 720);

  g_autoptr(FlDartProject) project = fl_dart_project_new();
  fl_dart_project_set_dart_entrypoint_arguments(
      project, self->dart_entrypoint_arguments);

  FlView* view = fl_view_new(project);
  GdkRGBA background_color;
  // Background defaults to black, override it here if necessary, e.g. #00000000
  // for transparent.
  gdk_rgba_parse(&background_color, "#000000");
  fl_view_set_background_color(view, &background_color);
  gtk_widget_show(GTK_WIDGET(view));
  gtk_container_add(GTK_CONTAINER(window), GTK_WIDGET(view));

  // Show the window when Flutter renders.
  // Requires the view to be realized so we can start rendering.
  g_signal_connect_swapped(view, "first-frame", G_CALLBACK(first_frame_cb),
                           self);
  gtk_widget_realize(GTK_WIDGET(view));

  if (has_data_rescue_argument(self)) {
    register_data_rescue_plugins(FL_PLUGIN_REGISTRY(view));
  } else {
    fl_register_plugins(FL_PLUGIN_REGISTRY(view));
  }
  if (!has_data_rescue_argument(self)) {
    desktop_multi_window_plugin_set_window_created_callback(
      [](FlPluginRegistry* registry) {
        fl_register_plugins(registry);
        register_operit_crash_channel(FL_VIEW(registry));
        register_operit_runtime_channel(FL_VIEW(registry));
        register_webview_theme(FL_VIEW(registry));
      });
    register_operit_crash_channel(view);
    register_webview_theme(view);
  }
  register_operit_runtime_channel(view);

  gtk_widget_grab_focus(GTK_WIDGET(view));
}

// Implements GApplication::local_command_line.
static gboolean my_application_local_command_line(GApplication* application,
                                                  gchar*** arguments,
                                                  int* exit_status) {
  MyApplication* self = MY_APPLICATION(application);
  // Strip out the first argument as it is the binary name.
  self->dart_entrypoint_arguments = g_strdupv(*arguments + 1);

  g_autoptr(GError) error = nullptr;
  if (!g_application_register(application, nullptr, &error)) {
    g_warning("Failed to register: %s", error->message);
    *exit_status = 1;
    return TRUE;
  }

  g_application_activate(application);
  *exit_status = 0;

  return TRUE;
}

// Implements GApplication::startup.
static void my_application_startup(GApplication* application) {
  MyApplication* self = MY_APPLICATION(application);
  if (!has_data_rescue_argument(self)) {
    register_plugin_sdk_activation();
    register_data_rescue_desktop_entry();
  }
  // MyApplication* self = MY_APPLICATION(object);

  // Perform any actions required at application startup.

  G_APPLICATION_CLASS(my_application_parent_class)->startup(application);
}

// Implements GApplication::shutdown.
static void my_application_shutdown(GApplication* application) {
  // MyApplication* self = MY_APPLICATION(object);

  // Perform any actions required at application shutdown.

  G_APPLICATION_CLASS(my_application_parent_class)->shutdown(application);
}

// Implements GObject::dispose.
static void my_application_dispose(GObject* object) {
  MyApplication* self = MY_APPLICATION(object);
  g_clear_pointer(&self->dart_entrypoint_arguments, g_strfreev);
  G_OBJECT_CLASS(my_application_parent_class)->dispose(object);
}

static void my_application_class_init(MyApplicationClass* klass) {
  G_APPLICATION_CLASS(klass)->activate = my_application_activate;
  G_APPLICATION_CLASS(klass)->local_command_line =
      my_application_local_command_line;
  G_APPLICATION_CLASS(klass)->startup = my_application_startup;
  G_APPLICATION_CLASS(klass)->shutdown = my_application_shutdown;
  G_OBJECT_CLASS(klass)->dispose = my_application_dispose;
}

static void my_application_init(MyApplication* self) {}

MyApplication* my_application_new() {
  // Set the program name to the application ID, which helps various systems
  // like GTK and desktop environments map this running application to its
  // corresponding .desktop file. This ensures better integration by allowing
  // the application to be recognized beyond its binary name.
  g_set_prgname(APPLICATION_ID);

  return MY_APPLICATION(g_object_new(my_application_get_type(),
                                     "application-id", APPLICATION_ID, "flags",
                                     G_APPLICATION_NON_UNIQUE, nullptr));
}
