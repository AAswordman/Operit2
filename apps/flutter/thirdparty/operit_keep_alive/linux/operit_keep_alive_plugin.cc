#include "include/operit_keep_alive/operit_keep_alive_plugin.h"
#include <libayatana-appindicator/app-indicator.h>
#include <set>
#include <stdexcept>
#include <string>
#include <cstdint>
#include <chrono>
#include <optional>
#include <cstdio>

namespace {
/// Owns a retained GTK window and its explicit StatusNotifier representation.
class Residency {
 public:
  /// Defers native UI ownership until the main Dart entrypoint initializes it.
  explicit Residency(FlView* view) : view_(view) {
    g_object_add_weak_pointer(G_OBJECT(view_), reinterpret_cast<gpointer*>(&view_));
  }

  /// Disconnects callbacks before releasing window, application and tray resources.
  ~Residency() {
    if (view_ != nullptr) g_object_remove_weak_pointer(G_OBJECT(view_), reinterpret_cast<gpointer*>(&view_));
    if (window_ != nullptr) {
      g_signal_handlers_disconnect_by_data(window_, this);
      g_object_unref(window_);
    }
    if (indicator_ != nullptr) {
      g_signal_handlers_disconnect_by_data(indicator_, this);
      app_indicator_set_status(indicator_, APP_INDICATOR_STATUS_PASSIVE);
      g_object_unref(indicator_);
    }
    if (menu_ != nullptr) { gtk_widget_destroy(menu_); g_object_unref(menu_); }
    if (application_ != nullptr) {
      g_application_release(G_APPLICATION(application_));
      g_object_unref(application_);
    }
  }

  /// Requires exact string arguments from the shared protocol.
  static std::string String(FlValue* map, const char* key) {
    FlValue* value = fl_value_lookup_string(map, key);
    if (value == nullptr || fl_value_get_type(value) != FL_VALUE_TYPE_STRING ||
        *fl_value_get_string(value) == '\0') throw std::invalid_argument(std::string("Missing argument: ") + key);
    return fl_value_get_string(value);
  }

  /// Checks that the desktop session has a real host capable of displaying its tray.
  static void RequireTrayHost() {
    g_autoptr(GError) error = nullptr;
    g_autoptr(GDBusConnection) bus = g_bus_get_sync(G_BUS_TYPE_SESSION, nullptr, &error);
    if (bus == nullptr) throw std::runtime_error(error->message);
    g_autoptr(GVariant) response = g_dbus_connection_call_sync(bus,
        "org.kde.StatusNotifierWatcher", "/StatusNotifierWatcher", "org.freedesktop.DBus.Properties", "Get",
        g_variant_new("(ss)", "org.kde.StatusNotifierWatcher", "IsStatusNotifierHostRegistered"),
        G_VARIANT_TYPE("(v)"), G_DBUS_CALL_FLAGS_NONE, 1000, nullptr, &error);
    if (response == nullptr) throw std::runtime_error(error->message);
    g_autoptr(GVariant) boxed = g_variant_get_child_value(response, 0);
    g_autoptr(GVariant) present = g_variant_get_variant(boxed);
    if (!g_variant_is_of_type(present, G_VARIANT_TYPE_BOOLEAN) || !g_variant_get_boolean(present)) {
      throw std::runtime_error("This desktop session has no StatusNotifier tray host");
    }
  }

  /// Restores the original retained window and Flutter engine.
  static void Open(GtkMenuItem*, gpointer data) {
    auto* self = static_cast<Residency*>(data);
    gtk_window_present(self->window_);
  }

  /// Confirms an explicit process exit while Core execution leases remain active.
  static void Exit(GtkMenuItem*, gpointer data) {
    auto* self = static_cast<Residency*>(data);
    if (!self->leases_.empty()) {
      GtkWidget* dialog = gtk_message_dialog_new(self->window_, GTK_DIALOG_MODAL,
          GTK_MESSAGE_QUESTION, GTK_BUTTONS_OK_CANCEL, "%s", self->quit_message_.c_str());
      const int response = gtk_dialog_run(GTK_DIALOG(dialog));
      gtk_widget_destroy(dialog);
      if (response != GTK_RESPONSE_OK) return;
    }
    g_application_quit(G_APPLICATION(self->application_));
  }

