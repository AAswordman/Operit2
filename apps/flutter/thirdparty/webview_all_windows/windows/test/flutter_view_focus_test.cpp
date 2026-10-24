// Runs against real Win32 window relationships without changing desktop focus.
// Build in a Visual Studio developer shell:
// clang-cl /std:c++20 /EHsc flutter_view_focus_test.cpp /link user32.lib comctl32.lib
#include <windows.h>
#include <commctrl.h>

#include <cstdio>
#include <cstdlib>

namespace {
HWND simulated_focus = nullptr;

/// Supplies controlled focus state without activating a test window.
HWND TestGetFocus() { return simulated_focus; }
}  // namespace

#define GetFocus TestGetFocus
#include "../src/plugin/flutter_view_focus.h"
#undef GetFocus

namespace {
constexpr UINT_PTR kObserverId = 1;
constexpr UINT_PTR kFocusHostId = 2;
int lost_focus_count = 0;
bool subclass_removed_on_destroy = false;

/// Stops the check immediately when a native focus invariant is violated.
void Require(bool condition, const char* message) {
  if (!condition) {
    std::fprintf(stderr, "FAIL: %s\n", message);
    std::exit(1);
  }
}

/// Observes only messages that reach the Flutter window procedure.
LRESULT CALLBACK ObserveFlutterFocus(HWND hwnd, UINT message, WPARAM wparam,
                                    LPARAM lparam, UINT_PTR subclass_id,
                                    DWORD_PTR) {
  if (message == WM_KILLFOCUS) ++lost_focus_count;
  if (message == WM_NCDESTROY) {
    DWORD_PTR reference_data = 0;
    subclass_removed_on_destroy = !GetWindowSubclass(
        hwnd, webview_all_windows::FlutterViewFocusProc, kFocusHostId,
        &reference_data);
    RemoveWindowSubclass(hwnd, ObserveFlutterFocus, subclass_id);
  }
  return DefSubclassProc(hwnd, message, wparam, lparam);
}

/// Delivers a native focus transfer and checks its visibility to Flutter.
void ExpectFocusTransfer(HWND host, HWND target, int expected_count,
                         const char* message) {
  SendMessage(host, WM_KILLFOCUS, reinterpret_cast<WPARAM>(target), 0);
  Require(lost_focus_count == expected_count, message);
  std::printf("PASS: %s\n", message);
}

/// Checks a browser focus notification against real HWND ancestry.
void ExpectBrowserDeparture(HWND host, HWND target, int expected_count,
                            const char* message) {
  simulated_focus = target;
  webview_all_windows::NotifyFlutterViewFocusLost(host);
  Require(lost_focus_count == expected_count, message);
  std::printf("PASS: %s\n", message);
}
}  // namespace

/// Exercises internal transfers, external departures, and subclass teardown.
int main() {
  const auto host = CreateWindowExW(0, L"STATIC", L"", 0, 0, 0, 1, 1,
                                    HWND_MESSAGE, nullptr, nullptr, nullptr);
  const auto browser = CreateWindowExW(0, L"STATIC", L"", WS_CHILD, 0, 0, 1, 1,
                                       host, nullptr, nullptr, nullptr);
  const auto browser_child = CreateWindowExW(
      0, L"STATIC", L"", WS_CHILD, 0, 0, 1, 1, browser, nullptr, nullptr, nullptr);
  const auto other_browser = CreateWindowExW(
      0, L"STATIC", L"", WS_CHILD, 0, 0, 1, 1, host, nullptr, nullptr, nullptr);
  const auto outside = CreateWindowExW(0, L"STATIC", L"", 0, 0, 0, 1, 1,
                                       HWND_MESSAGE, nullptr, nullptr, nullptr);
  Require(host && browser && browser_child && other_browser && outside,
          "create message-only windows");
  Require(SetWindowSubclass(host, ObserveFlutterFocus, kObserverId, 0),
          "install focus observer");
  Require(SetWindowSubclass(host, webview_all_windows::FlutterViewFocusProc,
                            kFocusHostId, 0), "install production focus host");

  ExpectFocusTransfer(host, browser, 0, "entering WebView retains Flutter focus");
  ExpectFocusTransfer(host, browser_child, 0,
                      "entering nested browser content retains Flutter focus");
  ExpectFocusTransfer(host, outside, 1,
                      "leaving for another window reaches Flutter");
  ExpectFocusTransfer(host, nullptr, 2,
                      "losing all native focus reaches Flutter");
  ExpectBrowserDeparture(host, host, 2,
                         "returning to Flutter stays inside the view");
  ExpectBrowserDeparture(host, other_browser, 2,
                         "switching browsers stays inside the view");
  ExpectBrowserDeparture(host, browser_child, 2,
                         "moving within browser content stays inside the view");
  ExpectBrowserDeparture(host, outside, 3,
                         "browser departure to another window reaches Flutter");
  ExpectBrowserDeparture(host, nullptr, 4,
                         "browser deactivation reaches Flutter");

  Require(RemoveWindowSubclass(host, webview_all_windows::FlutterViewFocusProc,
                                kFocusHostId), "remove focus host");
  ExpectFocusTransfer(host, browser, 5,
                      "explicit teardown restores the original window procedure");
  Require(SetWindowSubclass(host, webview_all_windows::FlutterViewFocusProc,
                            kFocusHostId, 0), "reinstall focus host");
  Require(DestroyWindow(host), "destroy host and native descendants");
  Require(subclass_removed_on_destroy,
          "window destruction removes the subclass before forwarding");
  std::puts("PASS: window destruction removes the subclass before forwarding");
  Require(DestroyWindow(outside), "destroy outside window");
  return 0;
}
