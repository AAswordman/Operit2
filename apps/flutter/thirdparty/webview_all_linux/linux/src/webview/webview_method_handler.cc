#include "common/method_channel_utils.h"
#include "webview/download_policy.h"
#include "webview/webview_internal.h"

#include <libsoup/soup.h>

#include <algorithm>
#include <cmath>
#include <cstring>
#include <limits>

namespace {

void remove_user_script_from_order(LinuxWebView *webview,
                                   const gchar *identifier) {
  for (guint index = 0; index < webview->user_script_order->len; ++index) {
    const gchar *existing = static_cast<const gchar *>(
        g_ptr_array_index(webview->user_script_order, index));
    if (g_strcmp0(existing, identifier) == 0) {
      g_ptr_array_remove_index(webview->user_script_order, index);
      return;
    }
  }
}

typedef struct {
  WebKitWebView *web_view;
  FlMethodCall *method_call;
  SoupSession *session;
  SoupMessage *message;
  gchar *url;
} PendingSoupLoadRequest;

double clamp_double(double value, double minimum, double maximum) {
  return std::max(minimum, std::min(value, maximum));
}

gint floor_to_gint(double value) {
  if (!std::isfinite(value)) {
    return 0;
  }
  const double minimum = static_cast<double>(std::numeric_limits<gint>::min());
  const double maximum = static_cast<double>(std::numeric_limits<gint>::max());
  return static_cast<gint>(std::floor(clamp_double(value, minimum, maximum)));
}

gint frame_extent(double origin, double extent, gint rounded_origin) {
  if (!std::isfinite(origin) || !std::isfinite(extent) || extent <= 0) {
    return 0;
  }
  const double maximum = static_cast<double>(std::numeric_limits<gint>::max());
  const double rounded_edge =
      std::ceil(clamp_double(origin + extent, -maximum, maximum));
  const double rounded_extent =
      rounded_edge - static_cast<double>(rounded_origin);
  return static_cast<gint>(clamp_double(rounded_extent, 0.0, maximum));
}

gint clamp_int64_to_gint(gint64 value) {
  return static_cast<gint>(CLAMP(value, static_cast<gint64>(G_MININT),
                                 static_cast<gint64>(G_MAXINT)));
}

GtkWidget *ensure_clip_container(LinuxWebView *webview, GtkOverlay *overlay) {
  if (webview->clip_container != nullptr) {
    return webview->clip_container;
  }

  GtkWidget *container = gtk_layout_new(nullptr, nullptr);
  gtk_widget_set_halign(container, GTK_ALIGN_START);
  gtk_widget_set_valign(container, GTK_ALIGN_START);
  gtk_widget_set_hexpand(container, FALSE);
  gtk_widget_set_vexpand(container, FALSE);
  g_object_set_data(G_OBJECT(container), kLinuxWebViewInstanceKey, webview);
  webview->clip_container = container;
  g_object_add_weak_pointer(
      G_OBJECT(container),
      reinterpret_cast<gpointer *>(&webview->clip_container));

  gtk_layout_put(GTK_LAYOUT(container), GTK_WIDGET(webview->web_view), 0, 0);
  fl_view_register_native_view(FL_VIEW(webview->plugin->flutter_view),
                               webview->id, container);
  return container;
}

void append_header_to_map(const char *name, const char *value,
                          gpointer user_data) {
  FlValue *headers = static_cast<FlValue *>(user_data);
  if (name == nullptr || value == nullptr) {
    return;
  }
  fl_value_set_string_take(headers, name, fl_value_new_string(value));
}

void append_headers_from_value(SoupMessageHeaders *headers,
                               FlValue *header_value) {
  if (headers == nullptr || header_value == nullptr ||
      fl_value_get_type(header_value) != FL_VALUE_TYPE_MAP) {
    return;
  }

  const size_t header_count = fl_value_get_length(header_value);
  for (size_t i = 0; i < header_count; ++i) {
    const gchar *key =
        fl_value_to_string_or_null(fl_value_get_map_key(header_value, i));
    const gchar *value =
        fl_value_to_string_or_null(fl_value_get_map_value(header_value, i));
    if (key != nullptr && value != nullptr) {
      soup_message_headers_append(headers, key, value);
    }
  }
}

GBytes *body_bytes_from_value(FlValue *body_value) {
  if (body_value == nullptr ||
      fl_value_get_type(body_value) != FL_VALUE_TYPE_UINT8_LIST) {
    return nullptr;
  }

  return g_bytes_new(fl_value_get_uint8_list(body_value),
                     fl_value_get_length(body_value));
}

guint map_lookup_guint(FlValue *map, const gchar *key, guint fallback) {
  const gint64 value = map_lookup_int(map, key, fallback);
  if (value < 0) {
    return 0;
  }
  if (value > static_cast<gint64>(G_MAXUINT)) {
    return G_MAXUINT;
  }
  return static_cast<guint>(value);
}

void apply_webkit_settings(LinuxWebView *webview, FlValue *args) {
  WebKitSettings *settings = webkit_web_view_get_settings(webview->web_view);
  FlValue *value = nullptr;

  value = map_lookup(args, "downloadsEnabled");
  if (value != nullptr && fl_value_get_type(value) == FL_VALUE_TYPE_BOOL) {
    webview->download_policy->SetEnabled(fl_value_get_bool(value));
  }

  value = map_lookup(args, "developerExtrasEnabled");
  if (value != nullptr && fl_value_get_type(value) == FL_VALUE_TYPE_BOOL) {
    webkit_settings_set_enable_developer_extras(settings,
                                                fl_value_get_bool(value));
  }

  value = map_lookup(args, "javascriptCanOpenWindowsAutomatically");
  if (value != nullptr && fl_value_get_type(value) == FL_VALUE_TYPE_BOOL) {
    webkit_settings_set_javascript_can_open_windows_automatically(
        settings, fl_value_get_bool(value));
  }

  value = map_lookup(args, "mediaPlaybackRequiresUserGesture");
  if (value != nullptr && fl_value_get_type(value) == FL_VALUE_TYPE_BOOL) {
    webview->media_playback_requires_user_gesture =
        fl_value_get_bool(value) ? 1 : 0;
    webkit_settings_set_media_playback_requires_user_gesture(
        settings, fl_value_get_bool(value));
  }

  value = map_lookup(args, "mediaPlaybackAllowsInline");
  if (value != nullptr && fl_value_get_type(value) == FL_VALUE_TYPE_BOOL) {
    webkit_settings_set_media_playback_allows_inline(settings,
                                                     fl_value_get_bool(value));
  }

  value = map_lookup(args, "pageCacheEnabled");
  if (value != nullptr && fl_value_get_type(value) == FL_VALUE_TYPE_BOOL) {
    webkit_settings_set_enable_page_cache(settings, fl_value_get_bool(value));
  }

  value = map_lookup(args, "allowFileAccessFromFileUrls");
  if (value != nullptr && fl_value_get_type(value) == FL_VALUE_TYPE_BOOL) {
    webkit_settings_set_allow_file_access_from_file_urls(
        settings, fl_value_get_bool(value));
  }

  value = map_lookup(args, "allowUniversalAccessFromFileUrls");
  if (value != nullptr && fl_value_get_type(value) == FL_VALUE_TYPE_BOOL) {
    webkit_settings_set_allow_universal_access_from_file_urls(
        settings, fl_value_get_bool(value));
  }

  value = map_lookup(args, "zoomTextOnly");
  if (value != nullptr && fl_value_get_type(value) == FL_VALUE_TYPE_BOOL) {
    webkit_settings_set_zoom_text_only(settings, fl_value_get_bool(value));
  }

  value = map_lookup(args, "defaultFontSize");
  if (value != nullptr && fl_value_get_type(value) == FL_VALUE_TYPE_INT) {
    webkit_settings_set_default_font_size(
        settings, map_lookup_guint(args, "defaultFontSize", 16));
  }

  value = map_lookup(args, "defaultMonospaceFontSize");
  if (value != nullptr && fl_value_get_type(value) == FL_VALUE_TYPE_INT) {
    webkit_settings_set_default_monospace_font_size(settings,
                                                    map_lookup_guint(
                                                        args,
                                                        "defaultMonospaceFontSize",
                                                        13));
  }

  value = map_lookup(args, "minimumFontSize");
  if (value != nullptr && fl_value_get_type(value) == FL_VALUE_TYPE_INT) {
    webkit_settings_set_minimum_font_size(
        settings, map_lookup_guint(args, "minimumFontSize", 0));
  }

  value = map_lookup(args, "zoomFactor");
  if (value != nullptr) {
    webkit_web_view_set_zoom_level(
        webview->web_view, map_lookup_double(args, "zoomFactor", 1.0));
  }
}

void destroy_pending_soup_load_request(PendingSoupLoadRequest *pending) {
  if (pending == nullptr) {
    return;
  }
  g_clear_object(&pending->web_view);
  g_clear_object(&pending->method_call);
  g_clear_object(&pending->session);
  g_clear_object(&pending->message);
  g_free(pending->url);
  g_free(pending);
}

void clear_local_storage_finished_cb(GObject *object, GAsyncResult *result,
                                     gpointer user_data) {
  FlMethodCall *method_call = FL_METHOD_CALL(user_data);
  GError *error = nullptr;
  webkit_website_data_manager_clear_finish(WEBKIT_WEBSITE_DATA_MANAGER(object),
                                           result, &error);
  if (error != nullptr) {
    respond(method_call, error_response("storage_error", error->message));
    g_clear_error(&error);
  } else {
    respond(method_call, success_response());
  }
  g_object_unref(method_call);
}

void emit_soup_http_error(LinuxWebView *webview, SoupMessage *message,
                          const gchar *url) {
  const SoupStatus status = soup_message_get_status(message);
  if (status < 400) {
    return;
  }

  FlValue *event = make_event("httpError");
  fl_value_set_string_take(event, "url",
                           fl_value_new_string(url != nullptr ? url : ""));
  fl_value_set_string_take(event, "statusCode",
                           fl_value_new_int(static_cast<gint>(status)));

  const gchar *method = soup_message_get_method(message);
  fl_value_set_string_take(
      event, "method", fl_value_new_string(method != nullptr ? method : ""));
  fl_value_set_string_take(event, "isForMainFrame", fl_value_new_bool(TRUE));

  FlValue *request_headers = fl_value_new_map();
  SoupMessageHeaders *raw_request_headers =
      soup_message_get_request_headers(message);
  if (raw_request_headers != nullptr) {
    soup_message_headers_foreach(raw_request_headers, append_header_to_map,
                                 request_headers);
  }
  fl_value_set_string_take(event, "requestHeaders", request_headers);

  FlValue *headers = fl_value_new_map();
  SoupMessageHeaders *response_headers =
      soup_message_get_response_headers(message);
  if (response_headers != nullptr) {
    const gchar *mime_type =
        soup_message_headers_get_content_type(response_headers, nullptr);
    if (mime_type != nullptr) {
      fl_value_set_string_take(event, "mimeType",
                               fl_value_new_string(mime_type));
    }
    soup_message_headers_foreach(response_headers, append_header_to_map,
                                 headers);
  }
  fl_value_set_string_take(event, "headers", headers);
  send_event(webview, event);
}

void soup_load_request_cb(GObject *source_object, GAsyncResult *result,
                          gpointer user_data) {
  PendingSoupLoadRequest *pending =
      static_cast<PendingSoupLoadRequest *>(user_data);
  GError *error = nullptr;
  GBytes *bytes =
      soup_session_send_and_read_finish(pending->session, result, &error);

  if (error != nullptr) {
    respond(pending->method_call,
            error_response("load_request_error", error->message));
    g_clear_error(&error);
    destroy_pending_soup_load_request(pending);
    return;
  }
  if (bytes == nullptr) {
    respond(pending->method_call,
            error_response("load_request_error", "No response body returned."));
    destroy_pending_soup_load_request(pending);
    return;
  }

  LinuxWebView *webview = static_cast<LinuxWebView *>(
      g_object_get_data(G_OBJECT(pending->web_view), kLinuxWebViewInstanceKey));
  if (webview != nullptr) {
    emit_soup_http_error(webview, pending->message, pending->url);

    SoupMessageHeaders *response_headers =
        soup_message_get_response_headers(pending->message);
    const gchar *mime_type =
        response_headers == nullptr
            ? nullptr
            : soup_message_headers_get_content_type(response_headers, nullptr);
    webkit_web_view_load_bytes(
        pending->web_view, bytes,
        mime_type != nullptr ? mime_type : "application/octet-stream", nullptr,
        pending->url);
  }

  g_bytes_unref(bytes);
  respond(pending->method_call, success_response());
  destroy_pending_soup_load_request(pending);
}

void load_request_with_soup(WebKitWebView *web_view, FlMethodCall *method_call,
                            const gchar *url, const gchar *method,
                            FlValue *headers, FlValue *body_value) {
  gchar *normalized_method =
      g_ascii_strup(method != nullptr ? method : "GET", -1);
  SoupMessage *message = soup_message_new(normalized_method, url);
  g_free(normalized_method);
  if (message == nullptr) {
    respond(method_call,
            error_response("load_request_error", "Failed to create request."));
    return;
  }

  SoupMessageHeaders *request_headers =
      soup_message_get_request_headers(message);
  append_headers_from_value(request_headers, headers);

  GBytes *body = body_bytes_from_value(body_value);
  if (body != nullptr) {
    const gchar *content_type =
        request_headers == nullptr
            ? nullptr
            : soup_message_headers_get_content_type(request_headers, nullptr);
    soup_message_set_request_body_from_bytes(
        message,
        content_type != nullptr ? content_type : "application/octet-stream",
        body);
    g_bytes_unref(body);
  }

  PendingSoupLoadRequest *pending = g_new0(PendingSoupLoadRequest, 1);
  pending->web_view = WEBKIT_WEB_VIEW(g_object_ref(web_view));
  pending->method_call = FL_METHOD_CALL(g_object_ref(method_call));
  pending->session = soup_session_new();
  pending->message = SOUP_MESSAGE(g_object_ref(message));
  pending->url = g_strdup(url);

  soup_session_send_and_read_async(pending->session, message,
                                   G_PRIORITY_DEFAULT, nullptr,
                                   soup_load_request_cb, pending);
  g_object_unref(message);
}

} // namespace

