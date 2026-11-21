#include "webview/download_policy.h"
#include <glib/gstdio.h>

// Scopes engine downloads to their originating view, including shared contexts.
DownloadPolicy::DownloadPolicy(WebKitWebView *web_view) : web_view_(web_view) {
  signal_id_ = g_signal_connect(webkit_web_view_get_context(web_view_),
      "download-started", G_CALLBACK(OnDownloadStarted), this);
}

// Disconnects native callbacks before the owning WebView is released.
DownloadPolicy::~DownloadPolicy() {
  g_signal_handler_disconnect(webkit_web_view_get_context(web_view_), signal_id_);
  while (!transfers_.empty()) {
    const auto id = transfers_.begin()->first;
    g_signal_handlers_disconnect_by_data(transfers_.begin()->second.download, this);
    Cancel(id);
    Release(id);
  }
}

// Takes ownership of an original response and assigns a unique staging directory.
void DownloadPolicy::OnDownloadStarted(WebKitWebContext *, WebKitDownload *download,
                                       gpointer user_data) {
  auto *policy = static_cast<DownloadPolicy *>(user_data);
  if (webkit_download_get_web_view(download) != policy->web_view_) return;
  if (!policy->enabled_) {
    webkit_download_cancel(download);
    return;
  }
  if (!policy->callback_) return;
  gchar *id = g_uuid_string_random();
  const gchar *uri = webkit_uri_request_get_uri(webkit_download_get_request(download));
  Update update;
  update.id = id;
  update.url = uri;
  update.state = "running";
  policy->transfers_.emplace(id, Transfer{WEBKIT_DOWNLOAD(g_object_ref(download)), update});
  g_free(id);
  g_signal_connect(download, "decide-destination", G_CALLBACK((+[](
      WebKitDownload *download, const gchar *suggested, gpointer data) -> gboolean {
    auto *policy = static_cast<DownloadPolicy *>(data);
    for (auto &[id, transfer] : policy->transfers_) {
      if (transfer.download != download) continue;
      GError *error = nullptr;
      gchar *directory = g_dir_make_tmp("operit-browser-XXXXXX", &error);
      if (!directory) {
        policy->Publish(download, "failed", error->message);
        g_clear_error(&error);
        webkit_download_cancel(download);
        return TRUE;
      }
      gchar *name = g_path_get_basename(suggested);
      if (g_str_equal(name, ".") || g_str_equal(name, "..")) {
        g_free(name);
        name = g_strdup("download");
      }
      gchar *path = g_build_filename(directory, name, nullptr);
      gchar *destination = g_filename_to_uri(path, nullptr, nullptr);
      transfer.update.filename = name;
      transfer.update.path = path;
      webkit_download_set_destination(download, destination);
      policy->Publish(download, "running");
      g_free(destination); g_free(path); g_free(name); g_free(directory);
      return TRUE;
    }
    return FALSE;
  })), policy);
  g_signal_connect(download, "received-data", G_CALLBACK((+[](
      WebKitDownload *download, guint64, gpointer data) {
    auto *policy = static_cast<DownloadPolicy *>(data);
    const auto now = g_get_monotonic_time();
    for (auto &[id, transfer] : policy->transfers_) {
      if (transfer.download != download || now - transfer.last_progress < 100000) continue;
      transfer.last_progress = now;
      policy->Publish(download, "running");
      break;
    }
  })), policy);
  g_signal_connect(download, "failed", G_CALLBACK((+[](
      WebKitDownload *download, GError *error, gpointer data) {
    static_cast<DownloadPolicy *>(data)->Publish(download,
        g_error_matches(error, WEBKIT_DOWNLOAD_ERROR, WEBKIT_DOWNLOAD_ERROR_CANCELLED_BY_USER)
            ? "cancelled" : "failed", error->message);
  })), policy);
  g_signal_connect(download, "finished", G_CALLBACK((+[](
      WebKitDownload *download, gpointer data) {
    auto *policy = static_cast<DownloadPolicy *>(data);
    for (const auto &[id, transfer] : policy->transfers_) {
      if (transfer.download == download && transfer.update.state == "running") {
        policy->Publish(download, "completed");
        break;
      }
    }
  })), policy);
}

// Reads progress from the same response that the browser engine is saving.
void DownloadPolicy::Publish(WebKitDownload *download, const char *state, const char *error) {
  for (auto &[id, transfer] : transfers_) {
    if (transfer.download != download) continue;
    transfer.update.state = state;
    transfer.update.error = error;
    transfer.update.received = webkit_download_get_received_data_length(download);
    auto *response = webkit_download_get_response(download);
    if (response) transfer.update.total = webkit_uri_response_get_content_length(response);
    callback_(transfer.update);
    return;
  }
}

// Stops the transfer identified by its engine-generated identifier.
bool DownloadPolicy::Cancel(const std::string &id) {
  const auto entry = transfers_.find(id);
  if (entry == transfers_.end()) return false;
  webkit_download_cancel(entry->second.download);
  return true;
}

// Releases a terminal transfer without touching any user-selected directory.
void DownloadPolicy::Release(const std::string &id) {
  const auto entry = transfers_.find(id);
  if (entry == transfers_.end()) return;
  auto &transfer = entry->second;
  g_signal_handlers_disconnect_by_data(transfer.download, this);
  if (!transfer.update.path.empty()) {
    g_unlink(transfer.update.path.c_str());
    gchar *directory = g_path_get_dirname(transfer.update.path.c_str());
    g_rmdir(directory);
    g_free(directory);
  }
  g_object_unref(transfer.download);
  transfers_.erase(entry);
}
