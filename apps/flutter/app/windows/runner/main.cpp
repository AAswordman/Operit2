#include <flutter/dart_project.h>
#include <flutter/flutter_view_controller.h>
#include <knownfolders.h>
#include <propkey.h>
#include <propvarutil.h>
#include <shlobj_core.h>
#include <shobjidl_core.h>
#include <windows.h>
#include <wrl/client.h>

#include <array>
#include <exception>
#include <filesystem>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

#include "crash_channel.h"
#include "engine_channel_lifetime.h"
#include "flutter_window.h"
#include "operit_runtime_channel.h"
#include "system_audio_input_channel.h"
#include "utils.h"

namespace {

constexpr wchar_t kOperitToastAppUserModelId[] = L"app.operit.operit2";

/// Throws a descriptive startup error when one Windows toast setup call fails.
void RequireWindowsToastSuccess(HRESULT result, const char* operation) {
  if (FAILED(result)) {
    throw std::runtime_error(operation);
  }
}

/// Throws a descriptive startup error when one Windows registry call fails.
void RequireWindowsRegistrySuccess(LSTATUS result, const char* operation) {
  if (result != ERROR_SUCCESS) {
    throw std::runtime_error(operation);
  }
}

/// Registers the current executable as the desktop toast activator for Operit.
void ConfigureOperitWindowsToastIdentity() {
  RequireWindowsToastSuccess(
      ::SetCurrentProcessExplicitAppUserModelID(kOperitToastAppUserModelId),
      "Failed to assign the Operit Windows notification identity.");

  std::array<wchar_t, 32768> executable_path{};
  const DWORD path_length = ::GetModuleFileNameW(
      nullptr, executable_path.data(), static_cast<DWORD>(executable_path.size()));
  if (path_length == 0 || path_length == executable_path.size()) {
    throw std::runtime_error("Failed to resolve the Operit executable path.");
  }

  PWSTR start_menu_path = nullptr;
  RequireWindowsToastSuccess(
      ::SHGetKnownFolderPath(FOLDERID_StartMenu, KF_FLAG_CREATE, nullptr,
                             &start_menu_path),
      "Failed to resolve the Windows Start Menu path.");
  const std::filesystem::path shortcut_path =
      std::filesystem::path(start_menu_path) / L"Programs" / L"Operit2.lnk";
  ::CoTaskMemFree(start_menu_path);
  std::error_code directory_error;
  std::filesystem::create_directories(shortcut_path.parent_path(), directory_error);
  if (directory_error) {
    throw std::runtime_error("Failed to create the Windows Start Menu shortcut directory.");
  }

  Microsoft::WRL::ComPtr<IShellLinkW> shell_link;
  RequireWindowsToastSuccess(
      ::CoCreateInstance(CLSID_ShellLink, nullptr, CLSCTX_INPROC_SERVER,
                         IID_PPV_ARGS(&shell_link)),
      "Failed to create the Windows Start Menu shortcut.");
  RequireWindowsToastSuccess(
      shell_link->SetPath(executable_path.data()),
      "Failed to set the Operit shortcut executable path.");

  Microsoft::WRL::ComPtr<IPropertyStore> property_store;
  RequireWindowsToastSuccess(
      shell_link.As(&property_store),
      "Failed to access the Operit shortcut property store.");
  PROPVARIANT app_user_model_id;
  ::PropVariantInit(&app_user_model_id);
  RequireWindowsToastSuccess(
      ::InitPropVariantFromString(kOperitToastAppUserModelId, &app_user_model_id),
      "Failed to create the Operit notification identity property.");
  RequireWindowsToastSuccess(
      property_store->SetValue(PKEY_AppUserModel_ID, app_user_model_id),
      "Failed to set the Operit notification identity on its shortcut.");
  ::PropVariantClear(&app_user_model_id);
  RequireWindowsToastSuccess(
      property_store->Commit(),
      "Failed to commit the Operit shortcut notification identity.");

  Microsoft::WRL::ComPtr<IPersistFile> persist_file;
  RequireWindowsToastSuccess(
      shell_link.As(&persist_file),
      "Failed to persist the Operit Start Menu shortcut.");
  RequireWindowsToastSuccess(
      persist_file->Save(shortcut_path.c_str(), TRUE),
      "Failed to save the Operit Start Menu shortcut.");
}

/// Registers this executable as the handler for Operit notification protocol URLs.
void ConfigureOperitNotificationProtocol() {
  std::array<wchar_t, 32768> executable_path{};
  const DWORD path_length = ::GetModuleFileNameW(
      nullptr, executable_path.data(), static_cast<DWORD>(executable_path.size()));
  if (path_length == 0 || path_length == executable_path.size()) {
    throw std::runtime_error("Failed to resolve the Operit executable path.");
  }

  HKEY protocol_key = nullptr;
  RequireWindowsRegistrySuccess(
      ::RegCreateKeyExW(HKEY_CURRENT_USER, L"Software\\Classes\\operit2", 0,
                        nullptr, 0, KEY_SET_VALUE, nullptr, &protocol_key,
                        nullptr),
      "Failed to create the Operit notification protocol registry key.");
  const std::wstring protocol_description = L"URL:Operit2 Protocol";
  RequireWindowsRegistrySuccess(
      ::RegSetValueExW(protocol_key, nullptr, 0, REG_SZ,
                       reinterpret_cast<const BYTE*>(protocol_description.c_str()),
                       static_cast<DWORD>((protocol_description.size() + 1) * sizeof(wchar_t))),
      "Failed to set the Operit notification protocol description.");
  RequireWindowsRegistrySuccess(
      ::RegSetValueExW(protocol_key, L"URL Protocol", 0, REG_SZ,
                       reinterpret_cast<const BYTE*>(L""), sizeof(wchar_t)),
      "Failed to mark the Operit notification protocol as a URL handler.");
  ::RegCloseKey(protocol_key);

  HKEY command_key = nullptr;
  RequireWindowsRegistrySuccess(
      ::RegCreateKeyExW(HKEY_CURRENT_USER,
                        L"Software\\Classes\\operit2\\shell\\open\\command", 0,
                        nullptr, 0, KEY_SET_VALUE, nullptr, &command_key,
                        nullptr),
      "Failed to create the Operit notification protocol command registry key.");
  const std::wstring command = std::wstring(L"\"") + executable_path.data() +
                               L"\" \"%1\"";
  RequireWindowsRegistrySuccess(
      ::RegSetValueExW(command_key, nullptr, 0, REG_SZ,
                       reinterpret_cast<const BYTE*>(command.c_str()),
                       static_cast<DWORD>((command.size() + 1) * sizeof(wchar_t))),
      "Failed to set the Operit notification protocol command.");
  ::RegCloseKey(command_key);
}

/// Extracts the notification protocol URI from a Flutter desktop command line.
std::optional<std::string> NotificationActivationArgument(
    const std::vector<std::string>& command_line_arguments) {
  constexpr char kNotificationPrefix[] = "operit2://notification/";
  for (const std::string& argument : command_line_arguments) {
    if (argument.compare(0, sizeof(kNotificationPrefix) - 1,
                         kNotificationPrefix) == 0) {
      return argument;
    }
  }
  return std::nullopt;
}

/// Transfers one notification protocol URI to the existing main Operit window.
bool ForwardNotificationActivationToExistingWindow(const std::string& uri) {
  HWND window = ::FindWindowW(L"FLUTTER_RUNNER_WIN32_WINDOW", L"Operit2");
  if (window == nullptr) {
    return false;
  }
  COPYDATASTRUCT copy_data{};
  copy_data.dwData = kOperitNotificationActivationCopyData;
  copy_data.cbData = static_cast<DWORD>(uri.size() + 1);
  copy_data.lpData = const_cast<char*>(uri.c_str());
  DWORD_PTR response = FALSE;
  const LRESULT delivered = ::SendMessageTimeoutW(
      window, WM_COPYDATA, 0, reinterpret_cast<LPARAM>(&copy_data),
      SMTO_ABORTIFHUNG, 5000, &response);
  if (delivered == 0 || response == FALSE) {
    return false;
  }
  ::ShowWindow(window, SW_RESTORE);
  ::SetForegroundWindow(window);
  return true;
}

}  // namespace

