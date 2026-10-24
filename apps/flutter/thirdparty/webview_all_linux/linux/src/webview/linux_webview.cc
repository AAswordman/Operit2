#include "common/method_channel_utils.h"
#include "plugin/webview_all_linux_plugin_private.h"
#include "webview/download_policy.h"
#include "webview/webview_internal.h"

#include <libsoup/soup.h>

#include <cmath>
#include <cstring>

constexpr const gchar *kResourceRequestDetailsKey =
    "webview_all_linux_request_details";

typedef struct {
  gchar *method;
  FlValue *headers;
  gboolean is_main_frame;
} ResourceRequestDetails;

enum class PendingRequestType {
  kNavigation,
  kHttpAuthentication,
  kPermission,
  kJavaScriptDialog,
  kTls,
};

typedef struct {
  LinuxWebView *webview;
  gint request_id;
  PendingRequestType type;
} PendingRequestTimeout;

constexpr guint kPendingRequestTimeoutSeconds = 30;

static void destroy_pending_tls_error(gpointer data) {
  PendingTlsError *error = static_cast<PendingTlsError *>(data);
  if (error == nullptr) {
    return;
  }
  g_clear_object(&error->certificate);
  g_free(error->host);
  g_free(error->uri);
  g_free(error);
}

static void destroy_pending_navigation_decision(gpointer data) {
  PendingNavigationDecision *pending =
      static_cast<PendingNavigationDecision *>(data);
  if (pending == nullptr) {
    return;
  }
  g_clear_object(&pending->decision);
  g_free(pending->uri);
  g_free(pending);
}

static void destroy_resource_request_details(gpointer data) {
  ResourceRequestDetails *details = static_cast<ResourceRequestDetails *>(data);
  if (details == nullptr) {
    return;
  }
  g_free(details->method);
  if (details->headers != nullptr) {
    fl_value_unref(details->headers);
  }
  g_free(details);
}

void send_event(LinuxWebView *webview, FlValue *event) {
  if (!webview->event_listening) {
    fl_value_unref(event);
    return;
  }

  GError *error = nullptr;
  fl_event_channel_send(webview->event_channel, event, nullptr, &error);
  if (error != nullptr) {
    g_warning("Failed to send event: %s", error->message);
    g_clear_error(&error);
  }
  fl_value_unref(event);
}

static gint next_request_id(LinuxWebView *webview) {
  return webview->next_request_id++;
}

static void resolve_pending_request_with_safe_default(LinuxWebView *webview,
                                                      gint request_id,
                                                      PendingRequestType type) {
  gpointer key = GINT_TO_POINTER(request_id);
  switch (type) {
  case PendingRequestType::kNavigation: {
    PendingNavigationDecision *pending =
        static_cast<PendingNavigationDecision *>(
            g_hash_table_lookup(webview->pending_nav_decisions, key));
    if (pending != nullptr) {
      webkit_policy_decision_ignore(pending->decision);
      g_hash_table_remove(webview->pending_nav_decisions, key);
    }
    break;
  }
  case PendingRequestType::kHttpAuthentication: {
    WebKitAuthenticationRequest *request = WEBKIT_AUTHENTICATION_REQUEST(
        g_hash_table_lookup(webview->pending_auth_requests, key));
    if (request != nullptr) {
      webkit_authentication_request_cancel(request);
      g_hash_table_remove(webview->pending_auth_requests, key);
    }
    break;
  }
  case PendingRequestType::kPermission: {
    WebKitPermissionRequest *request = WEBKIT_PERMISSION_REQUEST(
        g_hash_table_lookup(webview->pending_permission_requests, key));
    if (request != nullptr) {
      webkit_permission_request_deny(request);
      g_hash_table_remove(webview->pending_permission_requests, key);
    }
    break;
  }
  case PendingRequestType::kJavaScriptDialog: {
    WebKitScriptDialog *dialog = static_cast<WebKitScriptDialog *>(
        g_hash_table_lookup(webview->pending_script_dialogs, key));
    if (dialog != nullptr) {
      const WebKitScriptDialogType dialog_type =
          webkit_script_dialog_get_dialog_type(dialog);
      if (dialog_type == WEBKIT_SCRIPT_DIALOG_CONFIRM ||
          dialog_type == WEBKIT_SCRIPT_DIALOG_BEFORE_UNLOAD_CONFIRM) {
        webkit_script_dialog_confirm_set_confirmed(dialog, FALSE);
      }
      webkit_script_dialog_close(dialog);
      g_hash_table_remove(webview->pending_script_dialogs, key);
    }
    break;
  }
  case PendingRequestType::kTls:
    g_hash_table_remove(webview->pending_tls_errors, key);
    break;
  }
}