  /// Hides the window only when a visible tray supplies a route back into the app.
  static gboolean Close(GtkWidget*, GdkEvent*, gpointer data) {
    auto* self = static_cast<Residency*>(data);
    try {
      RequireTrayHost();
      if (!self->available_) throw std::runtime_error("The tray has not acknowledged the application icon");
      gtk_widget_hide(GTK_WIDGET(self->window_));
    } catch (const std::exception& error) {
      self->available_ = false;
      g_autoptr(FlValue) report = self->Report(error.what());
      fl_method_channel_invoke_method(self->channel_, "protectionChanged", report, nullptr, nullptr, nullptr);
      GtkWidget* dialog = gtk_message_dialog_new(self->window_, GTK_DIALOG_MODAL,
          GTK_MESSAGE_ERROR, GTK_BUTTONS_CLOSE, "%s", error.what());
      gtk_dialog_run(GTK_DIALOG(dialog));
      gtk_widget_destroy(dialog);
    }
    return TRUE;
  }

  /// Restores access to the retained engine whenever the desktop removes its tray host.
  static void ConnectionChanged(AppIndicator*, gboolean connected, gpointer data) {
    auto* self = static_cast<Residency*>(data);
    self->available_ = connected;
    if (!connected) gtk_window_present(self->window_);
    g_autoptr(FlValue) report = self->Report();
    fl_method_channel_invoke_method(self->channel_, "protectionChanged", report, nullptr, nullptr, nullptr);
  }

  /// Creates the tray and retains exactly the main window's application instance.
  void Initialize(FlValue* arguments) {
    if (window_ != nullptr) return;
    const auto title = String(arguments, "applicationName");
    const auto open = String(arguments, "openLabel");
    const auto exit = String(arguments, "exitLabel");
    quit_message_ = String(arguments, "quitMessage");
    if (view_ == nullptr) throw std::runtime_error("Flutter view is unavailable");
    GtkWidget* root = gtk_widget_get_toplevel(GTK_WIDGET(view_));
    if (!GTK_IS_WINDOW(root)) throw std::runtime_error("Main GTK window is unavailable");
    GtkApplication* application = gtk_window_get_application(GTK_WINDOW(root));
    if (application == nullptr) throw std::runtime_error("Main GTK application is unavailable");
    RequireTrayHost();
    window_ = GTK_WINDOW(g_object_ref(root));
    application_ = GTK_APPLICATION(g_object_ref(application));
    g_application_hold(G_APPLICATION(application_));
    menu_ = gtk_menu_new();
    g_object_ref_sink(menu_);
    GtkWidget* open_item = gtk_menu_item_new_with_label(open.c_str());
    GtkWidget* exit_item = gtk_menu_item_new_with_label(exit.c_str());
    gtk_menu_shell_append(GTK_MENU_SHELL(menu_), open_item);
    gtk_menu_shell_append(GTK_MENU_SHELL(menu_), exit_item);
    g_signal_connect(open_item, "activate", G_CALLBACK(Open), this);
    g_signal_connect(exit_item, "activate", G_CALLBACK(Exit), this);
    gtk_widget_show_all(menu_);
    indicator_ = APP_INDICATOR(g_object_new(APP_INDICATOR_TYPE,
        "id", "operit-core-runtime", "icon-name", "application-x-executable",
        "category", "ApplicationStatus", nullptr));
    g_signal_connect(indicator_, "connection-changed", G_CALLBACK(ConnectionChanged), this);
    app_indicator_set_title(indicator_, title.c_str());
    app_indicator_set_menu(indicator_, GTK_MENU(menu_));
    app_indicator_set_status(indicator_, APP_INDICATOR_STATUS_ACTIVE);
    g_signal_connect(window_, "delete-event", G_CALLBACK(Close), this);
  }

  /// Returns the actual tray capability and acknowledged work count.
  FlValue* Report(const char* failure = nullptr) const {
    FlValue* result = fl_value_new_map();
    fl_value_set_string_take(result, "protection", fl_value_new_string(available_ ? "continuous" : "unavailable"));
    fl_value_set_string_take(result, "closeToTray", fl_value_new_bool(available_));
    fl_value_set_string_take(result, "activeLeases", fl_value_new_int(static_cast<int64_t>(leases_.size())));
    const char* detail = failure;
    if (detail == nullptr) {
      detail = available_ ? "GTK retains the main engine behind the StatusNotifier tray; OS shutdown still ends execution."
                          : "The desktop has not acknowledged a visible tray icon; the main window remains accessible.";
    }
    fl_value_set_string_take(result, "detail", fl_value_new_string(detail));
    return result;
  }

  /// Retains immutable action data for each native work menu entry.
  struct WorkMenuAction {
    Residency* owner;
    std::string id, action;
    FlValue* activation;
    /// Releases the retained opaque activation when its native menu action disappears.
    ~WorkMenuAction() { fl_value_unref(activation); }
  };

