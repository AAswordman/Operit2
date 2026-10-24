#include "common/method_channel_utils.h"
#include "webview/webview_internal.h"

#include <libsoup/soup.h>
#include <webkit2/webkit2.h>

#include <cstring>

namespace {

constexpr const gchar *kLocalResourceScheme = "operit-vfs";

struct PendingLocalResource {
  WebKitURISchemeRequest *request;
  FlMethodChannel *channel;
};

void append_request_header(const gchar *name, const gchar *value,
                           gpointer user_data) {
  FlValue *headers = static_cast<FlValue *>(user_data);
  if (name == nullptr || value == nullptr) {
    return;
  }
  fl_value_set_string_take(headers, name, fl_value_new_string(value));
}

void finish_local_resource_status(WebKitURISchemeRequest *request,
                                  guint status_code,
                                  const gchar *reason_phrase) {
  GInputStream *stream = g_memory_input_stream_new();
  WebKitURISchemeResponse *response =
      webkit_uri_scheme_response_new(stream, 0);
  g_object_unref(stream);
  webkit_uri_scheme_response_set_status(response, status_code, reason_phrase);
  webkit_uri_scheme_response_set_content_type(response, "text/plain");
  webkit_uri_scheme_request_finish_with_response(request, response);
  g_object_unref(response);
}

void finish_local_resource_value(WebKitURISchemeRequest *request,
                                 FlValue *value) {
  if (value == nullptr || fl_value_get_type(value) != FL_VALUE_TYPE_MAP) {
    finish_local_resource_status(request, 500, "Resource Error");
    return;
  }

  FlValue *body = fl_value_lookup_string(value, "body");
  const guint8 *bytes = nullptr;
  size_t length = 0;
  if (body != nullptr && fl_value_get_type(body) == FL_VALUE_TYPE_UINT8_LIST) {
    bytes = fl_value_get_uint8_list(body);
    length = fl_value_get_length(body);
  }

  gchar *copy = static_cast<gchar *>(g_malloc(length == 0 ? 1 : length));
  if (length != 0 && bytes != nullptr) {
    memcpy(copy, bytes, length);
  }
  GInputStream *stream =
      g_memory_input_stream_new_from_data(copy, static_cast<gssize>(length),
                                          g_free);
  WebKitURISchemeResponse *response = webkit_uri_scheme_response_new(
      stream, static_cast<goffset>(length));
  g_object_unref(stream);

  const gint status_code = static_cast<gint>(
      map_lookup_int(value, "statusCode", 200));
  const gchar *reason_phrase =
      map_lookup_string(value, "reasonPhrase");
  webkit_uri_scheme_response_set_status(
      response, static_cast<guint>(status_code),
      reason_phrase != nullptr ? reason_phrase : "OK");

  const gchar *mime_type = map_lookup_string(value, "mimeType");
  const gchar *encoding = map_lookup_string(value, "encoding");
  gchar *content_type = g_strdup_printf(
      "%s; charset=%s", mime_type != nullptr ? mime_type : "text/plain",
      encoding != nullptr ? encoding : "utf-8");
  webkit_uri_scheme_response_set_content_type(response, content_type);
  g_free(content_type);

  FlValue *headers = fl_value_lookup_string(value, "headers");
  if (headers != nullptr && fl_value_get_type(headers) == FL_VALUE_TYPE_MAP) {
    SoupMessageHeaders *response_headers =
        soup_message_headers_new(SOUP_MESSAGE_HEADERS_RESPONSE);
    const size_t count = fl_value_get_length(headers);
    for (size_t index = 0; index < count; ++index) {
      const gchar *name =
          fl_value_to_string_or_null(fl_value_get_map_key(headers, index));
      const gchar *item =
          fl_value_to_string_or_null(fl_value_get_map_value(headers, index));
      if (name == nullptr || item == nullptr ||
          g_ascii_strcasecmp(name, "Content-Type") == 0) {
        continue;
      }
      soup_message_headers_append(response_headers, name, item);
    }
    webkit_uri_scheme_response_set_http_headers(response, response_headers);
  }

  webkit_uri_scheme_request_finish_with_response(request, response);
  g_object_unref(response);
}

void local_resource_response_cb(GObject *object, GAsyncResult *result,
                                gpointer user_data) {
  PendingLocalResource *pending =
      static_cast<PendingLocalResource *>(user_data);
  GError *error = nullptr;
  FlMethodResponse *response = fl_method_channel_invoke_method_finish(
      pending->channel, result, &error);
  if (response == nullptr) {
    if (error != nullptr) {
      g_warning("Local resource request failed: %s", error->message);
      g_clear_error(&error);
    }
    finish_local_resource_status(pending->request, 500, "Resource Error");
  } else {
    FlValue *value = fl_method_response_get_result(response, &error);
    if (value == nullptr) {
      if (error != nullptr) {
        g_warning("Local resource request returned an error: %s",
                  error->message);
        g_clear_error(&error);
      }
      finish_local_resource_status(pending->request, 500, "Resource Error");
    } else {
      finish_local_resource_value(pending->request, value);
    }
    g_object_unref(response);
  }
  g_object_unref(pending->request);
  g_object_unref(pending->channel);
  g_free(pending);
}

void local_resource_request_cb(WebKitURISchemeRequest *request,
                               gpointer user_data) {
  WebKitWebView *web_view = webkit_uri_scheme_request_get_web_view(request);
  LinuxWebView *webview =
      web_view == nullptr
          ? nullptr
          : static_cast<LinuxWebView *>(g_object_get_data(
                G_OBJECT(web_view), kLinuxWebViewInstanceKey));
  if (webview == nullptr || webview->plugin == nullptr ||
      !webview->local_resources_enabled ||
      webview->plugin->resource_channel == nullptr) {
    finish_local_resource_status(request, 410, "Gone");
    return;
  }

  const gchar *uri = webkit_uri_scheme_request_get_uri(request);
  const gchar *method = webkit_uri_scheme_request_get_http_method(request);
  const gchar *current_uri = webkit_web_view_get_uri(web_view);

  FlValue *args = fl_value_new_map();
  fl_value_set_string_take(args, "identifier", fl_value_new_int(webview->id));
  fl_value_set_string_take(
      args, "url", fl_value_new_string(uri != nullptr ? uri : ""));
  fl_value_set_string_take(
      args, "method",
      fl_value_new_string(method != nullptr && *method != '\0' ? method
                                                              : "GET"));
  FlValue *headers = fl_value_new_map();
  SoupMessageHeaders *request_headers =
      webkit_uri_scheme_request_get_http_headers(request);
  if (request_headers != nullptr) {
    soup_message_headers_foreach(request_headers, append_request_header,
                                 headers);
  }
  fl_value_set_string_take(args, "headers", headers);
  fl_value_set_string_take(
      args, "isMainFrame",
      fl_value_new_bool(current_uri != nullptr && uri != nullptr &&
                        g_strcmp0(current_uri, uri) == 0));

  PendingLocalResource *pending = g_new0(PendingLocalResource, 1);
  pending->request = static_cast<WebKitURISchemeRequest *>(
      g_object_ref(request));
  pending->channel = static_cast<FlMethodChannel *>(
      g_object_ref(webview->plugin->resource_channel));
  fl_method_channel_invoke_method(pending->channel, "request", args, nullptr,
                                  local_resource_response_cb, pending);
  fl_value_unref(args);
}

}  // namespace