static gboolean pending_request_timeout_cb(gpointer user_data) {
  PendingRequestTimeout *timeout =
      static_cast<PendingRequestTimeout *>(user_data);
  LinuxWebView *webview = timeout->webview;
  g_hash_table_remove(webview->pending_request_timeouts,
                      GINT_TO_POINTER(timeout->request_id));
  g_warning("Linux WebView request %d timed out; applying a safe default.",
            timeout->request_id);
  resolve_pending_request_with_safe_default(webview, timeout->request_id,
                                            timeout->type);
  return G_SOURCE_REMOVE;
}

static void schedule_pending_request_timeout(LinuxWebView *webview,
                                             gint request_id,
                                             PendingRequestType type) {
  PendingRequestTimeout *timeout = g_new0(PendingRequestTimeout, 1);
  timeout->webview = webview;
  timeout->request_id = request_id;
  timeout->type = type;
  const guint source_id = g_timeout_add_seconds_full(
      G_PRIORITY_DEFAULT, kPendingRequestTimeoutSeconds,
      pending_request_timeout_cb, timeout,
      reinterpret_cast<GDestroyNotify>(g_free));
  g_hash_table_insert(webview->pending_request_timeouts,
                      GINT_TO_POINTER(request_id), GUINT_TO_POINTER(source_id));
}

void cancel_pending_request_timeout(LinuxWebView *webview, gint request_id) {
  gpointer key = GINT_TO_POINTER(request_id);
  const guint source_id = GPOINTER_TO_UINT(
      g_hash_table_lookup(webview->pending_request_timeouts, key));
  if (source_id != 0) {
    g_source_remove(source_id);
    g_hash_table_remove(webview->pending_request_timeouts, key);
  }
}

static void resolve_all_pending_requests(LinuxWebView *webview) {
  GList *timeout_ids = g_hash_table_get_keys(webview->pending_request_timeouts);
  for (GList *item = timeout_ids; item != nullptr; item = item->next) {
    cancel_pending_request_timeout(webview, GPOINTER_TO_INT(item->data));
  }
  g_list_free(timeout_ids);

  GList *request_ids = g_hash_table_get_keys(webview->pending_nav_decisions);
  for (GList *item = request_ids; item != nullptr; item = item->next) {
    resolve_pending_request_with_safe_default(
        webview, GPOINTER_TO_INT(item->data), PendingRequestType::kNavigation);
  }
  g_list_free(request_ids);

  request_ids = g_hash_table_get_keys(webview->pending_auth_requests);
  for (GList *item = request_ids; item != nullptr; item = item->next) {
    resolve_pending_request_with_safe_default(
        webview, GPOINTER_TO_INT(item->data),
        PendingRequestType::kHttpAuthentication);
  }
  g_list_free(request_ids);

  request_ids = g_hash_table_get_keys(webview->pending_permission_requests);
  for (GList *item = request_ids; item != nullptr; item = item->next) {
    resolve_pending_request_with_safe_default(
        webview, GPOINTER_TO_INT(item->data), PendingRequestType::kPermission);
  }
  g_list_free(request_ids);

  request_ids = g_hash_table_get_keys(webview->pending_script_dialogs);
  for (GList *item = request_ids; item != nullptr; item = item->next) {
    resolve_pending_request_with_safe_default(
        webview, GPOINTER_TO_INT(item->data),
        PendingRequestType::kJavaScriptDialog);
  }
  g_list_free(request_ids);

  g_hash_table_remove_all(webview->pending_tls_errors);
}

void use_navigation_decision(LinuxWebView *webview,
                             WebKitPolicyDecision *decision) {
  if (webview->media_playback_requires_user_gesture < 0) {
    webkit_policy_decision_use(decision);
    return;
  }

  const WebKitAutoplayPolicy autoplay_policy =
      webview->media_playback_requires_user_gesture ? WEBKIT_AUTOPLAY_DENY
                                                    : WEBKIT_AUTOPLAY_ALLOW;
  WebKitWebsitePolicies *policies = webkit_website_policies_new_with_policies(
      "autoplay", autoplay_policy, nullptr);
  webkit_policy_decision_use_with_policies(decision, policies);
  g_object_unref(policies);
}

static WebKitWebView *
create_web_view_cb(WebKitWebView *widget,
                   WebKitNavigationAction *navigation_action, gpointer) {
  WebKitURIRequest *request =
      webkit_navigation_action_get_request(navigation_action);
  const gchar *uri =
      request == nullptr ? nullptr : webkit_uri_request_get_uri(request);
  if (uri != nullptr && *uri != '\0') {
    webkit_web_view_load_uri(widget, uri);
  }
  return nullptr;
}

