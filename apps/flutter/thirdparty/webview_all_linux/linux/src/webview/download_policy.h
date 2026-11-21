#ifndef WEBVIEW_ALL_LINUX_DOWNLOAD_POLICY_H_
#define WEBVIEW_ALL_LINUX_DOWNLOAD_POLICY_H_

#include <webkit2/webkit2.h>
#include <functional>
#include <map>
#include <string>

// Owned by a single WebView and destroyed before that WebView is released.
class DownloadPolicy {
public:
  explicit DownloadPolicy(WebKitWebView *web_view);
  ~DownloadPolicy();
  DownloadPolicy(const DownloadPolicy &) = delete;
  DownloadPolicy &operator=(const DownloadPolicy &) = delete;

  void SetEnabled(bool enabled) { enabled_ = enabled; }
  struct Update {
    std::string id, url, filename, path, state, error;
    gint64 received = 0, total = -1;
  };
  using Callback = std::function<void(const Update &)>;
  // Registers the owner of engine transfer notifications.
  void SetCallback(Callback callback) { callback_ = std::move(callback); }
  // Cancels an original response without issuing another network request.
  bool Cancel(const std::string &id);
  // Disconnects callbacks and deletes only this transfer's private staging file.
  void Release(const std::string &id);

private:
  static void OnDownloadStarted(WebKitWebContext *context,
                                WebKitDownload *download, gpointer user_data);
  struct Transfer {
    WebKitDownload *download;
    Update update;
    gint64 last_progress = 0;
  };
  // Publishes byte counts directly from WebKit's original response.
  void Publish(WebKitDownload *download, const char *state, const char *error = "");
  Callback callback_;
  std::map<std::string, Transfer> transfers_;
  WebKitWebView *web_view_;
  gulong signal_id_;
  bool enabled_ = true;
};

#endif // WEBVIEW_ALL_LINUX_DOWNLOAD_POLICY_H_
