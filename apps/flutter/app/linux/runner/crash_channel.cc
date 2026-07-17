#include "crash_channel.h"

namespace {

FlMethodChannel* g_operit_crash_channel = nullptr;

void present_crash_screen(FlMethodCall* method_call) {
  FlValue* arguments = fl_method_call_get_args(method_call);
  if (arguments == nullptr || fl_value_get_type(arguments) != FL_VALUE_TYPE_MAP) {
    fl_method_call_respond_error(method_call, "INVALID_ARGS",
                                 "present requires crash details", nullptr,
                                 nullptr);
    return;
  }
  FlValue* details = fl_value_lookup_string(arguments, "details");
  if (details == nullptr || fl_value_get_type(details) != FL_VALUE_TYPE_STRING) {
    fl_method_call_respond_error(method_call, "INVALID_ARGS",
                                 "present requires crash details", nullptr,
                                 nullptr);
    return;
  }
  GtkWidget* dialog = gtk_message_dialog_new(
      nullptr, GTK_DIALOG_MODAL, GTK_MESSAGE_ERROR, GTK_BUTTONS_CLOSE,
      "Operit2 has stopped");
  gtk_message_dialog_format_secondary_text(
      GTK_MESSAGE_DIALOG(dialog), "%s", fl_value_get_string(details));
  gtk_dialog_run(GTK_DIALOG(dialog));
  gtk_widget_destroy(dialog);
  fl_method_call_respond_success(method_call, nullptr, nullptr);
}

void crash_method_call_cb(FlMethodChannel*, FlMethodCall* method_call,
                          gpointer) {
  if (g_strcmp0(fl_method_call_get_name(method_call), "present") != 0) {
    fl_method_call_respond_not_implemented(method_call, nullptr);
    return;
  }
  present_crash_screen(method_call);
}

}  // namespace

void register_operit_crash_channel(FlView* view) {
  if (g_operit_crash_channel != nullptr) {
    fl_method_channel_set_method_call_handler(g_operit_crash_channel, nullptr,
                                              nullptr, nullptr);
    g_clear_object(&g_operit_crash_channel);
  }
  FlBinaryMessenger* messenger = fl_engine_get_binary_messenger(fl_view_get_engine(view));
  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  g_operit_crash_channel = fl_method_channel_new(
      messenger, "operit/crash", FL_METHOD_CODEC(codec));
  fl_method_channel_set_method_call_handler(g_operit_crash_channel,
                                             crash_method_call_cb, nullptr,
                                             nullptr);
}