static gboolean decide_policy_cb(WebKitWebView *widget,
                                 WebKitPolicyDecision *decision,
                                 WebKitPolicyDecisionType type,
                                 gpointer user_data) {
  if (type != WEBKIT_POLICY_DECISION_TYPE_NAVIGATION_ACTION &&
      type != WEBKIT_POLICY_DECISION_TYPE_NEW_WINDOW_ACTION) {
    return FALSE;
  }

  LinuxWebView *webview = static_cast<LinuxWebView *>(user_data);
  WebKitNavigationPolicyDecision *navigation_decision =
      WEBKIT_NAVIGATION_POLICY_DECISION(decision);
  WebKitNavigationAction *navigation_action =
      webkit_navigation_policy_decision_get_navigation_action(
          navigation_decision);
  WebKitURIRequest *request =
      webkit_navigation_action_get_request(navigation_action);
  const gchar *uri = webkit_uri_request_get_uri(request);

  if (type == WEBKIT_POLICY_DECISION_TYPE_NEW_WINDOW_ACTION &&
      (!webview->event_listening || uri == nullptr || *uri == '\0')) {
    if (uri != nullptr && *uri != '\0') {
      webkit_web_view_load_uri(widget, uri);
      webkit_policy_decision_ignore(decision);
      return TRUE;
    }
    return FALSE;
  }
  if (!webview->event_listening) {
    if (webview->media_playback_requires_user_gesture >= 0) {
      use_navigation_decision(webview, decision);
      return TRUE;
    }
    return FALSE;
  }

  gint request_id = next_request_id(webview);
  PendingNavigationDecision *pending = g_new0(PendingNavigationDecision, 1);
  pending->decision = WEBKIT_POLICY_DECISION(g_object_ref(decision));
  pending->uri = g_strdup(uri);
  pending->open_in_place =
      type == WEBKIT_POLICY_DECISION_TYPE_NEW_WINDOW_ACTION;
  g_hash_table_insert(webview->pending_nav_decisions,
                      GINT_TO_POINTER(request_id), pending);
  schedule_pending_request_timeout(webview, request_id,
                                   PendingRequestType::kNavigation);

  FlValue *event = make_event("navigationRequest");
  fl_value_set_string_take(event, "requestId", fl_value_new_int(request_id));
  fl_value_set_string_take(event, "url",
                           fl_value_new_string(uri != nullptr ? uri : ""));
  fl_value_set_string_take(event, "isMainFrame", fl_value_new_bool(true));
  send_event(webview, event);
  return TRUE;
}

static void load_changed_cb(WebKitWebView *widget, WebKitLoadEvent load_event,
                            gpointer user_data) {
  LinuxWebView *webview = static_cast<LinuxWebView *>(user_data);
  const gchar *uri = webkit_web_view_get_uri(widget);
  switch (load_event) {
  case WEBKIT_LOAD_STARTED: {
    FlValue *event = make_event("pageStarted");
    fl_value_set_string_take(event, "url",
                             fl_value_new_string(uri != nullptr ? uri : ""));
    send_event(webview, event);
    break;
  }
  case WEBKIT_LOAD_FINISHED: {
    FlValue *event = make_event("pageFinished");
    fl_value_set_string_take(event, "url",
                             fl_value_new_string(uri != nullptr ? uri : ""));
    send_event(webview, event);
    break;
  }
  default:
    break;
  }
  emit_url_change(webview);
  update_history(webview);
}

static void notify_progress_cb(WebKitWebView *widget, GParamSpec *pspec,
                               gpointer user_data) {
  LinuxWebView *webview = static_cast<LinuxWebView *>(user_data);
  FlValue *event = make_event("progress");
  fl_value_set_string_take(
      event, "progress",
      fl_value_new_int(static_cast<gint>(
          webkit_web_view_get_estimated_load_progress(widget) * 100.0)));
  send_event(webview, event);
}

static void notify_uri_cb(WebKitWebView *widget, GParamSpec *pspec,
                          gpointer user_data) {
  emit_url_change(static_cast<LinuxWebView *>(user_data));
}

static void notify_title_cb(WebKitWebView *widget, GParamSpec *pspec,
                            gpointer user_data) {
  emit_title_change(static_cast<LinuxWebView *>(user_data));
}

static gboolean load_failed_cb(WebKitWebView *widget,
                               WebKitLoadEvent load_event,
                               const gchar *failing_url, GError *error,
                               gpointer user_data) {
  if (error != nullptr && error->domain == webkit_network_error_quark() &&
      error->code == WEBKIT_NETWORK_ERROR_CANCELLED) {
    return FALSE;
  }
  emit_load_error(static_cast<LinuxWebView *>(user_data), error, failing_url);
  return FALSE;
}

