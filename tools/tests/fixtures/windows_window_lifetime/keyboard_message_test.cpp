#include <windows.h>

#include <cstdlib>
#include <iostream>

#include "keyboard_message_normalizer.h"

/// Stops on a failed keyboard invariant in optimized builds too.
void Require(bool condition, const char* message) {
  if (!condition) {
    std::cerr << message << std::endl;
    std::exit(1);
  }
}

/// Checks clipboard-history injection, translated characters, and ordinary keyboard input.
int main() {
  const LPARAM down = 1;
  const LPARAM up = static_cast<LPARAM>(0xc0000001);
  const auto control = NormalizeOperitKeyboardMessage(WM_KEYDOWN, VK_CONTROL, down);
  const auto paste = NormalizeOperitKeyboardMessage(WM_KEYDOWN, 'V', down);
  Require((control & 0x00ff0000) != 0, "Injected Control has no scan code");
  Require((paste & 0x00ff0000) != 0, "Injected V has no scan code");
  Require((control & 0x00ff0000) != (paste & 0x00ff0000), "Control and V share a physical key");
  Require(NormalizeOperitKeyboardMessage(WM_KEYDOWN, 'V', paste) == paste,
          "Ordinary Ctrl+V scan code changed");
  const auto released = NormalizeOperitKeyboardMessage(WM_KEYUP, 'V', up);
  Require((released & 0x00ff0000) == (paste & 0x00ff0000), "Key up has a different physical key");
  Require((released & 0xc000ffff) == up, "Key up flags changed");
  Require(NormalizeOperitKeyboardMessage(WM_KEYDOWN, VK_PACKET, down) == down,
          "Unicode packet input changed");
  Require(NormalizeOperitKeyboardMessage(WM_CHAR, 22, down) == down,
          "Character input was treated as a virtual key");
  const auto right_control = NormalizeOperitKeyboardMessage(WM_KEYDOWN, VK_RCONTROL, down);
  Require((right_control & 0x01000000) != 0, "Right Control extended key flag was lost");
  Require(NormalizeOperitKeyboardMessage(WM_KEYDOWN, VK_RIGHT, down) == down,
          "Unrelated virtual-key input changed");

  const HWND window = CreateWindowW(L"STATIC", L"", 0, 0, 0, 1, 1,
                                    HWND_MESSAGE, nullptr, nullptr, nullptr);
  Require(window != nullptr, "Could not create keyboard test window");
  MSG key = {};
  key.hwnd = window;
  key.message = WM_KEYDOWN;
  key.wParam = 'V';
  key.lParam = paste;
  Require(TranslateMessage(&key), "Injected V did not produce a character");
  MSG character = {};
  Require(PeekMessage(&character, window, WM_CHAR, WM_CHAR, PM_REMOVE),
          "Translated character was not queued");
  Require((character.lParam & 0x00ff0000) == (paste & 0x00ff0000),
          "TranslateMessage lost the injected key scan code");
  DestroyWindow(window);
  std::cout << "Windows clipboard-history keyboard checks passed" << std::endl;
}
