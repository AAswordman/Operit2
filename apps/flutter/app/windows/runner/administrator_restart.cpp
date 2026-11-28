#include "administrator_restart.h"

#include <objbase.h>
#include <shellapi.h>

#include <array>
#include <charconv>
#include <stdexcept>

#include "operit_runtime_channel.h"

namespace {

constexpr char kRestartArgument[] = "--operit-admin-restart";
constexpr DWORD kReadyTimeoutMillis = 15000;
constexpr DWORD kShutdownTimeoutMillis = 60000;

/// Closes a Windows kernel handle on every return and exception path.
class ScopedHandle {
 public:
  /// Takes ownership of one nullable kernel handle.
  explicit ScopedHandle(HANDLE value) : value_(value) {}
  /// Releases the owned kernel handle.
  ~ScopedHandle() { if (value_ != nullptr) ::CloseHandle(value_); }
  ScopedHandle(const ScopedHandle&) = delete;
  ScopedHandle& operator=(const ScopedHandle&) = delete;
  /// Returns the borrowed handle for a Windows API call.
  HANDLE get() const { return value_; }
 private:
  HANDLE value_;
};

/// Throws an authorization error with the exact Windows operation failure.
void Require(bool succeeded, const char* operation) {
  if (!succeeded) {
    throw std::runtime_error(std::string(operation) + " (Windows error " +
                             std::to_string(::GetLastError()) + ")");
  }
}

/// Reads the full executable path without the legacy MAX_PATH restriction.
std::wstring ExecutablePath() {
  std::array<wchar_t, 32768> path{};
  const DWORD length = ::GetModuleFileNameW(nullptr, path.data(),
                                            static_cast<DWORD>(path.size()));
  Require(length != 0 && length < path.size(), "Cannot resolve executable path");
  return std::wstring(path.data(), length);
}

/// Quotes one Windows argument, preserving embedded quotes and trailing slashes.
std::wstring QuoteArgument(const wchar_t* argument) {
  std::wstring result = L"\"";
  size_t slashes = 0;
  for (const wchar_t* cursor = argument; *cursor; ++cursor) {
    if (*cursor == L'\\') {
      ++slashes;
      continue;
    }
    result.append(*cursor == L'"' ? slashes * 2 + 1 : slashes, L'\\');
    result += *cursor;
    slashes = 0;
  }
  result.append(slashes * 2, L'\\');
  return result + L'"';
}

/// Copies the original launch arguments after the private restart handshake.
std::wstring RestartParameters(const wchar_t* identity) {
  std::wstring parameters = L"--operit-admin-restart " +
      std::to_wstring(::GetCurrentProcessId()) + L" " + identity;
  int count = 0;
  wchar_t** arguments = ::CommandLineToArgvW(::GetCommandLineW(), &count);
  Require(arguments != nullptr, "Cannot read application arguments");
  for (int index = 1; index < count; ++index) {
    parameters += L" " + QuoteArgument(arguments[index]);
  }
  ::LocalFree(arguments);
  return parameters;
}

/// Reads a process account SID so UAC cannot silently change the app's data owner.
std::vector<BYTE> ProcessUser(HANDLE process) {
  HANDLE raw_token = nullptr;
  Require(::OpenProcessToken(process, TOKEN_QUERY, &raw_token) != 0,
          "Cannot open process token");
  ScopedHandle token(raw_token);
  DWORD size = 0;
  ::GetTokenInformation(token.get(), TokenUser, nullptr, 0, &size);
  Require(size != 0, "Cannot size process user information");
  std::vector<BYTE> user(size);
  Require(::GetTokenInformation(token.get(), TokenUser, user.data(), size, &size) != 0,
          "Cannot read process user information");
  return user;
}

/// Ensures the replacement belongs to the same installation and Windows account.
void ValidateRestartParent(HANDLE parent) {
  std::array<wchar_t, 32768> path{};
  DWORD length = static_cast<DWORD>(path.size());
  Require(::QueryFullProcessImageNameW(parent, 0, path.data(), &length) != 0,
          "Cannot inspect restart parent");
  const auto executable = ExecutablePath();
  if (::CompareStringOrdinal(path.data(), static_cast<int>(length),
                            executable.c_str(), static_cast<int>(executable.size()),
                            TRUE) != CSTR_EQUAL) {
    throw std::runtime_error("Administrator restart executable does not match");
  }
  const auto parent_user = ProcessUser(parent);
  const auto current_user = ProcessUser(::GetCurrentProcess());
  if (!::EqualSid(reinterpret_cast<const TOKEN_USER*>(parent_user.data())->User.Sid,
                  reinterpret_cast<const TOKEN_USER*>(current_user.data())->User.Sid)) {
    throw std::runtime_error("Restart Operit as administrator using the same Windows account");
  }
}

}  // namespace

/// Reports elevation from the current process token, never from launch success.
bool IsOperitProcessElevated() {
  HANDLE raw_token = nullptr;
  Require(::OpenProcessToken(::GetCurrentProcess(), TOKEN_QUERY, &raw_token) != 0,
          "Cannot open current process token");
  ScopedHandle token(raw_token);
  TOKEN_ELEVATION elevation{};
  DWORD size = 0;
  Require(::GetTokenInformation(token.get(), TokenElevation, &elevation,
                                sizeof(elevation), &size) != 0,
          "Cannot read process elevation");
  return elevation.TokenIsElevated != 0;
}

