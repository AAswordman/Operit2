#pragma once

#include <windows.h>

/// Gives clipboard-history Ctrl+V injection distinct physical keys before Flutter handles it.
inline LPARAM NormalizeOperitKeyboardMessage(UINT message, WPARAM key,
                                             LPARAM data) {
  if (message != WM_KEYDOWN && message != WM_KEYUP &&
      message != WM_SYSKEYDOWN && message != WM_SYSKEYUP) {
    return data;
  }
  if ((data & 0x00ff0000) != 0 ||
      (key != VK_CONTROL && key != VK_LCONTROL && key != VK_RCONTROL &&
       key != 'V')) {
    return data;
  }
  const UINT scan = MapVirtualKeyW(static_cast<UINT>(key), MAPVK_VK_TO_VSC_EX);
  data |= static_cast<LPARAM>(scan & 0xff) << 16;
  if ((scan & 0xff00) == 0xe000) {
    data |= static_cast<LPARAM>(1) << 24;
  }
  return data;
}

