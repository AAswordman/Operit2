#pragma once

#include <windows.h>
#include <commctrl.h>

namespace webview_all_windows {

/// Keeps focus transfers into hosted native content inside the Flutter view.
inline LRESULT CALLBACK FlutterViewFocusProc(
    HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam, UINT_PTR subclass_id,
    DWORD_PTR) {
  if (message == WM_LBUTTONDOWN || message == WM_RBUTTONDOWN ||
      message == WM_MBUTTONDOWN || message == WM_XBUTTONDOWN ||
      message == WM_POINTERDOWN) {
    // Composition WebViews receive forwarded pointer events, while the HWND
    // receiving this click is Flutter's. Reclaim keyboard/IME synchronously:
    // Dart's focus node may already be focused and emit no focus change.
    const HWND current = GetFocus();
    if (IsChild(hwnd, current)) SetFocus(hwnd);
  }
  if (message == WM_KILLFOCUS &&
      IsChild(hwnd, reinterpret_cast<HWND>(wparam))) {
    // Flutter must retain its focus scope while WebView2 owns keyboard input.
    DestroyCaret();
    return 0;
  }
  if (message == WM_NCDESTROY) {
    RemoveWindowSubclass(hwnd, FlutterViewFocusProc, subclass_id);
  }
  return DefSubclassProc(hwnd, message, wparam, lparam);
}

/// Notifies Flutter only when native focus leaves its entire hosted subtree.
inline void NotifyFlutterViewFocusLost(HWND hwnd) {
  const HWND current = GetFocus();
  if (current != hwnd && !IsChild(hwnd, current)) {
    SendMessage(hwnd, WM_KILLFOCUS, reinterpret_cast<WPARAM>(current), 0);
  }
}

}  // namespace webview_all_windows