static gboolean zoom_scroll_event_cb(GtkWidget *widget, GdkEventScroll *event,
                                     gpointer user_data) {
  LinuxWebView *webview = static_cast<LinuxWebView *>(user_data);
  return !webview->zoom_enabled &&
         (event->state & GDK_CONTROL_MASK) != 0;
}

static gboolean zoom_key_press_event_cb(GtkWidget *widget, GdkEventKey *event,
                                        gpointer user_data) {
  LinuxWebView *webview = static_cast<LinuxWebView *>(user_data);
  if (webview->zoom_enabled || (event->state & GDK_CONTROL_MASK) == 0) {
    return FALSE;
  }

  switch (event->keyval) {
  case GDK_KEY_plus:
  case GDK_KEY_equal:
  case GDK_KEY_minus:
  case GDK_KEY_0:
  case GDK_KEY_KP_Add:
  case GDK_KEY_KP_Subtract:
  case GDK_KEY_KP_0:
    return TRUE;
  default:
    return FALSE;
  }
}

static void copy_response_header(const char *name, const char *value,
                                 gpointer user_data) {
  FlValue *headers = static_cast<FlValue *>(user_data);
  if (name == nullptr || value == nullptr) {
    return;
  }
  fl_value_set_string_take(headers, name, fl_value_new_string(value));
}

static void resource_response_cb(WebKitWebResource *resource, GParamSpec *pspec,
                                 gpointer user_data) {
  WebKitWebView *web_view = WEBKIT_WEB_VIEW(user_data);
  LinuxWebView *webview = static_cast<LinuxWebView *>(
      g_object_get_data(G_OBJECT(web_view), kLinuxWebViewInstanceKey));
  if (webview == nullptr || !webview->event_listening) {
    return;
  }

  WebKitURIResponse *response = webkit_web_resource_get_response(resource);
  if (response == nullptr) {
    return;
  }

  guint status_code = webkit_uri_response_get_status_code(response);
  if (status_code < 400) {
    return;
  }

  const gchar *uri = webkit_uri_response_get_uri(response);
  if (uri == nullptr || *uri == '\0') {
    uri = webkit_web_resource_get_uri(resource);
  }

  FlValue *event = make_event("httpError");
  fl_value_set_string_take(event, "url",
                           fl_value_new_string(uri != nullptr ? uri : ""));
  fl_value_set_string_take(event, "statusCode",
                           fl_value_new_int(static_cast<gint>(status_code)));
  const gchar *mime_type = webkit_uri_response_get_mime_type(response);
  if (mime_type != nullptr) {
    fl_value_set_string_take(event, "mimeType", fl_value_new_string(mime_type));
  }
  FlValue *headers = fl_value_new_map();
  SoupMessageHeaders *raw_headers =
      webkit_uri_response_get_http_headers(response);
  if (raw_headers != nullptr) {
    soup_message_headers_foreach(raw_headers, copy_response_header, headers);
  }
  fl_value_set_string_take(event, "headers", headers);

  ResourceRequestDetails *request_details =
      static_cast<ResourceRequestDetails *>(
          g_object_get_data(G_OBJECT(resource), kResourceRequestDetailsKey));
  if (request_details != nullptr) {
    fl_value_set_string_take(
        event, "method",
        fl_value_new_string(
            request_details->method != nullptr ? request_details->method : ""));
    fl_value_set_string_take(event, "requestHeaders",
                             request_details->headers == nullptr
                                 ? fl_value_new_map()
                                 : fl_value_ref(request_details->headers));
    fl_value_set_string_take(event, "isForMainFrame",
                             fl_value_new_bool(request_details->is_main_frame));
  }
  send_event(webview, event);
}

static void resource_load_started_cb(WebKitWebView *widget,
                                     WebKitWebResource *resource,
                                     WebKitURIRequest *request,
                                     gpointer user_data) {
  ResourceRequestDetails *details = g_new0(ResourceRequestDetails, 1);
  const gchar *method = webkit_uri_request_get_http_method(request);
  details->method = g_strdup(method != nullptr ? method : "");
  details->headers = fl_value_new_map();
  SoupMessageHeaders *headers = webkit_uri_request_get_http_headers(request);
  if (headers != nullptr) {
    soup_message_headers_foreach(headers, copy_response_header,
                                 details->headers);
  }
  details->is_main_frame =
      webkit_web_view_get_main_resource(widget) == resource;
  g_object_set_data_full(G_OBJECT(resource), kResourceRequestDetailsKey,
                         details, destroy_resource_request_details);

  g_signal_connect_object(resource, "notify::response",
                          G_CALLBACK(resource_response_cb), widget,
                          static_cast<GConnectFlags>(0));
}

