#include "include/operit_keep_alive/operit_keep_alive_plugin.h"
#include <flutter/method_channel.h>
#include <flutter/plugin_registrar_windows.h>
#include <flutter/standard_method_codec.h>
#include <windows.h>
#include <shellapi.h>

#include <memory>
#include <optional>
#include <set>
#include <stdexcept>
#include <string>
#include <vector>
#include <cstdint>
#include <chrono>

namespace {
using Value = flutter::EncodableValue;
using Map = flutter::EncodableMap;
constexpr UINT kTrayMessage = WM_APP + 0x5b3;
constexpr UINT kTrayId = 1;
constexpr UINT kOpen = 1;
constexpr UINT kExit = 2;
constexpr UINT kWorkBase = 100;

/// Retains action identity independently of live updates while a native menu is open.
struct WorkItem {
  std::string id;
  Map activation;
  std::wstring title, subtitle, open, stop;
  bool can_stop;
  int64_t started;
  std::optional<int64_t> ended;
};

/// Decodes an explicit UTF-8 protocol string without substituting application text.
std::string String(const Map& arguments, const char* key) {
  const auto entry = arguments.find(Value(key));
  if (entry == arguments.end() || !std::holds_alternative<std::string>(entry->second)) {
    throw std::invalid_argument(std::string("Missing keep-alive argument: ") + key);
  }
  const auto& value = std::get<std::string>(entry->second);
  if (value.empty()) throw std::invalid_argument("Keep-alive strings must not be empty");
  return value;
}

/// Reads the two integer encodings defined by Flutter's standard message codec.
int64_t Integer(const Value& value) {
  if (const auto* int32Value = std::get_if<int32_t>(&value)) return *int32Value;
  if (const auto* int64Value = std::get_if<int64_t>(&value)) return *int64Value;
  throw std::invalid_argument("Expected work revision integer");
}

/// Converts validated protocol text to Windows UTF-16.
std::wstring Wide(const std::string& text) {
  const int length = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(),
                                        static_cast<int>(text.size()), nullptr, 0);
  if (length == 0) throw std::invalid_argument("Invalid keep-alive UTF-8 text");
  std::wstring value(length, L'\0');
  if (MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(),
                         static_cast<int>(text.size()), value.data(), length) == 0) {
    throw std::runtime_error("Unable to decode keep-alive text");
  }
  return value;
}

/// Owns the main window's tray icon without replacing its Flutter engine or Core.
class KeepAlivePlugin final : public flutter::Plugin {
 public:
  /// Installs only message routing until the main Dart entrypoint initializes residency.
  explicit KeepAlivePlugin(flutter::PluginRegistrarWindows* registrar)
      : registrar_(registrar),
        channel_(std::make_unique<flutter::MethodChannel<Value>>(
            registrar->messenger(), "operit/keep_alive", &flutter::StandardMethodCodec::GetInstance())) {
    delegate_ = registrar_->RegisterTopLevelWindowProcDelegate(
        [this](HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
          return WindowMessage(window, message, wparam, lparam);
        });
    channel_->SetMethodCallHandler(
        [this](const flutter::MethodCall<Value>& call,
               std::unique_ptr<flutter::MethodResult<Value>> result) {
          Handle(call, std::move(result));
        });
  }

  /// Releases native resources during plugin registrar destruction.
  ~KeepAlivePlugin() override {
    // Flutter has already detached the engine from its messenger at this point.
    // The registrar owns the messenger and releases its handlers after plugins.
    registrar_->UnregisterTopLevelWindowProcDelegate(delegate_);
    if (tray_added_) Shell_NotifyIconW(NIM_DELETE, &tray_);
  }

 private:
  /// Encodes the actual window-residency policy and explicit work leases.
  Map Report(const std::string& detail =
      "Closing the main window hides it to the tray and retains the Core engine.") const {
    return {{Value("protection"), Value(tray_added_ ? "continuous" : "unavailable")},
            {Value("closeToTray"), Value(initialized_ && tray_added_)},
            {Value("activeLeases"), Value(static_cast<int64_t>(leases_.size()))},
            {Value("detail"), Value(detail)}};
  }

