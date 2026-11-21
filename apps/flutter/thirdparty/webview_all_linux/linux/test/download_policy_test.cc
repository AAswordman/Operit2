#include "webview/download_policy.h"

#include <glib/gstdio.h>
#include <libsoup/soup.h>

#include <memory>
#include <string>

namespace {

struct DownloadResult {
  GMainLoop *loop;
  std::string destination;
  bool destination_requested = false;
  bool cancelled = false;
  bool finished = false;
  bool timed_out = false;
};

class Fixture {
public:
  Fixture() {
    GError *error = nullptr;
    directory_ = g_dir_make_tmp("webview-all-download-test-XXXXXX", &error);
    g_assert_no_error(error);
    server_ = soup_server_new(nullptr, nullptr);
    soup_server_add_handler(
        server_, nullptr,
        [](SoupServer *, SoupServerMessage *message, const char *, GHashTable *,
           gpointer) {
          soup_server_message_set_status(message, SOUP_STATUS_OK, nullptr);
          soup_message_headers_append(
              soup_server_message_get_response_headers(message),
              "Content-Disposition", "attachment; filename=test.txt");
          soup_server_message_set_response(
              message, "text/plain", SOUP_MEMORY_STATIC, "download test", 13);
        },
        nullptr, nullptr);
    soup_server_listen_local(server_, 0, SOUP_SERVER_LISTEN_IPV4_ONLY, &error);
    g_assert_no_error(error);
    GSList *uris = soup_server_get_uris(server_);
    gchar *uri = g_uri_to_string(static_cast<GUri *>(uris->data));
    url_ = uri;
    g_free(uri);
    g_slist_free_full(uris, reinterpret_cast<GDestroyNotify>(g_uri_unref));
    context_ = webkit_web_context_new_ephemeral();
    first_ = WEBKIT_WEB_VIEW(webkit_web_view_new_with_context(context_));
    second_ = WEBKIT_WEB_VIEW(webkit_web_view_new_with_context(context_));
    g_object_ref_sink(first_);
    g_object_ref_sink(second_);
    first_policy_ = std::make_unique<DownloadPolicy>(first_);
    second_policy_ = std::make_unique<DownloadPolicy>(second_);
  }

  ~Fixture() {
    first_policy_.reset();
    second_policy_.reset();
    gtk_widget_destroy(GTK_WIDGET(first_));
    gtk_widget_destroy(GTK_WIDGET(second_));
    g_object_unref(first_);
    g_object_unref(second_);
    g_object_unref(context_);
    soup_server_disconnect(server_);
    g_object_unref(server_);
    g_assert_cmpint(g_rmdir(directory_), ==, 0);
    g_free(directory_);
  }

  void Check(WebKitWebView *view, bool allowed) {
    const std::string path = std::string(directory_) + "/download.txt";
    gchar *destination = g_filename_to_uri(path.c_str(), nullptr, nullptr);
    DownloadResult result{g_main_loop_new(nullptr, FALSE), destination};
    g_free(destination);
    WebKitDownload *download =
        view != nullptr
            ? webkit_web_view_download_uri(view, url_.c_str())
            : webkit_web_context_download_uri(context_, url_.c_str());
    g_signal_connect(download, "decide-destination",
                     G_CALLBACK(+[](WebKitDownload *download, const gchar *,
                                    gpointer data) -> gboolean {
                       auto *result = static_cast<DownloadResult *>(data);
                       result->destination_requested = true;
                       webkit_download_set_destination(
                           download, result->destination.c_str());
                       return TRUE;
                     }),
                     &result);
    g_signal_connect(
        download, "failed",
        G_CALLBACK(+[](WebKitDownload *, GError *error, gpointer data) {
          auto *result = static_cast<DownloadResult *>(data);
          g_assert_error(error, WEBKIT_DOWNLOAD_ERROR,
                         WEBKIT_DOWNLOAD_ERROR_CANCELLED_BY_USER);
          result->cancelled = true;
        }),
        &result);
    g_signal_connect(download, "finished",
                     G_CALLBACK(+[](WebKitDownload *, gpointer data) {
                       auto *result = static_cast<DownloadResult *>(data);
                       result->finished = true;
                       g_main_loop_quit(result->loop);
                     }),
                     &result);
    const guint timeout = g_timeout_add_seconds(
        10,
        +[](gpointer data) -> gboolean {
          auto *result = static_cast<DownloadResult *>(data);
          result->timed_out = true;
          g_main_loop_quit(result->loop);
          return G_SOURCE_REMOVE;
        },
        &result);
    g_main_loop_run(result.loop);
    g_assert_false(result.timed_out);
    g_source_remove(timeout);
    g_assert_true(result.finished);
    g_assert_cmpint(result.cancelled, ==, !allowed);
    g_assert_cmpint(result.destination_requested, ==, allowed);
    g_assert_cmpint(g_file_test(path.c_str(), G_FILE_TEST_EXISTS), ==, allowed);
    if (allowed) {
      gchar *contents = nullptr;
      gsize length = 0;
      g_assert_true(
          g_file_get_contents(path.c_str(), &contents, &length, nullptr));
      g_assert_cmpstr(contents, ==, "download test");
      g_free(contents);
      g_assert_cmpint(g_remove(path.c_str()), ==, 0);
    }
    g_object_unref(download);
    g_main_loop_unref(result.loop);
  }

