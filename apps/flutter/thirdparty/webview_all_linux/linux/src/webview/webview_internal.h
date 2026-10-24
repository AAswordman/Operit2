#ifndef WEBVIEW_ALL_LINUX_WEBVIEW_INTERNAL_H_
#define WEBVIEW_ALL_LINUX_WEBVIEW_INTERNAL_H_

#include "plugin/webview_all_linux_plugin_private.h"

// Dispatches pointer input accepted by Flutter's platform-view hit test.
void dispatch_linux_webview_pointer(LinuxWebView* webview, FlValue* args,
                                    FlMethodCall* method_call);

void send_event(LinuxWebView *webview, FlValue *event);
void update_history(LinuxWebView *webview);
void emit_url_change(LinuxWebView *webview);
void emit_title_change(LinuxWebView *webview);
void emit_load_error(LinuxWebView *webview, GError *error,
                     const gchar *failing_url);
void use_navigation_decision(LinuxWebView *webview,
                             WebKitPolicyDecision *decision);
void evaluate_javascript(WebKitWebView *web_view, const gchar *script,
                         FlMethodCall *method_call);
gchar *build_scrollbar_style_script(LinuxWebView *webview);
gchar *build_overscroll_style_script(LinuxWebView *webview);
void rebuild_user_scripts(LinuxWebView *webview);
void console_message_received_cb(WebKitUserContentManager *manager,
                                 WebKitJavascriptResult *result,
                                 gpointer user_data);
void scroll_message_received_cb(WebKitUserContentManager *manager,
                                WebKitJavascriptResult *result,
                                gpointer user_data);
void async_javascript_message_received_cb(WebKitUserContentManager *manager,
                                          WebKitJavascriptResult *result,
                                          gpointer user_data);
void javascript_channel_message_received_cb(WebKitUserContentManager *manager,
                                            WebKitJavascriptResult *result,
                                            gpointer user_data);
void destroy_js_channel_handler_data(gpointer data, GClosure *closure);
void instance_method_call_cb(FlMethodChannel *channel,
                             FlMethodCall *method_call, gpointer user_data);
void resource_method_call_cb(FlMethodChannel *channel,
                             FlMethodCall *method_call, gpointer user_data);
void ensure_local_resource_scheme(WebKitWebContext *context);

#endif // WEBVIEW_ALL_LINUX_WEBVIEW_INTERNAL_H_