  /// Releases an action payload when GTK destroys its menu item.
  static void DestroyWorkAction(gpointer data, GClosure*) { delete static_cast<WorkMenuAction*>(data); }

  /// Delivers an explicit work action through the shared plugin channel.
  static void ActivateWork(GtkMenuItem*, gpointer data) {
    const auto* action = static_cast<WorkMenuAction*>(data);
    if (action->action == "open") gtk_window_present(action->owner->window_);
    g_autoptr(FlValue) value = fl_value_new_map();
    fl_value_set_string_take(value, "workId", fl_value_new_string(action->id.c_str()));
    fl_value_set_string(value, "activation", action->activation);
    fl_value_set_string_take(value, "action", fl_value_new_string(action->action.c_str()));
    fl_method_channel_invoke_method(action->owner->channel_, "workAction", value, nullptr, nullptr, nullptr);
  }

  /// Creates one immutable menu action using the explicit application-provided label.
  void AppendWorkAction(GtkWidget* menu, FlValue* work, const char* label, const char* action) {
    GtkWidget* item = gtk_menu_item_new_with_label(String(work, label).c_str());
    FlValue* activation = fl_value_lookup_string(work, "activation");
    if (activation == nullptr || fl_value_get_type(activation) != FL_VALUE_TYPE_MAP) throw std::invalid_argument("Invalid work activation");
    auto* payload = new WorkMenuAction{this, String(work, "workId"), action, fl_value_ref(activation)};
    g_signal_connect_data(item, "activate", G_CALLBACK(ActivateWork), payload, DestroyWorkAction, G_CONNECT_DEFAULT);
    gtk_menu_shell_append(GTK_MENU_SHELL(menu), item);
  }

  /// Retains only the timestamps needed while a native work submenu is visible.
  struct WorkClock { GtkWidget* status; std::string subtitle; int64_t started; std::optional<int64_t> ended; };

  /// Releases the display clock with its owning submenu.
  static void DestroyWorkClock(gpointer data, GClosure*) { delete static_cast<WorkClock*>(data); }

  /// Refreshes elapsed time at menu opening without introducing a background poller.
  static void RefreshWorkClock(GtkWidget*, gpointer data) {
    auto* clock = static_cast<WorkClock*>(data);
    const auto ended = clock->ended.has_value() ? *clock->ended : std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::system_clock::now().time_since_epoch()).count();
    const auto seconds = (ended - clock->started) / 1000;
    char duration[64];
    std::snprintf(duration, sizeof(duration), "%02lld:%02lld", static_cast<long long>(seconds / 60), static_cast<long long>(seconds % 60));
    gtk_menu_item_set_label(GTK_MENU_ITEM(clock->status), (clock->subtitle + " · " + duration).c_str());
  }

  /// Replaces the complete native work section without retaining Flutter route objects.
  void PublishWork(FlValue* arguments) {
    FlValue* revision = fl_value_lookup_string(arguments, "revision");
    FlValue* items = fl_value_lookup_string(arguments, "items");
    if (revision == nullptr || fl_value_get_type(revision) != FL_VALUE_TYPE_INT ||
        items == nullptr || fl_value_get_type(items) != FL_VALUE_TYPE_LIST) throw std::invalid_argument("Invalid work presentation");
    const auto next = fl_value_get_int(revision);
    if (next <= work_revision_) return;
    if (menu_ == nullptr) throw std::logic_error("Work tray is not initialized");
    const auto summary = String(arguments, "summary");
    GtkWidget* section = gtk_menu_item_new_with_label(summary.c_str());
    GtkWidget* list = gtk_menu_new();
    gtk_menu_item_set_submenu(GTK_MENU_ITEM(section), list);
    for (size_t index = 0; index < fl_value_get_length(items); ++index) {
      FlValue* work = fl_value_get_list_value(items, index);
      GtkWidget* entry = gtk_menu_item_new_with_label(String(work, "title").c_str());
      GtkWidget* details = gtk_menu_new();
      GtkWidget* status = gtk_menu_item_new_with_label(String(work, "subtitle").c_str());
      FlValue* started = fl_value_lookup_string(work, "startedAtMillis");
      FlValue* ended = fl_value_lookup_string(work, "endedAtMillis");
      if (started == nullptr || fl_value_get_type(started) != FL_VALUE_TYPE_INT || ended == nullptr ||
          (fl_value_get_type(ended) != FL_VALUE_TYPE_INT && fl_value_get_type(ended) != FL_VALUE_TYPE_NULL)) {
        gtk_widget_destroy(section); throw std::invalid_argument("Invalid work timestamps");
      }
      auto* clock = new WorkClock{status, String(work, "subtitle"), fl_value_get_int(started),
          fl_value_get_type(ended) == FL_VALUE_TYPE_NULL ? std::nullopt : std::optional<int64_t>(fl_value_get_int(ended))};
      g_signal_connect_data(details, "show", G_CALLBACK(RefreshWorkClock), clock, DestroyWorkClock, G_CONNECT_DEFAULT);
      gtk_widget_set_sensitive(status, FALSE);
      gtk_menu_shell_append(GTK_MENU_SHELL(details), status);
      AppendWorkAction(details, work, "openLabel", "open");
      FlValue* can_stop = fl_value_lookup_string(work, "canStop");
      if (can_stop == nullptr || fl_value_get_type(can_stop) != FL_VALUE_TYPE_BOOL) {
        gtk_widget_destroy(section); throw std::invalid_argument("Invalid work stop capability");
      }
      if (fl_value_get_bool(can_stop)) AppendWorkAction(details, work, "stopLabel", "stop");
      gtk_menu_item_set_submenu(GTK_MENU_ITEM(entry), details);
      gtk_menu_shell_append(GTK_MENU_SHELL(list), entry);
    }
    if (work_section_ != nullptr) gtk_widget_destroy(work_section_);
    work_section_ = section;
    gtk_menu_shell_prepend(GTK_MENU_SHELL(menu_), section);
    gtk_widget_show_all(section);
    app_indicator_set_title(indicator_, summary.c_str());
    work_revision_ = next;
  }