  /// Publishes shell errors without treating failed tray registration as successful protection.
  void ReportFailure(const std::string& error) {
    channel_->InvokeMethod("protectionChanged", std::make_unique<Value>(Report(error)));
  }

  /// Registers the main window's own icon and uses the current shell notification protocol.
  void AddTray() {
    if (!Shell_NotifyIconW(NIM_ADD, &tray_)) throw std::runtime_error("Unable to register the system tray icon");
    tray_.uVersion = NOTIFYICON_VERSION_4;
    if (!Shell_NotifyIconW(NIM_SETVERSION, &tray_)) {
      Shell_NotifyIconW(NIM_DELETE, &tray_);
      throw std::runtime_error("Unable to configure the system tray protocol");
    }
    tray_added_ = true;
  }

  /// Binds each Dart lifetime to the retained engine window and its single tray icon.
  void Initialize(const Map& arguments) {
    auto* view = registrar_->GetView();
    if (view == nullptr) throw std::runtime_error("Keep-alive requires the main Flutter view");
    const auto window = GetAncestor(view->GetNativeWindow(), GA_ROOT);
    if (window == nullptr) throw std::runtime_error("Main window is unavailable");
    if (initialized_ && window != window_) throw std::logic_error("Keep-alive window ownership changed");
    const auto title = Wide(String(arguments, "applicationName"));
    const auto open = Wide(String(arguments, "openLabel"));
    const auto exit = Wide(String(arguments, "exitLabel"));
    const auto quit_message = Wide(String(arguments, "quitMessage"));
    const auto taskbar_created = RegisterWindowMessageW(L"TaskbarCreated");
    if (taskbar_created == 0) throw std::runtime_error("Unable to register shell lifecycle message");
    NOTIFYICONDATAW tray{};
    tray.cbSize = sizeof(tray);
    tray.hWnd = window;
    tray.uID = kTrayId;
    tray.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
    tray.uCallbackMessage = kTrayMessage;
    tray.hIcon = reinterpret_cast<HICON>(GetClassLongPtr(window, GCLP_HICON));
    if (tray.hIcon == nullptr) throw std::runtime_error("Main window has no application icon");
    if (title.size() >= sizeof(tray.szTip) / sizeof(wchar_t)) {
      throw std::invalid_argument("Application tray title is too long");
    }
    wcscpy_s(tray.szTip, title.c_str());
    if (initialized_) {
      if (!Shell_NotifyIconW(NIM_MODIFY, &tray)) throw std::runtime_error("Unable to update the system tray icon");
      tray.uVersion = NOTIFYICON_VERSION_4;
      tray_added_ = true;
    }
    window_ = window;
    title_ = title;
    open_ = open;
    exit_ = exit;
    quit_message_ = quit_message;
    taskbar_created_ = taskbar_created;
    tray_ = tray;
    if (!initialized_) AddTray();

    // Hot restart replaces Dart without destroying this plugin or releasing its leases.
    leases_.clear();
    work_items_.clear();
    work_summary_.clear();
    work_revision_ = -1;
    closing_ = false;
    initialized_ = true;
  }

  /// Restores the retained window and foregrounds its existing Flutter engine.
  void Open() {
    ShowWindow(window_, IsIconic(window_) ? SW_RESTORE : SW_SHOW);
    SetForegroundWindow(window_);
  }

  /// Requires an explicit user decision before closing a Core that still owns work leases.
  void Exit() {
    if (!leases_.empty() && MessageBoxW(window_, quit_message_.c_str(), title_.c_str(),
        MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2 | MB_SETFOREGROUND) != IDYES) return;
    if (!PostMessageW(window_, WM_CLOSE, 0, 0)) throw std::runtime_error("Unable to request application shutdown");
    closing_ = true;
  }