static gboolean authenticate_cb(WebKitWebView *widget,
                                WebKitAuthenticationRequest *request,
                                gpointer user_data) {
  LinuxWebView *webview = static_cast<LinuxWebView *>(user_data);
  if (!webview->event_listening) {
    return FALSE;
  }

  gint request_id = next_request_id(webview);
  g_hash_table_insert(webview->pending_auth_requests,
                      GINT_TO_POINTER(request_id), g_object_ref(request));
  schedule_pending_request_timeout(webview, request_id,
                                   PendingRequestType::kHttpAuthentication);

  FlValue *event = make_event("httpAuthRequest");
  fl_value_set_string_take(event, "requestId", fl_value_new_int(request_id));
  fl_value_set_string_take(
      event, "host",
      fl_value_new_string(webkit_authentication_request_get_host(request) !=
                                  nullptr
                              ? webkit_authentication_request_get_host(request)
                              : ""));
  const gchar *realm = webkit_authentication_request_get_realm(request);
  if (realm != nullptr) {
    fl_value_set_string_take(event, "realm", fl_value_new_string(realm));
  }
  send_event(webview, event);
  return TRUE;
}

static gboolean permission_request_cb(WebKitWebView *widget,
                                      WebKitPermissionRequest *request,
                                      gpointer user_data) {
  LinuxWebView *webview = static_cast<LinuxWebView *>(user_data);
  if (!webview->event_listening) {
    return FALSE;
  }

  gint request_id = next_request_id(webview);
  g_hash_table_insert(webview->pending_permission_requests,
                      GINT_TO_POINTER(request_id), g_object_ref(request));
  schedule_pending_request_timeout(webview, request_id,
                                   PendingRequestType::kPermission);

  FlValue *event = make_event("permissionRequest");
  FlValue *types = fl_value_new_list();
  if (WEBKIT_IS_USER_MEDIA_PERMISSION_REQUEST(request)) {
    WebKitUserMediaPermissionRequest *media_request =
        WEBKIT_USER_MEDIA_PERMISSION_REQUEST(request);
    if (webkit_user_media_permission_is_for_video_device(media_request)) {
      fl_value_append_take(types, fl_value_new_string("camera"));
    }
    if (webkit_user_media_permission_is_for_audio_device(media_request)) {
      fl_value_append_take(types, fl_value_new_string("microphone"));
    }
  }
  fl_value_set_string_take(event, "requestId", fl_value_new_int(request_id));
  fl_value_set_string_take(event, "types", types);
  send_event(webview, event);
  return TRUE;
}

static const gchar *script_dialog_type_name(WebKitScriptDialogType type) {
  switch (type) {
  case WEBKIT_SCRIPT_DIALOG_ALERT:
    return "alert";
  case WEBKIT_SCRIPT_DIALOG_CONFIRM:
    return "confirm";
  case WEBKIT_SCRIPT_DIALOG_PROMPT:
    return "prompt";
  case WEBKIT_SCRIPT_DIALOG_BEFORE_UNLOAD_CONFIRM:
    return "beforeUnloadConfirm";
  }
  return "alert";
}

static gboolean script_dialog_enabled(LinuxWebView *webview,
                                      WebKitScriptDialogType type) {
  switch (type) {
  case WEBKIT_SCRIPT_DIALOG_ALERT:
    return webview->java_script_alert_dialog_enabled;
  case WEBKIT_SCRIPT_DIALOG_CONFIRM:
  case WEBKIT_SCRIPT_DIALOG_BEFORE_UNLOAD_CONFIRM:
    return webview->java_script_confirm_dialog_enabled;
  case WEBKIT_SCRIPT_DIALOG_PROMPT:
    return webview->java_script_prompt_dialog_enabled;
  }
  return FALSE;
}

static gboolean script_dialog_cb(WebKitWebView *widget,
                                 WebKitScriptDialog *dialog,
                                 gpointer user_data) {
  LinuxWebView *webview = static_cast<LinuxWebView *>(user_data);
  WebKitScriptDialogType dialog_type =
      webkit_script_dialog_get_dialog_type(dialog);
  if (!webview->event_listening ||
      !script_dialog_enabled(webview, dialog_type)) {
    return FALSE;
  }

  gint request_id = next_request_id(webview);
  g_hash_table_insert(webview->pending_script_dialogs,
                      GINT_TO_POINTER(request_id),
                      webkit_script_dialog_ref(dialog));
  schedule_pending_request_timeout(webview, request_id,
                                   PendingRequestType::kJavaScriptDialog);

  FlValue *event = make_event("javaScriptDialog");
  fl_value_set_string_take(event, "requestId", fl_value_new_int(request_id));
  fl_value_set_string_take(
      event, "dialogType",
      fl_value_new_string(script_dialog_type_name(dialog_type)));
  fl_value_set_string_take(
      event, "message",
      fl_value_new_string(webkit_script_dialog_get_message(dialog) != nullptr
                              ? webkit_script_dialog_get_message(dialog)
                              : ""));
  const gchar *default_text =
      webkit_script_dialog_prompt_get_default_text(dialog);
  if (default_text != nullptr) {
    fl_value_set_string_take(event, "defaultText",
                             fl_value_new_string(default_text));
  }
  const gchar *uri = webkit_web_view_get_uri(widget);
  fl_value_set_string_take(event, "url",
                           fl_value_new_string(uri != nullptr ? uri : ""));
  send_event(webview, event);
  return TRUE;
}