void instance_method_call_cb(FlMethodChannel *channel,
                             FlMethodCall *method_call, gpointer user_data) {
  LinuxWebView *webview = static_cast<LinuxWebView *>(user_data);
  const gchar *method = fl_method_call_get_name(method_call);
  FlValue *args = fl_method_call_get_args(method_call);

  if (strcmp(method, "dispatchPointerEvent") == 0) {
    dispatch_linux_webview_pointer(webview, args, method_call);
    return;
  }
  if (strcmp(method, "releaseFocus") == 0) {
    release_linux_webview_focus(webview);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setFrame") == 0) {
    GtkWidget *widget = GTK_WIDGET(webview->web_view);
    const gint64 frame_sequence =
        map_lookup_int(args, "sequence", webview->frame_sequence + 1);
    if (frame_sequence <= webview->frame_sequence) {
      respond(method_call, success_response());
      return;
    }
    if (webview->clip_container == nullptr) {
      GtkOverlay *overlay = ensure_overlay(webview->plugin);
      if (overlay == nullptr) {
        respond(method_call,
                error_response("attachment_error",
                               "Unable to attach the Linux WebView."));
        return;
      }
      ensure_clip_container(webview, overlay);
    }
    webview->frame_sequence = frame_sequence;
    const double x = map_lookup_double(args, "x", 0);
    const double y = map_lookup_double(args, "y", 0);
    const double width = map_lookup_double(args, "width", 0);
    const double height = map_lookup_double(args, "height", 0);
    const double clip_x = map_lookup_double(args, "clipX", x);
    const double clip_y = map_lookup_double(args, "clipY", y);
    const double clip_width = map_lookup_double(args, "clipWidth", width);
    const double clip_height = map_lookup_double(args, "clipHeight", height);
    webview->frame_x = floor_to_gint(x);
    webview->frame_y = floor_to_gint(y);
    webview->frame_width = frame_extent(x, width, webview->frame_x);
    webview->frame_height = frame_extent(y, height, webview->frame_y);
    const gint requested_clip_x = floor_to_gint(clip_x);
    const gint requested_clip_y = floor_to_gint(clip_y);
    const gint requested_clip_width =
        frame_extent(clip_x, clip_width, requested_clip_x);
    const gint requested_clip_height =
        frame_extent(clip_y, clip_height, requested_clip_y);
    const gint64 clip_left =
        MAX(static_cast<gint64>(webview->frame_x),
            static_cast<gint64>(requested_clip_x));
    const gint64 clip_top =
        MAX(static_cast<gint64>(webview->frame_y),
            static_cast<gint64>(requested_clip_y));
    const gint64 clip_right =
        MIN(static_cast<gint64>(webview->frame_x) + webview->frame_width,
            static_cast<gint64>(requested_clip_x) + requested_clip_width);
    const gint64 clip_bottom =
        MIN(static_cast<gint64>(webview->frame_y) + webview->frame_height,
            static_cast<gint64>(requested_clip_y) + requested_clip_height);
    webview->clip_x = clamp_int64_to_gint(clip_left);
    webview->clip_y = clamp_int64_to_gint(clip_top);
    webview->clip_width = clip_right > clip_left
                              ? clamp_int64_to_gint(clip_right - clip_left)
                              : 0;
    webview->clip_height = clip_bottom > clip_top
                               ? clamp_int64_to_gint(clip_bottom - clip_top)
                               : 0;
    webview->visible = map_lookup_bool(args, "visible", TRUE) &&
                       webview->frame_width > 0 && webview->frame_height > 0 &&
                       webview->clip_width > 0 && webview->clip_height > 0;
    gtk_widget_set_halign(widget, GTK_ALIGN_START);
    gtk_widget_set_valign(widget, GTK_ALIGN_START);
    gtk_widget_set_size_request(widget, webview->frame_width,
                                webview->frame_height);
    GtkWidget *clip_container = webview->clip_container;
    // Keep WebKit at the full Flutter widget size and move it inside a smaller
    // GtkLayout viewport. Resizing WebKit to the clip would reflow the page.
    gtk_widget_set_size_request(clip_container, webview->clip_width,
                                webview->clip_height);
    gtk_layout_set_size(GTK_LAYOUT(clip_container), webview->clip_width,
                        webview->clip_height);
    gtk_layout_move(
        GTK_LAYOUT(clip_container), widget,
        clamp_int64_to_gint(static_cast<gint64>(webview->frame_x) -
                            webview->clip_x),
        clamp_int64_to_gint(static_cast<gint64>(webview->frame_y) -
                            webview->clip_y));
    if (webview->plugin->overlay != nullptr) {
      gtk_widget_queue_resize(GTK_WIDGET(webview->plugin->overlay));
    }
    if (webview->visible) {
      gtk_widget_show(widget);
      gtk_widget_show(clip_container);
    } else {
      release_linux_webview_focus(webview);
      gtk_widget_hide(clip_container);
    }
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "applySettings") == 0) {
    apply_webkit_settings(webview, args);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "loadFile") == 0) {
    const gchar *file_path = map_lookup_string(args, "path");
    GError *error = nullptr;
    gchar *uri = g_filename_to_uri(file_path, nullptr, &error);
    if (error != nullptr) {
      respond(method_call, error_response("load_file_error", error->message));
      g_clear_error(&error);
      g_free(uri);
      return;
    }
    webkit_web_view_load_uri(webview->web_view, uri);
    g_free(uri);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "loadHtmlString") == 0) {
    webkit_web_view_load_html(webview->web_view,
                              map_lookup_string(args, "html"),
                              map_lookup_string(args, "baseUrl"));
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "loadRequest") == 0) {
    const gchar *url = map_lookup_string(args, "url");
    const gchar *request_method = map_lookup_string(args, "method");
    FlValue *headers = map_lookup(args, "headers");
    FlValue *body = map_lookup(args, "body");

    const bool has_body = body != nullptr &&
                          fl_value_get_type(body) == FL_VALUE_TYPE_UINT8_LIST &&
                          fl_value_get_length(body) > 0;
    if (g_ascii_strcasecmp(request_method != nullptr ? request_method : "get",
                           "get") != 0 ||
        has_body) {
      load_request_with_soup(webview->web_view, method_call, url,
                             request_method != nullptr ? request_method : "GET",
                             headers, body);
      return;
    }

    WebKitURIRequest *request = webkit_uri_request_new(url);
    append_headers_from_value(webkit_uri_request_get_http_headers(request),
                              headers);
    webkit_web_view_load_request(webview->web_view, request);
    g_object_unref(request);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "currentUrl") == 0) {
    const gchar *uri = webkit_web_view_get_uri(webview->web_view);
    respond(method_call,
            success_response(uri != nullptr ? fl_value_new_string(uri)
                                            : fl_value_new_null()));
    return;
  }

  if (strcmp(method, "canGoBack") == 0) {
    respond(method_call, success_response(fl_value_new_bool(
                             webkit_web_view_can_go_back(webview->web_view))));
    return;
  }

  if (strcmp(method, "canGoForward") == 0) {
    respond(method_call,
            success_response(fl_value_new_bool(
                webkit_web_view_can_go_forward(webview->web_view))));
    return;
  }

  if (strcmp(method, "goBack") == 0) {
    webkit_web_view_go_back(webview->web_view);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "goForward") == 0) {
    webkit_web_view_go_forward(webview->web_view);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "reload") == 0) {
    webkit_web_view_reload(webview->web_view);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "clearCache") == 0) {
    webkit_web_context_clear_cache(
        webkit_web_view_get_context(webview->web_view));
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "clearLocalStorage") == 0) {
    WebKitWebContext *context = webkit_web_view_get_context(webview->web_view);
    WebKitWebsiteDataManager *website_data_manager =
        webkit_web_context_get_website_data_manager(context);
    g_object_ref(method_call);
    webkit_website_data_manager_clear(
        website_data_manager,
        static_cast<WebKitWebsiteDataTypes>(
            WEBKIT_WEBSITE_DATA_LOCAL_STORAGE |
            WEBKIT_WEBSITE_DATA_SESSION_STORAGE),
        0, nullptr, clear_local_storage_finished_cb, method_call);
    return;
  }

  if (strcmp(method, "runJavaScript") == 0 ||
      strcmp(method, "runJavaScriptReturningResult") == 0) {
    evaluate_javascript(webview->web_view, map_lookup_string(args, "script"),
                        method_call);
    return;
  }

  if (strcmp(method, "addJavaScriptChannel") == 0) {
    const gchar *name = map_lookup_string(args, "name");
    if (!webkit_user_content_manager_register_script_message_handler(
            webview->content_manager, name)) {
      respond(method_call, error_response("channel_error",
                                          "Failed to register JS channel."));
      return;
    }
    gchar *signal_name = g_strdup_printf("script-message-received::%s", name);
    JavaScriptChannelHandlerData *data =
        g_new0(JavaScriptChannelHandlerData, 1);
    data->webview = webview;
    data->name = g_strdup(name);
    guint signal_id = g_signal_connect_data(
        webview->content_manager, signal_name,
        G_CALLBACK(javascript_channel_message_received_cb), data,
        destroy_js_channel_handler_data, GConnectFlags(0));
    g_free(signal_name);
    g_hash_table_insert(webview->js_channels, g_strdup(name),
                        GINT_TO_POINTER(1));
    g_hash_table_insert(webview->js_channel_signal_ids, g_strdup(name),
                        GUINT_TO_POINTER(signal_id));
    rebuild_user_scripts(webview);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "removeJavaScriptChannel") == 0) {
    const gchar *name = map_lookup_string(args, "name");
    gpointer signal_id_ptr =
        g_hash_table_lookup(webview->js_channel_signal_ids, name);
    if (signal_id_ptr != nullptr) {
      g_signal_handler_disconnect(webview->content_manager,
                                  GPOINTER_TO_UINT(signal_id_ptr));
      g_hash_table_remove(webview->js_channel_signal_ids, name);
    }
    webkit_user_content_manager_unregister_script_message_handler(
        webview->content_manager, name);
    g_hash_table_remove(webview->js_channels, name);
    rebuild_user_scripts(webview);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "addUserScript") == 0) {
    const gchar *identifier = map_lookup_string(args, "identifier");
    const gchar *source = map_lookup_string(args, "source");
    if (identifier == nullptr || *identifier == '\0' || source == nullptr) {
      respond(method_call,
              error_response("invalid_user_script",
                             "A script identifier and source are required."));
      return;
    }
    WebKitUserScript *script = webkit_user_script_new(
        source,
        map_lookup_bool(args, "mainFrameOnly", TRUE)
            ? WEBKIT_USER_CONTENT_INJECT_TOP_FRAME
            : WEBKIT_USER_CONTENT_INJECT_ALL_FRAMES,
        WEBKIT_USER_SCRIPT_INJECT_AT_DOCUMENT_START, nullptr, nullptr);
    remove_user_script_from_order(webview, identifier);
    g_hash_table_insert(webview->user_scripts, g_strdup(identifier), script);
    g_ptr_array_add(webview->user_script_order, g_strdup(identifier));
    rebuild_user_scripts(webview);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "removeUserScript") == 0) {
    const gchar *identifier = map_lookup_string(args, "identifier");
    if (identifier != nullptr &&
        g_hash_table_remove(webview->user_scripts, identifier)) {
      remove_user_script_from_order(webview, identifier);
      rebuild_user_scripts(webview);
    }
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "removeAllUserScripts") == 0) {
    if (g_hash_table_size(webview->user_scripts) > 0) {
      g_hash_table_remove_all(webview->user_scripts);
      g_ptr_array_set_size(webview->user_script_order, 0);
      rebuild_user_scripts(webview);
    }
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "getTitle") == 0) {
    const gchar *title = webkit_web_view_get_title(webview->web_view);
    respond(method_call,
            success_response(title != nullptr ? fl_value_new_string(title)
                                              : fl_value_new_null()));
    return;
  }

  if (strcmp(method, "scrollTo") == 0) {
    gchar *script = g_strdup_printf(
        "window.scrollTo(%" G_GINT64_FORMAT ", %" G_GINT64_FORMAT ");",
        map_lookup_int(args, "x", 0), map_lookup_int(args, "y", 0));
    evaluate_javascript(webview->web_view, script, method_call);
    g_free(script);
    return;
  }

  if (strcmp(method, "scrollBy") == 0) {
    gchar *script = g_strdup_printf(
        "window.scrollBy(%" G_GINT64_FORMAT ", %" G_GINT64_FORMAT ");",
        map_lookup_int(args, "x", 0), map_lookup_int(args, "y", 0));
    evaluate_javascript(webview->web_view, script, method_call);
    g_free(script);
    return;
  }

  if (strcmp(method, "getScrollPosition") == 0) {
    FlValue *result = fl_value_new_map();
    fl_value_set_string_take(result, "x",
                             fl_value_new_float(webview->last_scroll_x));
    fl_value_set_string_take(result, "y",
                             fl_value_new_float(webview->last_scroll_y));
    respond(method_call, success_response(result));
    return;
  }

  if (strcmp(method, "setVerticalScrollBarEnabled") == 0) {
    webview->vertical_scrollbar_enabled =
        map_lookup_bool(args, "enabled", TRUE);
    rebuild_user_scripts(webview);
    gchar *script = build_scrollbar_style_script(webview);
    evaluate_javascript(webview->web_view, script, method_call);
    g_free(script);
    return;
  }

  if (strcmp(method, "setHorizontalScrollBarEnabled") == 0) {
    webview->horizontal_scrollbar_enabled =
        map_lookup_bool(args, "enabled", TRUE);
    rebuild_user_scripts(webview);
    gchar *script = build_scrollbar_style_script(webview);
    evaluate_javascript(webview->web_view, script, method_call);
    g_free(script);
    return;
  }

  if (strcmp(method, "enableZoom") == 0) {
    webview->zoom_enabled = map_lookup_bool(args, "enabled", TRUE);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setBackgroundColor") == 0) {
    GdkRGBA color = {
        map_lookup_double(args, "r", 1.0), map_lookup_double(args, "g", 1.0),
        map_lookup_double(args, "b", 1.0), map_lookup_double(args, "a", 1.0)};
    webkit_web_view_set_background_color(webview->web_view, &color);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setJavaScriptMode") == 0) {
    WebKitSettings *settings = webkit_web_view_get_settings(webview->web_view);
    webkit_settings_set_enable_javascript(
        settings, map_lookup_bool(args, "enabled", TRUE));
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setDeveloperExtrasEnabled") == 0) {
    WebKitSettings *settings = webkit_web_view_get_settings(webview->web_view);
    webkit_settings_set_enable_developer_extras(
        settings, map_lookup_bool(args, "enabled", FALSE));
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setDownloadsEnabled") == 0) {
    FlValue *enabled = map_lookup(args, "enabled");
    if (enabled == nullptr || fl_value_get_type(enabled) != FL_VALUE_TYPE_BOOL) {
      respond(method_call, error_response("invalid_arguments",
                                         "enabled must be a boolean"));
      return;
    }
    webview->download_policy->SetEnabled(fl_value_get_bool(enabled));
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "openDevTools") == 0) {
    WebKitSettings *settings = webkit_web_view_get_settings(webview->web_view);
    webkit_settings_set_enable_developer_extras(settings, TRUE);
    webkit_web_inspector_show(webkit_web_view_get_inspector(webview->web_view));
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setJavaScriptCanOpenWindowsAutomatically") == 0) {
    WebKitSettings *settings = webkit_web_view_get_settings(webview->web_view);
    webkit_settings_set_javascript_can_open_windows_automatically(
        settings, map_lookup_bool(args, "enabled", FALSE));
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setMediaPlaybackRequiresUserGesture") == 0) {
    WebKitSettings *settings = webkit_web_view_get_settings(webview->web_view);
    const gboolean require = map_lookup_bool(args, "require", FALSE);
    webview->media_playback_requires_user_gesture = require ? 1 : 0;
    webkit_settings_set_media_playback_requires_user_gesture(settings, require);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setMediaPlaybackAllowsInline") == 0) {
    WebKitSettings *settings = webkit_web_view_get_settings(webview->web_view);
    webkit_settings_set_media_playback_allows_inline(
        settings, map_lookup_bool(args, "allow", TRUE));
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setPageCacheEnabled") == 0) {
    WebKitSettings *settings = webkit_web_view_get_settings(webview->web_view);
    webkit_settings_set_enable_page_cache(
        settings, map_lookup_bool(args, "enabled", TRUE));
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setAllowFileAccessFromFileUrls") == 0) {
    WebKitSettings *settings = webkit_web_view_get_settings(webview->web_view);
    webkit_settings_set_allow_file_access_from_file_urls(
        settings, map_lookup_bool(args, "allow", FALSE));
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setAllowUniversalAccessFromFileUrls") == 0) {
    WebKitSettings *settings = webkit_web_view_get_settings(webview->web_view);
    webkit_settings_set_allow_universal_access_from_file_urls(
        settings, map_lookup_bool(args, "allow", FALSE));
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setZoomTextOnly") == 0) {
    WebKitSettings *settings = webkit_web_view_get_settings(webview->web_view);
    webkit_settings_set_zoom_text_only(settings,
                                       map_lookup_bool(args, "enabled", FALSE));
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setDefaultFontSize") == 0) {
    WebKitSettings *settings = webkit_web_view_get_settings(webview->web_view);
    webkit_settings_set_default_font_size(
        settings, map_lookup_guint(args, "fontSize", 16));
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setDefaultMonospaceFontSize") == 0) {
    WebKitSettings *settings = webkit_web_view_get_settings(webview->web_view);
    webkit_settings_set_default_monospace_font_size(
        settings, map_lookup_guint(args, "fontSize", 13));
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setMinimumFontSize") == 0) {
    WebKitSettings *settings = webkit_web_view_get_settings(webview->web_view);
    webkit_settings_set_minimum_font_size(
        settings, map_lookup_guint(args, "fontSize", 0));
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setZoomFactor") == 0) {
    webkit_web_view_set_zoom_level(
        webview->web_view, map_lookup_double(args, "zoomFactor", 1.0));
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setUserAgent") == 0) {
    WebKitSettings *settings = webkit_web_view_get_settings(webview->web_view);
    const gchar *user_agent = map_lookup_string(args, "userAgent");
    webkit_settings_set_user_agent(settings, user_agent);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "getUserAgent") == 0) {
    WebKitSettings *settings = webkit_web_view_get_settings(webview->web_view);
    const gchar *user_agent = webkit_settings_get_user_agent(settings);
    respond(method_call, success_response(user_agent != nullptr
                                              ? fl_value_new_string(user_agent)
                                              : fl_value_new_null()));
    return;
  }

  if (strcmp(method, "setOnConsoleMessage") == 0) {
    webview->console_enabled = map_lookup_bool(args, "enabled", TRUE);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setOnScrollPositionChange") == 0) {
    webview->scroll_enabled = map_lookup_bool(args, "enabled", TRUE);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setJavaScriptDialogCallbacksEnabled") == 0) {
    webview->java_script_alert_dialog_enabled =
        map_lookup_bool(args, "alert", FALSE);
    webview->java_script_confirm_dialog_enabled =
        map_lookup_bool(args, "confirm", FALSE);
    webview->java_script_prompt_dialog_enabled =
        map_lookup_bool(args, "prompt", FALSE);
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "setOverScrollMode") == 0) {
    const gchar *mode = map_lookup_string(args, "mode");
    if (g_strcmp0(mode, "never") == 0) {
      webview->over_scroll_behavior = "none";
    } else if (g_strcmp0(mode, "ifContentScrolls") == 0) {
      webview->over_scroll_behavior = "contain";
    } else {
      webview->over_scroll_behavior = "";
    }
    rebuild_user_scripts(webview);
    gchar *script = build_overscroll_style_script(webview);
    evaluate_javascript(webview->web_view, script, method_call);
    g_free(script);
    return;
  }

  if (strcmp(method, "completeNavigationRequest") == 0) {
    gint request_id = static_cast<gint>(map_lookup_int(args, "requestId", -1));
    cancel_pending_request_timeout(webview, request_id);
    gboolean allow = map_lookup_bool(args, "allow", TRUE);
    PendingNavigationDecision *pending =
        static_cast<PendingNavigationDecision *>(g_hash_table_lookup(
            webview->pending_nav_decisions, GINT_TO_POINTER(request_id)));
    if (pending != nullptr) {
      if (allow) {
        if (pending->open_in_place && pending->uri != nullptr) {
          webkit_web_view_load_uri(webview->web_view, pending->uri);
          webkit_policy_decision_ignore(pending->decision);
        } else {
          use_navigation_decision(webview, pending->decision);
        }
      } else {
        webkit_policy_decision_ignore(pending->decision);
      }
      g_hash_table_remove(webview->pending_nav_decisions,
                          GINT_TO_POINTER(request_id));
    }
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "completeHttpAuthRequest") == 0) {
    gint request_id = static_cast<gint>(map_lookup_int(args, "requestId", -1));
    cancel_pending_request_timeout(webview, request_id);
    WebKitAuthenticationRequest *request =
        WEBKIT_AUTHENTICATION_REQUEST(g_hash_table_lookup(
            webview->pending_auth_requests, GINT_TO_POINTER(request_id)));
    if (request != nullptr) {
      const gchar *action = map_lookup_string(args, "action");
      if (g_strcmp0(action, "proceed") == 0) {
        WebKitCredential *credential =
            webkit_credential_new(map_lookup_string(args, "user") != nullptr
                                      ? map_lookup_string(args, "user")
                                      : "",
                                  map_lookup_string(args, "password") != nullptr
                                      ? map_lookup_string(args, "password")
                                      : "",
                                  WEBKIT_CREDENTIAL_PERSISTENCE_NONE);
        webkit_authentication_request_authenticate(request, credential);
        webkit_credential_free(credential);
      } else {
        webkit_authentication_request_cancel(request);
      }
      g_hash_table_remove(webview->pending_auth_requests,
                          GINT_TO_POINTER(request_id));
    }
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "completePermissionRequest") == 0) {
    gint request_id = static_cast<gint>(map_lookup_int(args, "requestId", -1));
    cancel_pending_request_timeout(webview, request_id);
    WebKitPermissionRequest *request =
        WEBKIT_PERMISSION_REQUEST(g_hash_table_lookup(
            webview->pending_permission_requests, GINT_TO_POINTER(request_id)));
    if (request != nullptr) {
      if (map_lookup_bool(args, "grant", FALSE)) {
        webkit_permission_request_allow(request);
      } else {
        webkit_permission_request_deny(request);
      }
      g_hash_table_remove(webview->pending_permission_requests,
                          GINT_TO_POINTER(request_id));
    }
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "completeJavaScriptDialog") == 0) {
    gint request_id = static_cast<gint>(map_lookup_int(args, "requestId", -1));
    cancel_pending_request_timeout(webview, request_id);
    WebKitScriptDialog *dialog =
        static_cast<WebKitScriptDialog *>(g_hash_table_lookup(
            webview->pending_script_dialogs, GINT_TO_POINTER(request_id)));
    if (dialog != nullptr) {
      const gchar *action = map_lookup_string(args, "action");
      switch (webkit_script_dialog_get_dialog_type(dialog)) {
      case WEBKIT_SCRIPT_DIALOG_ALERT:
        break;
      case WEBKIT_SCRIPT_DIALOG_CONFIRM:
      case WEBKIT_SCRIPT_DIALOG_BEFORE_UNLOAD_CONFIRM:
        webkit_script_dialog_confirm_set_confirmed(
            dialog, g_strcmp0(action, "confirm") == 0);
        break;
      case WEBKIT_SCRIPT_DIALOG_PROMPT:
        if (g_strcmp0(action, "cancel") != 0) {
          webkit_script_dialog_prompt_set_text(dialog,
                                               map_lookup_string(args, "text"));
        }
        break;
      }
      webkit_script_dialog_close(dialog);
      g_hash_table_remove(webview->pending_script_dialogs,
                          GINT_TO_POINTER(request_id));
    }
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "completeSslAuthError") == 0) {
    gint request_id = static_cast<gint>(map_lookup_int(args, "requestId", -1));
    cancel_pending_request_timeout(webview, request_id);
    PendingTlsError *pending =
        static_cast<PendingTlsError *>(g_hash_table_lookup(
            webview->pending_tls_errors, GINT_TO_POINTER(request_id)));
    if (pending != nullptr) {
      if (map_lookup_bool(args, "proceed", FALSE) &&
          pending->certificate != nullptr && pending->host != nullptr) {
        WebKitWebContext *context =
            webkit_web_view_get_context(webview->web_view);
        webkit_web_context_allow_tls_certificate_for_host(
            context, pending->certificate, pending->host);
        if (pending->uri != nullptr) {
          webkit_web_view_load_uri(webview->web_view, pending->uri);
        }
      }
      g_hash_table_remove(webview->pending_tls_errors,
                          GINT_TO_POINTER(request_id));
    }
    respond(method_call, success_response());
    return;
  }

  if (strcmp(method, "dispose") == 0) {
    webview->visible = FALSE;
    release_linux_webview_focus(webview);
    gtk_widget_hide(GTK_WIDGET(webview->web_view));
    WebviewAllLinuxPlugin *plugin = webview->plugin;
    const gint identifier = webview->id;
    respond(method_call, success_response());
    g_hash_table_remove(plugin->webviews, GINT_TO_POINTER(identifier));
    return;
  }

  respond(method_call,
          FL_METHOD_RESPONSE(fl_method_not_implemented_response_new()));
}
