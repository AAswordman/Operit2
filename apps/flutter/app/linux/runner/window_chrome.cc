#include "window_chrome.h"

#include <cstdint>
#include <cstring>

namespace {

struct WindowChrome {
  GWeakRef view;
  GtkCssProvider* provider;
  GdkScreen* screen;
  gchar* style_class;
};

void destroy_window_chrome(gpointer data) {
  auto* chrome = static_cast<WindowChrome*>(data);
  gtk_style_context_remove_provider_for_screen(
      chrome->screen, GTK_STYLE_PROVIDER(chrome->provider));
  g_object_unref(chrome->provider);
  g_object_unref(chrome->screen);
  g_weak_ref_clear(&chrome->view);
  g_free(chrome->style_class);
  g_free(chrome);
}

void handle_window_chrome(FlMethodChannel*, FlMethodCall* call, gpointer data) {
  if (strcmp(fl_method_call_get_name(call), "configure") != 0) {
    fl_method_call_respond_not_implemented(call, nullptr);
    return;
  }
  FlValue* arguments = fl_method_call_get_args(call);
  if (arguments == nullptr || fl_value_get_type(arguments) != FL_VALUE_TYPE_MAP) {
    fl_method_call_respond_error(call, "invalid_arguments",
                                "Window appearance is required", nullptr, nullptr);
    return;
  }
  FlValue* dark = fl_value_lookup_string(arguments, "dark");
  FlValue* background = fl_value_lookup_string(arguments, "backgroundColor");
  FlValue* foreground = fl_value_lookup_string(arguments, "foregroundColor");
  if (dark == nullptr || fl_value_get_type(dark) != FL_VALUE_TYPE_BOOL ||
      background == nullptr || fl_value_get_type(background) != FL_VALUE_TYPE_INT ||
      foreground == nullptr || fl_value_get_type(foreground) != FL_VALUE_TYPE_INT) {
    fl_method_call_respond_error(call, "invalid_arguments",
                                "Window appearance is incomplete", nullptr, nullptr);
    return;
  }

  auto* chrome = static_cast<WindowChrome*>(data);
  g_autoptr(FlView) view = FL_VIEW(g_weak_ref_get(&chrome->view));
  GtkWidget* toplevel = view == nullptr ? nullptr :
      gtk_widget_get_toplevel(GTK_WIDGET(view));
  if (toplevel == nullptr || !GTK_IS_WINDOW(toplevel) ||
      gtk_widget_in_destruction(toplevel)) {
    fl_method_call_respond_error(call, "window_unavailable",
                                "Window is unavailable", nullptr, nullptr);
    return;
  }

  const auto background_argb = static_cast<uint32_t>(fl_value_get_int(background));
  const auto foreground_argb = static_cast<uint32_t>(fl_value_get_int(foreground));
  // Scope the provider to this window, including its inactive caption and
  // native controls. Other engines may have a different theme preference.
  gtk_style_context_add_class(gtk_widget_get_style_context(toplevel),
                              chrome->style_class);
  g_autofree gchar* css = g_strdup_printf(
      "window.%s, window.%s headerbar, window.%s headerbar:backdrop {"
      " background-color: #%06x; background-image: none;"
      " color: #%06x; border-bottom-width: 0; box-shadow: none; }"
      "window.%s headerbar label, window.%s headerbar image,"
      "window.%s headerbar button { color: #%06x; }",
      chrome->style_class, chrome->style_class, chrome->style_class,
      background_argb & 0xffffff, foreground_argb & 0xffffff,
      chrome->style_class, chrome->style_class, chrome->style_class,
      foreground_argb & 0xffffff);
  g_autoptr(GError) error = nullptr;
  if (!gtk_css_provider_load_from_data(chrome->provider, css, -1, &error)) {
    fl_method_call_respond_error(call, "invalid_appearance", error->message,
                                nullptr, nullptr);
    return;
  }

  // The caption is outside FlView, so Flutter must not reserve its height a
  // second time. GTK manages caption visibility when entering full screen.
  g_autoptr(FlValue) inset = fl_value_new_float(0.0);
  fl_method_call_respond_success(call, inset, nullptr);
}

}  // namespace

void prepare_operit_window_chrome(GtkWindow* window) {
  if (gtk_window_get_titlebar(window) != nullptr) return;
  GtkWidget* header = gtk_header_bar_new();
  gtk_header_bar_set_title(GTK_HEADER_BAR(header), "Operit2");
  gtk_header_bar_set_show_close_button(GTK_HEADER_BAR(header), TRUE);
  gtk_widget_show(header);
  gtk_window_set_titlebar(window, header);
}

void register_operit_window_chrome(FlView* view) {
  auto* chrome = g_new0(WindowChrome, 1);
  g_weak_ref_init(&chrome->view, G_OBJECT(view));
  chrome->provider = gtk_css_provider_new();
  chrome->screen = GDK_SCREEN(g_object_ref(gtk_widget_get_screen(GTK_WIDGET(view))));
  chrome->style_class = g_strdup_printf("operit-window-%p", static_cast<void*>(view));
  gtk_style_context_add_provider_for_screen(
      chrome->screen, GTK_STYLE_PROVIDER(chrome->provider),
      GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  g_autoptr(FlMethodChannel) channel = fl_method_channel_new(
      fl_engine_get_binary_messenger(fl_view_get_engine(view)),
      "operit/window_chrome", FL_METHOD_CODEC(codec));
  fl_method_channel_set_method_call_handler(
      channel, handle_window_chrome, chrome, destroy_window_chrome);
}