static gchar *describe_tls_error(const gchar *uri, const gchar *host,
                                 GTlsCertificateFlags errors) {
  GString *description = g_string_new("TLS certificate error");
  if (host != nullptr && *host != '\0') {
    g_string_append_printf(description, " for %s", host);
  }
  if (uri != nullptr && *uri != '\0') {
    g_string_append_printf(description, " while loading %s", uri);
  }
  if (errors != 0) {
    g_string_append(description, ".");
  }
  if (errors & G_TLS_CERTIFICATE_UNKNOWN_CA) {
    g_string_append(description, " Unknown certificate authority.");
  }
  if (errors & G_TLS_CERTIFICATE_BAD_IDENTITY) {
    g_string_append(description, " Certificate does not match host.");
  }
  if (errors & G_TLS_CERTIFICATE_NOT_ACTIVATED) {
    g_string_append(description, " Certificate is not yet valid.");
  }
  if (errors & G_TLS_CERTIFICATE_EXPIRED) {
    g_string_append(description, " Certificate has expired.");
  }
  if (errors & G_TLS_CERTIFICATE_REVOKED) {
    g_string_append(description, " Certificate has been revoked.");
  }
  if (errors & G_TLS_CERTIFICATE_INSECURE) {
    g_string_append(description, " Certificate uses an insecure algorithm.");
  }
  if (errors & G_TLS_CERTIFICATE_GENERIC_ERROR) {
    g_string_append(description, " Certificate validation failed.");
  }
  return g_string_free(description, FALSE);
}

static gboolean load_failed_with_tls_errors_cb(WebKitWebView *widget,
                                               const gchar *failing_uri,
                                               GTlsCertificate *certificate,
                                               GTlsCertificateFlags errors,
                                               gpointer user_data) {
  LinuxWebView *webview = static_cast<LinuxWebView *>(user_data);
  if (!webview->event_listening) {
    return FALSE;
  }

  gchar *host = nullptr;
  if (failing_uri != nullptr) {
    GUri *uri = g_uri_parse(failing_uri, G_URI_FLAGS_NONE, nullptr);
    if (uri != nullptr) {
      host = g_strdup(g_uri_get_host(uri));
      g_uri_unref(uri);
    }
  }

  PendingTlsError *pending = g_new0(PendingTlsError, 1);
  pending->certificate = certificate != nullptr
                             ? G_TLS_CERTIFICATE(g_object_ref(certificate))
                             : nullptr;
  pending->host = host;
  pending->uri = g_strdup(failing_uri);

  gint request_id = next_request_id(webview);
  g_hash_table_insert(webview->pending_tls_errors, GINT_TO_POINTER(request_id),
                      pending);
  schedule_pending_request_timeout(webview, request_id,
                                   PendingRequestType::kTls);

  FlValue *event = make_event("sslAuthError");
  fl_value_set_string_take(event, "requestId", fl_value_new_int(request_id));
  gchar *description = describe_tls_error(failing_uri, host, errors);
  fl_value_set_string_take(event, "description",
                           fl_value_new_string(description));
  g_free(description);
  if (failing_uri != nullptr) {
    fl_value_set_string_take(event, "url", fl_value_new_string(failing_uri));
  }
  send_event(webview, event);
  return TRUE;
}

static FlMethodErrorResponse *
event_listen_cb(FlEventChannel *channel, FlValue *args, gpointer user_data) {
  LinuxWebView *webview = static_cast<LinuxWebView *>(user_data);
  webview->event_listening = TRUE;
  return nullptr;
}

static FlMethodErrorResponse *
event_cancel_cb(FlEventChannel *channel, FlValue *args, gpointer user_data) {
  LinuxWebView *webview = static_cast<LinuxWebView *>(user_data);
  webview->event_listening = FALSE;
  resolve_all_pending_requests(webview);
  return nullptr;
}