  /// Replaces the complete work revision and updates the tray's compact description.
  void PublishWork(const Map& arguments) {
    const auto revision = Integer(arguments.at(Value("revision")));
    if (revision <= work_revision_) return;
    const auto& items = std::get<flutter::EncodableList>(arguments.at(Value("items")));
    if (items.size() > 10000) throw std::invalid_argument("Too many work menu entries");
    std::vector<WorkItem> next;
    for (const auto& encoded : items) {
      const auto& item = std::get<Map>(encoded);
      next.push_back({String(item, "workId"), std::get<Map>(item.at(Value("activation"))), Wide(String(item, "title")),
          Wide(String(item, "subtitle")), Wide(String(item, "openLabel")), Wide(String(item, "stopLabel")),
          std::get<bool>(item.at(Value("canStop"))), Integer(item.at(Value("startedAtMillis"))),
          std::holds_alternative<std::monostate>(item.at(Value("endedAtMillis"))) ? std::nullopt : std::optional<int64_t>(Integer(item.at(Value("endedAtMillis"))))});
    }
    const auto summary = Wide(String(arguments, "summary"));
    const auto tooltip = title_ + L" · " + summary;
    wcsncpy_s(tray_.szTip, tooltip.c_str(), _TRUNCATE);
    if (!Shell_NotifyIconW(NIM_MODIFY, &tray_)) throw std::runtime_error("Unable to update work tray text");
    work_items_ = std::move(next);
    work_summary_ = summary;
    work_revision_ = revision;
  }

