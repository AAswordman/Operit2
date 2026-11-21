#include <windows.h>

#include <cstdlib>
#include <iostream>
#include <map>
#include <thread>

#include "record_windows_plugin.h"

using Value = flutter::EncodableValue;
using Map = flutter::EncodableMap;
using record_windows::RecordWindowsPlugin;
using record_windows::WM_RUN_DELEGATE;

namespace {

/// Fails native lifecycle checks in optimized test builds too.
void Require(bool condition, const char* message) {
  if (!condition) {
    std::cerr << message << std::endl;
    std::exit(1);
  }
}

/// Observes actual production channel registration and encoded Dart messages.
class Messenger : public flutter::BinaryMessenger {
 public:
  bool alive = true;
  std::map<std::string, flutter::BinaryMessageHandler> handlers;
  mutable std::vector<std::string> events;

  /// Rejects communication through an engine whose lifetime has ended.
  void Send(const std::string& channel, const uint8_t*, size_t,
            flutter::BinaryReply = nullptr) const override {
    Require(alive, "Sent an event through a destroyed engine");
    events.push_back(channel);
  }

  /// Keeps each engine's stream and method registrations observable.
  void SetMessageHandler(const std::string& channel,
                         flutter::BinaryMessageHandler handler) override {
    Require(alive, "Registered a handler through a destroyed engine");
    if (handler) handlers[channel] = std::move(handler);
    else handlers.erase(channel);
  }

  /// Calls the production plugin through Flutter's real binary method codec.
  void Call(const std::string& channel, const std::string& method, Map arguments,
            bool succeeds = true) {
    Require(handlers.count(channel) == 1, "Method channel belongs to another engine");
    auto handler = handlers.at(channel);
    auto bytes = flutter::StandardMethodCodec::GetInstance().EncodeMethodCall(
        flutter::MethodCall<Value>(method, std::make_unique<Value>(std::move(arguments))));
    bool replied = false;
    handler(bytes->data(), bytes->size(), [&](const uint8_t* reply, size_t size) {
      Require(size > 0 && (reply[0] == 0) == succeeds, "Unexpected method result");
      replied = true;
    });
    Require(replied, "Method did not complete synchronously");
  }
};

/// Hosts one real plugin instance without starting Dart or opening a microphone.
class Engine {
 public:
  Messenger messenger;
  HWND window;
  record_windows::WindowProcDelegate delegate;
  std::unique_ptr<RecordWindowsPlugin> plugin;

  /// Supplies independent channel, window, and procedure ownership per engine.
  Engine() {
    window = CreateWindowW(L"STATIC", L"", 0, 0, 0, 1, 1,
                           HWND_MESSAGE, nullptr, nullptr, nullptr);
    Require(window != nullptr, "Unable to create an engine window");
    plugin = std::make_unique<RecordWindowsPlugin>(
        &messenger,
        [this](auto callback) { delegate = std::move(callback); return 1; },
        [this](int id) { Require(id == 1, "Wrong procedure removed"); delegate = nullptr; },
        window);
  }

  /// Stops transport before destroying the engine and its registrar-owned plugin.
  ~Engine() { Close(); }

  /// Models child engine teardown and verifies complete channel unregistration.
  void Close() {
    if (!plugin) return;
    plugin->Shutdown();
    plugin->Shutdown();
    Require(messenger.handlers.empty(), "Engine teardown left registered channels");
    Require(!delegate, "Engine teardown left its procedure installed");
    messenger.alive = false;
    plugin.reset();
    DestroyWindow(window);
  }

  /// Sends recorder operations with the same identity contract as Dart.
  void Call(const std::string& method, const std::string& id, bool succeeds = true) {
    messenger.Call("com.llfbandit.record/messages", method,
                   {{Value("recorderId"), Value(id)}}, succeeds);
  }

  /// Attaches a real event sink through the production stream handler.
  void Listen(const std::string& id) {
    messenger.Call("com.llfbandit.record/events/" + id, "listen", {});
  }

  /// Delivers only messages addressed to this engine's platform window.
  void Pump() {
    MSG message{};
    while (PeekMessage(&message, window, WM_RUN_DELEGATE, WM_RUN_DELEGATE, PM_REMOVE)) {
      Require(delegate(window, message.message, message.wParam, message.lParam).has_value(),
              "Owner did not consume its callback");
    }
  }
};

/// Covers secondary-engine creation, closure, and callbacks crossing recorder disposal.
void CheckEngineOwnership() {
  Engine main;
  main.Call("create", "same");
  main.Listen("same");
  Engine child;
  child.Call("create", "same");
  child.Listen("same");

  main.Call("create", "while-child-alive");
  Require(main.messenger.handlers.count("com.llfbandit.record/events/while-child-alive") == 1,
          "Main recorder registered its stream on the child engine");
  Require(child.messenger.handlers.count("com.llfbandit.record/events/while-child-alive") == 0,
          "Child engine received another engine's stream");
  main.Call("stop", "same");
  child.Pump();
  Require(child.messenger.events.empty() && main.messenger.events.empty(),
          "Child engine consumed the main engine's callback");
  main.Pump();
  Require(main.messenger.events.size() == 1, "Main engine did not receive its state event");

  child.Call("stop", "same");
  child.Close();
  main.Call("create", "after-child-close");
  main.Call("stop", "same");
  main.Call("dispose", "same");
  main.Pump();
  Require(main.messenger.events.size() == 1, "Disposed stream received a queued event");
  Require(main.messenger.handlers.count("com.llfbandit.record/events/same") == 0,
          "Disposed recorder left a stream handler registered");
  main.Call("create", "same");
  main.Call("create", "same", false);
  main.Listen("same");
  main.Call("stop", "same");
  main.Pump();
  Require(main.messenger.events.size() == 2, "Recreated recorder lost its stream");
}

/// Checks that worker callbacks cannot outlive the owning engine's dispatch queue.
void CheckDispatchCancellation() {
  Engine engine;
  const HWND view = CreateWindowW(L"STATIC", L"", WS_CHILD, 0, 0, 1, 1,
                                  HWND_MESSAGE, nullptr, nullptr, nullptr);
  Require(view != nullptr, "Unable to create a detached Flutter view");
  auto dispatcher = std::make_shared<record_windows::MainThreadDispatcher>(view);
  SetParent(view, engine.window);
  Require(dispatcher->window() == engine.window, "Dispatch did not follow view attachment");
  int calls = 0;
  Require(dispatcher->Post([&] { ++calls; }), "Live engine rejected callback");
  MSG message{};
  Require(PeekMessage(&message, engine.window, WM_RUN_DELEGATE, WM_RUN_DELEGATE, PM_REMOVE),
          "Callback was not posted to the attached view's owner");
  dispatcher->Close();
  std::thread worker([&] {
    Require(!dispatcher->Post([&] { ++calls; }), "Closed engine accepted worker callback");
  });
  worker.join();
  dispatcher->RunNext();
  Require(calls == 0, "Closed engine executed a cancelled callback");
  DestroyWindow(view);
}

}  // namespace

/// Executes the production recording plugin lifecycle without hardware recording.
int main() {
  CheckEngineOwnership();
  CheckDispatchCancellation();
  std::cout << "Windows recording engine ownership checks passed" << std::endl;
}