void destroy_linux_webview(gpointer data) {
  LinuxWebView *webview = static_cast<LinuxWebView *>(data);
  if (webview == nullptr) {
    return;
  }

  if (webview->web_view != nullptr) {
    delete webview->download_policy;
    webview->download_policy = nullptr;
    webview->visible = FALSE;
    release_linux_webview_focus(webview);
    resolve_all_pending_requests(webview);
    g_object_set_data(G_OBJECT(webview->web_view), kLinuxWebViewInstanceKey,
                      nullptr);
    if (webview->clip_container != nullptr) {
      g_object_set_data(G_OBJECT(webview->clip_container),
                        kLinuxWebViewInstanceKey, nullptr);
      g_object_remove_weak_pointer(
          G_OBJECT(webview->clip_container),
          reinterpret_cast<gpointer *>(&webview->clip_container));
      GtkWidget* container = webview->clip_container;
      g_object_ref(container);
      if (webview->plugin->flutter_view != nullptr) {
        fl_view_unregister_native_view(FL_VIEW(webview->plugin->flutter_view),
                                       webview->id);
      }
      gtk_widget_destroy(container);
      g_object_unref(container);
      webview->clip_container = nullptr;
    } else {
      gtk_widget_destroy(GTK_WIDGET(webview->web_view));
    }
    g_object_unref(webview->web_view);
  }
  g_clear_object(&webview->method_channel);
  g_clear_object(&webview->event_channel);
  g_clear_object(&webview->content_manager);
  g_hash_table_destroy(webview->pending_nav_decisions);
  g_hash_table_destroy(webview->pending_auth_requests);
  g_hash_table_destroy(webview->pending_permission_requests);
  g_hash_table_destroy(webview->pending_script_dialogs);
  g_hash_table_destroy(webview->pending_tls_errors);
  g_hash_table_destroy(webview->pending_request_timeouts);
  g_hash_table_destroy(webview->js_channel_signal_ids);
  g_hash_table_destroy(webview->js_channels);
  g_hash_table_destroy(webview->user_scripts);
  g_ptr_array_unref(webview->user_script_order);
  g_free(webview);
}

