#include "plugin/webview_all_linux_plugin_private.h"
#include "common/method_channel_utils.h"
#include "webview/webview_internal.h"

G_DEFINE_TYPE(WebviewAllLinuxPlugin,
              webview_all_linux_plugin,
              g_object_get_type())

static void webview_all_linux_plugin_dispose(GObject* object) {
  WebviewAllLinuxPlugin* self =
      reinterpret_cast<WebviewAllLinuxPlugin*>(object);

  self->disposing = TRUE;
  if (self->webviews != nullptr) {
    g_hash_table_destroy(self->webviews);
    self->webviews = nullptr;
  }
  detach_linux_webview_host(self);
  g_clear_object(&self->root_channel);
  g_clear_object(&self->resource_channel);
  g_clear_object(&self->registrar);

  G_OBJECT_CLASS(webview_all_linux_plugin_parent_class)->dispose(object);
}

static void webview_all_linux_plugin_class_init(
    WebviewAllLinuxPluginClass* klass) {
  G_OBJECT_CLASS(klass)->dispose = webview_all_linux_plugin_dispose;
}

static void webview_all_linux_plugin_init(WebviewAllLinuxPlugin* self) {
  self->next_webview_id = 1;
  self->webviews = g_hash_table_new_full(g_direct_hash, g_direct_equal, nullptr,
                                         destroy_linux_webview);
}

void webview_all_linux_plugin_register_with_registrar(
    FlPluginRegistrar* registrar) {
  WebviewAllLinuxPlugin* plugin = reinterpret_cast<WebviewAllLinuxPlugin*>(
      g_object_new(webview_all_linux_plugin_get_type(), nullptr));

  plugin->registrar = FL_PLUGIN_REGISTRAR(g_object_ref(registrar));
  plugin->root_channel = fl_method_channel_new(
      fl_plugin_registrar_get_messenger(registrar),
      "com.abandoft.webview_all_linux", method_codec());

  fl_method_channel_set_method_call_handler(plugin->root_channel,
                                            root_method_call_cb,
                                            g_object_ref(plugin),
                                            g_object_unref);
  plugin->resource_channel = fl_method_channel_new(
      fl_plugin_registrar_get_messenger(registrar), "operit/webview_resources",
      method_codec());
  fl_method_channel_set_method_call_handler(plugin->resource_channel,
                                            resource_method_call_cb,
                                            g_object_ref(plugin),
                                            g_object_unref);
  g_object_unref(plugin);
}