/// Performs a UAC restart handshake before requesting normal runtime teardown.
void RequestOperitAdministratorRestart(HWND window, DWORD platform_thread_id) {
  if (IsOperitProcessElevated()) return;
  GUID identity{};
  if (FAILED(::CoCreateGuid(&identity))) {
    throw std::runtime_error("Cannot create administrator restart identity");
  }
  std::array<wchar_t, 40> identity_text{};
  Require(::StringFromGUID2(identity, identity_text.data(),
                            static_cast<int>(identity_text.size())) != 0,
          "Cannot format administrator restart identity");
  const std::wstring event_name = std::wstring(L"Local\\Operit2.AdminRestart.") +
                                 identity_text.data();
  ScopedHandle ready(::CreateEventW(nullptr, TRUE, FALSE, (event_name + L".Ready").c_str()));
  Require(ready.get() != nullptr, "Cannot create administrator readiness event");
  ScopedHandle cancel(::CreateEventW(nullptr, TRUE, FALSE, (event_name + L".Cancel").c_str()));
  Require(cancel.get() != nullptr, "Cannot create administrator cancellation event");

  const auto executable = ExecutablePath();
  const auto parameters = RestartParameters(identity_text.data());
  SHELLEXECUTEINFOW launch{};
  launch.cbSize = sizeof(launch);
  launch.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI;
  launch.hwnd = ::GetAncestor(window, GA_ROOT);
  launch.lpVerb = L"runas";
  launch.lpFile = executable.c_str();
  launch.lpParameters = parameters.c_str();
  launch.nShow = SW_SHOWNORMAL;
  if (!::ShellExecuteExW(&launch)) {
    if (::GetLastError() == ERROR_CANCELLED) return;
    Require(false, "Cannot start administrator replacement");
  }
  ScopedHandle child(launch.hProcess);
  Require(child.get() != nullptr, "Administrator replacement has no process handle");
  const HANDLE waiting[] = {child.get(), ready.get()};
  const DWORD status = ::WaitForMultipleObjects(2, waiting, FALSE, kReadyTimeoutMillis);
  if (status != WAIT_OBJECT_0 + 1) {
    ::SetEvent(cancel.get());
    throw std::runtime_error("Administrator replacement did not become ready");
  }
  if (!RequestOperitApplicationExit(platform_thread_id)) {
    const DWORD error = ::GetLastError();
    ::SetEvent(cancel.get());
    throw std::runtime_error("Cannot exit for administrator restart (Windows error " +
                             std::to_string(error) + ")");
  }
}

/// Waits before COM registration, the singleton guard, Flutter, or Core startup.
bool CompleteOperitAdministratorRestart(std::vector<std::string>* arguments) try {
  if (arguments->empty() || arguments->front() != kRestartArgument) return true;
  if (arguments->size() < 3 || !IsOperitProcessElevated()) return false;
  DWORD parent_id = 0;
  const auto& parent_argument = (*arguments)[1];
  const auto parsed = std::from_chars(parent_argument.data(),
      parent_argument.data() + parent_argument.size(), parent_id);
  if (parsed.ec != std::errc() || parsed.ptr != parent_argument.data() + parent_argument.size() ||
      parent_id == 0 || parent_id == ::GetCurrentProcessId()) return false;
  const auto& identity = (*arguments)[2];
  if (identity.size() != 38) return false;
  const std::wstring wide_identity(identity.begin(), identity.end());
  GUID guid{};
  if (FAILED(::CLSIDFromString(wide_identity.c_str(), &guid))) return false;
  ScopedHandle parent(::OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
                                    FALSE, parent_id));
  Require(parent.get() != nullptr, "Cannot open administrator restart parent");
  ValidateRestartParent(parent.get());
  const std::wstring event_name = L"Local\\Operit2.AdminRestart." + wide_identity;
  ScopedHandle ready(::OpenEventW(EVENT_MODIFY_STATE, FALSE, (event_name + L".Ready").c_str()));
  Require(ready.get() != nullptr, "Cannot open administrator readiness event");
  ScopedHandle cancel(::OpenEventW(SYNCHRONIZE, FALSE, (event_name + L".Cancel").c_str()));
  Require(cancel.get() != nullptr, "Cannot open administrator cancellation event");
  Require(::SetEvent(ready.get()) != 0, "Cannot acknowledge administrator readiness");
  const HANDLE waiting[] = {cancel.get(), parent.get()};
  if (::WaitForMultipleObjects(2, waiting, FALSE, kShutdownTimeoutMillis) != WAIT_OBJECT_0 + 1) {
    return false;
  }
  arguments->erase(arguments->begin(), arguments->begin() + 3);
  return true;
} catch (const std::exception& error) {
  // The original process reports handshake failure in its authorization UI.
  // A rejected replacement must not open a second window, including a crash UI.
  ::OutputDebugStringA(error.what());
  return false;
}