LinuxWebView *create_linux_webview(WebviewAllLinuxPlugin *self) {
  LinuxWebView *webview = g_new0(LinuxWebView, 1);
  webview->plugin = self;
  webview->id = self->next_webview_id++;
  webview->content_manager = webkit_user_content_manager_new();
  webview->web_view = WEBKIT_WEB_VIEW(
      webkit_web_view_new_with_user_content_manager(webview->content_manager));
  g_object_ref_sink(webview->web_view);
  g_object_set_data(G_OBJECT(webview->web_view), kLinuxWebViewInstanceKey,
                    webview);
  ensure_local_resource_scheme(webkit_web_view_get_context(webview->web_view));
  webview->pending_nav_decisions =
      g_hash_table_new_full(g_direct_hash, g_direct_equal, nullptr,
                            destroy_pending_navigation_decision);
  webview->pending_auth_requests =
      g_hash_table_new_full(g_direct_hash, g_direct_equal, nullptr,
                            reinterpret_cast<GDestroyNotify>(g_object_unref));
  webview->pending_permission_requests =
      g_hash_table_new_full(g_direct_hash, g_direct_equal, nullptr,
                            reinterpret_cast<GDestroyNotify>(g_object_unref));
  webview->pending_script_dialogs = g_hash_table_new_full(
      g_direct_hash, g_direct_equal, nullptr,
      reinterpret_cast<GDestroyNotify>(webkit_script_dialog_unref));
  webview->pending_tls_errors = g_hash_table_new_full(
      g_direct_hash, g_direct_equal, nullptr, destroy_pending_tls_error);
  webview->pending_request_timeouts =
      g_hash_table_new(g_direct_hash, g_direct_equal);
  webview->js_channel_signal_ids =
      g_hash_table_new_full(g_str_hash, g_str_equal, g_free, nullptr);
  webview->js_channels =
      g_hash_table_new_full(g_str_hash, g_str_equal, g_free, nullptr);
  webview->user_scripts = g_hash_table_new_full(
      g_str_hash, g_str_equal, g_free,
      reinterpret_cast<GDestroyNotify>(webkit_user_script_unref));
  webview->user_script_order = g_ptr_array_new_with_free_func(g_free);
  webview->next_request_id = 1;
  webview->java_script_alert_dialog_enabled = FALSE;
  webview->java_script_confirm_dialog_enabled = FALSE;
  webview->java_script_prompt_dialog_enabled = FALSE;
  webview->vertical_scrollbar_enabled = TRUE;
  webview->horizontal_scrollbar_enabled = TRUE;
  webview->zoom_enabled = TRUE;
  webview->download_policy = new DownloadPolicy(webview->web_view);
  webview->media_playback_requires_user_gesture = -1;
  webview->frame_sequence = 0;
  webview->over_scroll_behavior = "";

  gchar *method_name =
      g_strdup_printf("com.abandoft.webview_all_linux/%d", webview->id);
  webview->method_channel =
      fl_method_channel_new(fl_plugin_registrar_get_messenger(self->registrar),
                            method_name, method_codec());
  g_free(method_name);

  gchar *event_name =
      g_strdup_printf("com.abandoft.webview_all_linux/%d/events", webview->id);
  webview->event_channel =
      fl_event_channel_new(fl_plugin_registrar_get_messenger(self->registrar),
                           event_name, method_codec());
  g_free(event_name);

  fl_method_channel_set_method_call_handler(
      webview->method_channel, instance_method_call_cb, webview, nullptr);
  fl_event_channel_set_stream_handlers(webview->event_channel, event_listen_cb,
                                       event_cancel_cb, webview, nullptr);

  webkit_user_content_manager_register_script_message_handler(
      webview->content_manager, "__webview_all_console");
  webkit_user_content_manager_register_script_message_handler(
      webview->content_manager, "__webview_all_scroll");
  webkit_user_content_manager_register_script_message_handler(
      webview->content_manager, "__webview_all_async_javascript");
  g_signal_connect(webview->content_manager,
                   "script-message-received::__webview_all_console",
                   G_CALLBACK(console_message_received_cb), webview);
  g_signal_connect(webview->content_manager,
                   "script-message-received::__webview_all_scroll",
                   G_CALLBACK(scroll_message_received_cb), webview);
  g_signal_connect(webview->content_manager,
                   "script-message-received::__webview_all_async_javascript",
                   G_CALLBACK(async_javascript_message_received_cb), webview);

  rebuild_user_scripts(webview);

  g_signal_connect(webview->web_view, "decide-policy",
                   G_CALLBACK(decide_policy_cb), webview);
  g_signal_connect(webview->web_view, "create", G_CALLBACK(create_web_view_cb),
                   webview);
  g_signal_connect(webview->web_view, "load-changed",
                   G_CALLBACK(load_changed_cb), webview);
  g_signal_connect(webview->web_view, "notify::estimated-load-progress",
                   G_CALLBACK(notify_progress_cb), webview);
  g_signal_connect(webview->web_view, "notify::uri", G_CALLBACK(notify_uri_cb),
                   webview);
  g_signal_connect(webview->web_view, "notify::title",
                   G_CALLBACK(notify_title_cb), webview);
  g_signal_connect(webview->web_view, "load-failed", G_CALLBACK(load_failed_cb),
                   webview);
  g_signal_connect(webview->web_view, "resource-load-started",
                   G_CALLBACK(resource_load_started_cb), webview);
  g_signal_connect(webview->web_view, "authenticate",
                   G_CALLBACK(authenticate_cb), webview);
  g_signal_connect(webview->web_view, "permission-request",
                   G_CALLBACK(permission_request_cb), webview);
  g_signal_connect(webview->web_view, "script-dialog",
                   G_CALLBACK(script_dialog_cb), webview);
  g_signal_connect(webview->web_view, "load-failed-with-tls-errors",
                   G_CALLBACK(load_failed_with_tls_errors_cb), webview);
  g_signal_connect(webview->web_view, "scroll-event",
                   G_CALLBACK(zoom_scroll_event_cb), webview);
  g_signal_connect(webview->web_view, "key-press-event",
                   G_CALLBACK(zoom_key_press_event_cb), webview);

  gtk_widget_set_halign(GTK_WIDGET(webview->web_view), GTK_ALIGN_START);
  gtk_widget_set_valign(GTK_WIDGET(webview->web_view), GTK_ALIGN_START);
  gtk_widget_set_hexpand(GTK_WIDGET(webview->web_view), FALSE);
  gtk_widget_set_vexpand(GTK_WIDGET(webview->web_view), FALSE);
  gtk_widget_set_can_focus(GTK_WIDGET(webview->web_view), TRUE);
  gtk_widget_set_focus_on_click(GTK_WIDGET(webview->web_view), TRUE);
  gtk_widget_set_sensitive(GTK_WIDGET(webview->web_view), TRUE);
  gtk_widget_add_events(GTK_WIDGET(webview->web_view),
                        GDK_BUTTON_PRESS_MASK | GDK_BUTTON_RELEASE_MASK |
                            GDK_KEY_PRESS_MASK |
                            GDK_SCROLL_MASK | GDK_SMOOTH_SCROLL_MASK |
                            GDK_POINTER_MOTION_MASK);
  gtk_widget_set_size_request(GTK_WIDGET(webview->web_view), 1, 1);
  gtk_widget_hide(GTK_WIDGET(webview->web_view));

  g_hash_table_insert(self->webviews, GINT_TO_POINTER(webview->id), webview);
  return webview;
}