  /// Computes visible duration when a menu opens and freezes completed work at its actual end.
  static std::wstring Elapsed(const WorkItem& work) {
    const auto ended = work.ended.has_value() ? *work.ended : std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::system_clock::now().time_since_epoch()).count();
    const auto seconds = (ended - work.started) / 1000;
    wchar_t text[64];
    swprintf_s(text, L"%02lld:%02lld", static_cast<long long>(seconds / 60), static_cast<long long>(seconds % 60));
    return text;
  }

  /// Emits one explicit system action against the menu snapshot selected by the user.
  void WorkAction(const WorkItem& item, const char* action) {
    if (std::string(action) == "open") Open();
    channel_->InvokeMethod("workAction", std::make_unique<Value>(Map{
        {Value("workId"), Value(item.id)}, {Value("activation"), Value(item.activation)}, {Value("action"), Value(action)}}));
  }

  /// Presents native open and exit actions through the shell tray menu.
  void Menu() {
    const auto works = work_items_;
    HMENU menu = CreatePopupMenu();
    if (menu == nullptr) throw std::runtime_error("Unable to create tray menu");
    if (!AppendMenuW(menu, MF_STRING, kOpen, open_.c_str()) ||
        !AppendMenuW(menu, MF_STRING, kExit, exit_.c_str())) {
      DestroyMenu(menu);
      throw std::runtime_error("Unable to populate tray menu");
    }
    if (!work_summary_.empty()) {
      AppendMenuW(menu, MF_SEPARATOR, 0, nullptr);
      AppendMenuW(menu, MF_STRING | MF_DISABLED, 0, work_summary_.c_str());
    }
    for (size_t i = 0; i < works.size(); ++i) {
      HMENU details = CreatePopupMenu();
      if (details == nullptr) { DestroyMenu(menu); throw std::runtime_error("Unable to create work menu"); }
      AppendMenuW(details, MF_STRING | MF_DISABLED, 0, (works[i].subtitle + L" · " + Elapsed(works[i])).c_str());
      AppendMenuW(details, MF_STRING, kWorkBase + static_cast<UINT>(i) * 2, works[i].open.c_str());
      if (works[i].can_stop) AppendMenuW(details, MF_STRING, kWorkBase + static_cast<UINT>(i) * 2 + 1, works[i].stop.c_str());
      if (!AppendMenuW(menu, MF_POPUP, reinterpret_cast<UINT_PTR>(details), works[i].title.c_str())) {
        DestroyMenu(details); DestroyMenu(menu); throw std::runtime_error("Unable to append work menu");
      }
    }
    POINT cursor{};
    if (!GetCursorPos(&cursor)) {
      DestroyMenu(menu);
      throw std::runtime_error("Unable to locate tray menu");
    }
    SetForegroundWindow(window_);
    const UINT selected = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON,
        cursor.x, cursor.y, 0, window_, nullptr);
    DestroyMenu(menu);
    PostMessageW(window_, WM_NULL, 0, 0);
    if (selected == kOpen) Open();
    if (selected == kExit) Exit();
    if (selected >= kWorkBase) {
      const auto index = (selected - kWorkBase) / 2;
      if (index >= works.size()) throw std::runtime_error("Invalid work menu selection");
      WorkAction(works[index], (selected - kWorkBase) % 2 == 0 ? "open" : "stop");
    }
  }

  /// Handles close and shell lifecycle messages exclusively for the main owner window.
  std::optional<LRESULT> WindowMessage(HWND window, UINT message, WPARAM, LPARAM lparam) {
    if (!initialized_ || window != window_) return std::nullopt;
    try {
      if (message == WM_CLOSE && !closing_) {
        if (!tray_added_) throw std::runtime_error("Tray icon is unavailable; the Core window must remain open");
        ShowWindow(window_, SW_HIDE);
        return 0;
      }
      if (message == taskbar_created_) {
        tray_added_ = false;
        AddTray();
        channel_->InvokeMethod("protectionChanged", std::make_unique<Value>(Report()));
        return 0;
      }
      if (message == kTrayMessage) {
        const UINT action = LOWORD(lparam);
        if (action == WM_LBUTTONUP || action == NIN_SELECT || action == NIN_KEYSELECT) Open();
        if (action == WM_CONTEXTMENU) Menu();
        return 0;
      }
    } catch (const std::exception& error) {
      Open();
      ReportFailure(error.what());
      MessageBoxW(window_, Wide(error.what()).c_str(), title_.c_str(), MB_OK | MB_ICONERROR);
      return 0;
    }
    return std::nullopt;
  }

  /// Validates the common protocol and acknowledges only committed lease mutations.
  void Handle(const flutter::MethodCall<Value>& call, std::unique_ptr<flutter::MethodResult<Value>> result) {
    try {
      const auto* arguments = call.arguments() == nullptr ? nullptr : std::get_if<Map>(call.arguments());
      if (arguments == nullptr) throw std::invalid_argument("Keep-alive requires an argument map");
      const auto& method = call.method_name();
      if (method == "workCapabilities") {
        result->Success(Value(Map{{Value("updatesOwner"), Value("client")}, {Value("surface"), Value("tray")}, {Value("liveUpdates"), Value(false)}}));
        return;
      }
      if (method == "workActionsReady") { result->Success(); return; }
      if (method == "publishWork") {
        if (!initialized_) throw std::logic_error("Keep-alive is not initialized");
        PublishWork(*arguments);
        result->Success();
        return;
      }
      if (method == "initialize") {
        Initialize(*arguments);
      } else if (method == "acquire" || method == "release") {
        if (!initialized_) throw std::logic_error("Keep-alive is not initialized");
        const auto id = String(*arguments, "id");
        if (method == "acquire") {
          String(*arguments, "reason");
          if (!leases_.insert(id).second) throw std::invalid_argument("Duplicate keep-alive lease");
        } else {
          if (leases_.erase(id) != 1) throw std::invalid_argument("Unknown keep-alive lease");
        }
      } else {
        result->NotImplemented();
        return;
      }
      result->Success(Value(Report()));
    } catch (const std::exception& error) {
      result->Error("KEEP_ALIVE_ERROR", error.what());
    }
  }

  flutter::PluginRegistrarWindows* registrar_;
  std::unique_ptr<flutter::MethodChannel<Value>> channel_;
  int delegate_ = -1;
  HWND window_ = nullptr;
  UINT taskbar_created_ = 0;
  NOTIFYICONDATAW tray_{};
  bool tray_added_ = false;
  bool initialized_ = false;
  bool closing_ = false;
  std::wstring title_, open_, exit_, quit_message_;
  std::set<std::string> leases_;
  int64_t work_revision_ = -1;
  std::vector<WorkItem> work_items_;
  std::wstring work_summary_;
};
}  // namespace

/// Gives Flutter ownership of the per-engine host while its Core remains in process.
void OperitKeepAlivePluginRegisterWithRegistrar(FlutterDesktopPluginRegistrarRef registrar) {
  auto* windows = flutter::PluginRegistrarManager::GetInstance()
      ->GetRegistrar<flutter::PluginRegistrarWindows>(registrar);
  windows->AddPlugin(std::make_unique<KeepAlivePlugin>(windows));
}