void resource_method_call_cb(FlMethodChannel *channel,
                             FlMethodCall *method_call, gpointer user_data) {
  WebviewAllLinuxPlugin *plugin =
      static_cast<WebviewAllLinuxPlugin *>(user_data);
  const gchar *method = fl_method_call_get_name(method_call);
  if (g_strcmp0(method, "configure") != 0) {
    respond(method_call, error_response("not_implemented",
                                        "Unsupported resource operation"));
    return;
  }
  FlValue *args = fl_method_call_get_args(method_call);
  const gint64 identifier = map_lookup_int(args, "identifier", -1);
  const gboolean enabled = map_lookup_bool(args, "enabled", FALSE);
  LinuxWebView *webview = static_cast<LinuxWebView *>(g_hash_table_lookup(
      plugin->webviews, GINT_TO_POINTER(static_cast<gint>(identifier))));
  if (webview == nullptr) {
    respond(method_call,
            error_response("unknown_view", "WebView no longer exists"));
    return;
  }
  webview->local_resources_enabled = enabled;
  respond(method_call, success_response());
}

void ensure_local_resource_scheme(WebKitWebContext *context) {
  static GHashTable *registered = nullptr;
  if (context == nullptr) {
    return;
  }
  if (registered == nullptr) {
    registered = g_hash_table_new_full(g_direct_hash, g_direct_equal,
                                       g_object_unref, nullptr);
  }
  if (g_hash_table_contains(registered, context)) {
    return;
  }
  webkit_web_context_register_uri_scheme(context, kLocalResourceScheme,
                                         local_resource_request_cb, nullptr,
                                         nullptr);
  WebKitSecurityManager *manager =
      webkit_web_context_get_security_manager(context);
  if (manager != nullptr) {
    webkit_security_manager_register_uri_scheme_as_secure(manager,
                                                          kLocalResourceScheme);
    webkit_security_manager_register_uri_scheme_as_cors_enabled(
        manager, kLocalResourceScheme);
  }
  g_hash_table_add(registered, g_object_ref(context));
}