  // Exercises the exact native response, staging path, completion, and cleanup.
  void CheckManaged() {
    GMainLoop *loop = g_main_loop_new(nullptr, FALSE);
    DownloadPolicy::Update latest;
    int running = 0;
    first_policy_->SetCallback([&](const DownloadPolicy::Update &update) {
      latest = update;
      if (update.state == "running") running++;
      if (update.state == "completed" || update.state == "failed") g_main_loop_quit(loop);
    });
    WebKitDownload *download = webkit_web_view_download_uri(first_, url_.c_str());
    const auto timeout = g_timeout_add_seconds(10, +[](gpointer value) -> gboolean {
      g_main_loop_quit(static_cast<GMainLoop *>(value));
      return G_SOURCE_REMOVE;
    }, loop);
    g_main_loop_run(loop);
    g_source_remove(timeout);
    g_assert_cmpstr(latest.state.c_str(), ==, "completed");
    g_assert_cmpstr(latest.filename.c_str(), ==, "test.txt");
    g_assert_cmpint(latest.received, ==, 13);
    g_assert_cmpint(running, >, 0);
    gchar *contents = nullptr;
    g_assert_true(g_file_get_contents(latest.path.c_str(), &contents, nullptr, nullptr));
    g_assert_cmpstr(contents, ==, "download test");
    g_free(contents);
    first_policy_->Release(latest.id);
    g_assert_false(g_file_test(latest.path.c_str(), G_FILE_TEST_EXISTS));
    first_policy_->SetCallback({});
    g_object_unref(download);
    g_main_loop_unref(loop);
  }

  WebKitWebView *first_;
  WebKitWebView *second_;
  std::unique_ptr<DownloadPolicy> first_policy_;
  std::unique_ptr<DownloadPolicy> second_policy_;

private:
  WebKitWebContext *context_;
  SoupServer *server_;
  gchar *directory_;
  std::string url_;
};

void TestScopeAndRuntimeChanges() {
  Fixture fixture;
  fixture.Check(fixture.first_, true);
  fixture.first_policy_->SetEnabled(false);
  fixture.Check(fixture.first_, false);
  fixture.Check(fixture.second_, true);
  fixture.first_policy_->SetEnabled(true);
  fixture.Check(fixture.first_, true);
  fixture.first_policy_->SetEnabled(false);
  fixture.Check(fixture.first_, false);
}

void TestCleanupAndContextDownloads() {
  Fixture fixture;
  fixture.first_policy_->SetEnabled(false);
  fixture.second_policy_->SetEnabled(false);
  fixture.Check(nullptr, true);
  fixture.first_policy_.reset();
  fixture.Check(fixture.first_, true);
  fixture.Check(fixture.second_, false);
}

// Verifies managed transfers without replacing the browser's HTTP request.
void TestManagedDownload() {
  Fixture fixture;
  fixture.CheckManaged();
}

} // namespace

int main(int argc, char **argv) {
  gtk_test_init(&argc, &argv, nullptr);
  g_test_add_func("/downloads/scope-and-runtime-changes",
                  TestScopeAndRuntimeChanges);
  g_test_add_func("/downloads/cleanup-and-context-downloads",
                  TestCleanupAndContextDownloads);
  g_test_add_func("/downloads/managed-original-response", TestManagedDownload);
  return g_test_run();
}