  /// Applies validated lease operations without substituting another tray mechanism.
  static void Handle(FlMethodChannel*, FlMethodCall* call, gpointer data) {
    auto* self = static_cast<Residency*>(data);
    try {
      FlValue* args = fl_method_call_get_args(call);
      if (args == nullptr || fl_value_get_type(args) != FL_VALUE_TYPE_MAP) throw std::invalid_argument("Expected argument map");
      const std::string method = fl_method_call_get_name(call);
      if (method == "workCapabilities") {
        g_autoptr(FlValue) caps = fl_value_new_map();
        fl_value_set_string_take(caps, "updatesOwner", fl_value_new_string("client"));
        fl_value_set_string_take(caps, "surface", fl_value_new_string("tray"));
        fl_value_set_string_take(caps, "liveUpdates", fl_value_new_bool(FALSE));
        fl_method_call_respond_success(call, caps, nullptr);
        return;
      }
      if (method == "workActionsReady") { fl_method_call_respond_success(call, nullptr, nullptr); return; }
      if (method == "publishWork") {
        self->PublishWork(args);
        fl_method_call_respond_success(call, nullptr, nullptr);
        return;
      }
      if (method == "initialize") self->Initialize(args);
      else if (method == "acquire" || method == "release") {
        if (self->window_ == nullptr) throw std::logic_error("Keep-alive is not initialized");
        const auto id = String(args, "id");
        if (method == "acquire") {
          String(args, "reason");
          if (!self->leases_.insert(id).second) throw std::invalid_argument("Duplicate lease");
        } else if (self->leases_.erase(id) != 1) throw std::invalid_argument("Unknown lease");
      } else {
        fl_method_call_respond_not_implemented(call, nullptr);
        return;
      }
      g_autoptr(FlValue) report = self->Report();
      fl_method_call_respond_success(call, report, nullptr);
    } catch (const std::exception& error) {
      fl_method_call_respond_error(call, "KEEP_ALIVE_ERROR", error.what(), nullptr, nullptr);
    }
  }

  FlMethodChannel* channel_ = nullptr;
 private:
  FlView* view_;
  GtkWindow* window_ = nullptr;
  GtkApplication* application_ = nullptr;
  GtkWidget* menu_ = nullptr;
  AppIndicator* indicator_ = nullptr;
  bool available_ = false;
  std::string quit_message_;
  std::set<std::string> leases_;
  GtkWidget* work_section_ = nullptr;
  int64_t work_revision_ = -1;
};

/// Releases per-engine state when Flutter unregisters its channel handler.
void Destroy(gpointer data) { delete static_cast<Residency*>(data); }
}  // namespace

/// Registers only transport; detached engines never create their own trays.
void operit_keep_alive_plugin_register_with_registrar(FlPluginRegistrar* registrar) {
  auto* residency = new Residency(fl_plugin_registrar_get_view(registrar));
  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  g_autoptr(FlMethodChannel) channel = fl_method_channel_new(fl_plugin_registrar_get_messenger(registrar),
      "operit/keep_alive", FL_METHOD_CODEC(codec));
  residency->channel_ = channel;
  fl_method_channel_set_method_call_handler(channel, Residency::Handle, residency, Destroy);
}