/// Presents the native crash dialog for unhandled Windows exceptions.
LONG WINAPI OperitUnhandledExceptionFilter(EXCEPTION_POINTERS*) {
  ShowOperitWindowsCrashScreen("Unhandled Windows exception outside Flutter.");
  return EXCEPTION_EXECUTE_HANDLER;
}

/// Runs the Windows Flutter application message loop.
int APIENTRY wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE prev,
                      _In_ wchar_t *command_line, _In_ int show_command) {
  ::SetUnhandledExceptionFilter(OperitUnhandledExceptionFilter);
  try {
    // Attach to console when present (e.g., 'flutter run') or create a
    // new console when running with a debugger.
    if (!::AttachConsole(ATTACH_PARENT_PROCESS) && ::IsDebuggerPresent()) {
      CreateAndAttachConsole();
    }

    // Initialize COM, so that it is available for use in the library and/or
    // plugins.
    ::CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    ConfigureOperitWindowsToastIdentity();
    ConfigureOperitNotificationProtocol();

    std::vector<std::string> command_line_arguments =
        GetCommandLineArguments();
    const std::optional<std::string> notification_activation =
        NotificationActivationArgument(command_line_arguments);
    if (notification_activation.has_value() &&
        ForwardNotificationActivationToExistingWindow(*notification_activation)) {
      ::CoUninitialize();
      return EXIT_SUCCESS;
    }

    flutter::DartProject project(L"data");

    project.set_dart_entrypoint_arguments(std::move(command_line_arguments));

    FlutterWindow window(project);
    Win32Window::Point origin(10, 10);
    Win32Window::Size size(1280, 720);
    if (!window.Create(L"Operit2", origin, size)) {
      return EXIT_FAILURE;
    }
    window.SetQuitOnClose(true);

    ::MSG msg;
    while (::GetMessage(&msg, nullptr, 0, 0)) {
      ::TranslateMessage(&msg);
      ::DispatchMessage(&msg);
    }

    ShutdownOperitRuntimeChannel();
    ShutdownAllOperitEngineChannels();
    ShutdownSystemAudioInputChannel();
    ShutdownOperitCrashChannel();
    ::CoUninitialize();
    return EXIT_SUCCESS;
  } catch (const std::exception& error) {
    ShowOperitWindowsCrashScreen(error.what());
    ShutdownOperitRuntimeChannel();
    ShutdownAllOperitEngineChannels();
    ShutdownSystemAudioInputChannel();
    ShutdownOperitCrashChannel();
    ::CoUninitialize();
    return EXIT_FAILURE;
  } catch (...) {
    ShowOperitWindowsCrashScreen("Unhandled C++ exception outside Flutter.");
    ShutdownOperitRuntimeChannel();
    ShutdownAllOperitEngineChannels();
    ShutdownSystemAudioInputChannel();
    ShutdownOperitCrashChannel();
    ::CoUninitialize();
    return EXIT_FAILURE;
  }
}
