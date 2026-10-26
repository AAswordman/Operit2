#include "native_crash_report.h"

#include <dbghelp.h>
#include <strsafe.h>

namespace {

SRWLOCK g_dump_lock = SRWLOCK_INIT;

struct NativeCrashRequest {
  EXCEPTION_POINTERS* exception;
  DWORD thread_id;
};

/// Creates the application's native crash report directory.
DWORD CreateCrashDirectory(wchar_t (&directory)[MAX_PATH]) {
  const DWORD length = GetEnvironmentVariableW(L"LOCALAPPDATA", directory, MAX_PATH);
  if (length == 0) {
    return GetLastError();
  }
  if (length >= MAX_PATH) {
    return ERROR_FILENAME_EXCED_RANGE;
  }
  if (FAILED(StringCchCatW(directory, MAX_PATH, L"\\Operit2"))) {
    return ERROR_FILENAME_EXCED_RANGE;
  }
  if (!CreateDirectoryW(directory, nullptr)) {
    const DWORD error = GetLastError();
    if (error != ERROR_ALREADY_EXISTS) {
      return error;
    }
  }
  if (FAILED(StringCchCatW(directory, MAX_PATH, L"\\crashes"))) {
    return ERROR_FILENAME_EXCED_RANGE;
  }
  if (!CreateDirectoryW(directory, nullptr)) {
    const DWORD error = GetLastError();
    if (error != ERROR_ALREADY_EXISTS) {
      return error;
    }
  }
  return ERROR_SUCCESS;
}

/// Writes a UTF-16 diagnostic report and returns the actual Windows I/O error.
DWORD WriteCrashDetails(const wchar_t* path, const wchar_t* details) {
  const HANDLE file = CreateFileW(path, GENERIC_WRITE, 0, nullptr, CREATE_NEW,
                                  FILE_ATTRIBUTE_NORMAL, nullptr);
  if (file == INVALID_HANDLE_VALUE) {
    return GetLastError();
  }
  const wchar_t bom = 0xfeff;
  DWORD written = 0;
  DWORD error = ERROR_SUCCESS;
  const DWORD size = static_cast<DWORD>(lstrlenW(details)) * sizeof(wchar_t);
  if (!WriteFile(file, &bom, sizeof(bom), &written, nullptr) ||
      !WriteFile(file, details, size, &written, nullptr)) {
    error = GetLastError();
  } else if (written != size) {
    error = ERROR_WRITE_FAULT;
  }
  CloseHandle(file);
  return error;
}

/// Saves the faulting thread context from a separate thread before showing the dialog.
DWORD WINAPI WriteNativeCrashReport(void* parameter) {
  const auto& request = *static_cast<NativeCrashRequest*>(parameter);
  const auto& exception = *request.exception->ExceptionRecord;
  wchar_t module[MAX_PATH] = {};
  HMODULE module_handle = nullptr;
  const BOOL module_found = GetModuleHandleExW(
      GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS |
          GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
      reinterpret_cast<LPCWSTR>(exception.ExceptionAddress), &module_handle);
  DWORD module_error = ERROR_SUCCESS;
  if (!module_found || GetModuleFileNameW(module_handle, module, MAX_PATH) == 0) {
    module_error = GetLastError();
  }
  wchar_t details[4096] = {};
  StringCchPrintfW(details, ARRAYSIZE(details),
      L"Unhandled Windows exception outside Flutter.\r\n\r\n"
      L"Exception code: 0x%08lX\r\nAddress: 0x%p\r\n"
      L"Process: %lu\r\nThread: %lu\r\nModule: %ls\r\n"
      L"Module base: 0x%p\r\nModule lookup error: %lu\r\n",
      exception.ExceptionCode, exception.ExceptionAddress,
      GetCurrentProcessId(), request.thread_id, module,
      static_cast<void*>(module_handle), module_error);
  if (exception.ExceptionCode == EXCEPTION_ACCESS_VIOLATION &&
      exception.NumberParameters >= 2) {
    wchar_t access[160] = {};
    StringCchPrintfW(access, ARRAYSIZE(access),
        L"Access operation: %llu (0=read, 1=write, 8=execute)\r\n"
        L"Access address: 0x%p\r\n",
        static_cast<unsigned long long>(exception.ExceptionInformation[0]),
        reinterpret_cast<void*>(exception.ExceptionInformation[1]));
    StringCchCatW(details, ARRAYSIZE(details), access);
  }
  wchar_t directory[MAX_PATH] = {};
  DWORD report_error = CreateCrashDirectory(directory);
  DWORD dump_error = report_error;
  wchar_t text_path[MAX_PATH] = {};
  wchar_t dump_path[MAX_PATH] = {};
  if (report_error == ERROR_SUCCESS) {
    SYSTEMTIME now = {};
    GetSystemTime(&now);
    wchar_t stem[MAX_PATH] = {};
    const HRESULT path_result = StringCchPrintfW(stem, ARRAYSIZE(stem),
        L"%ls\\native-%04u%02u%02u-%02u%02u%02u-%03u-%lu-%lu",
        directory, now.wYear, now.wMonth, now.wDay, now.wHour,
        now.wMinute, now.wSecond, now.wMilliseconds,
        GetCurrentProcessId(), request.thread_id);
    if (FAILED(path_result) ||
        FAILED(StringCchPrintfW(text_path, ARRAYSIZE(text_path), L"%ls.txt", stem)) ||
        FAILED(StringCchPrintfW(dump_path, ARRAYSIZE(dump_path), L"%ls.dmp", stem))) {
      report_error = ERROR_FILENAME_EXCED_RANGE;
      dump_error = report_error;
    } else {
      // Persist the exception details before entering DbgHelp.
      report_error = WriteCrashDetails(text_path, details);
      const HANDLE dump = CreateFileW(dump_path, GENERIC_WRITE, 0, nullptr,
                                      CREATE_NEW, FILE_ATTRIBUTE_NORMAL, nullptr);
      if (dump == INVALID_HANDLE_VALUE) {
        dump_error = GetLastError();
      } else {
        MINIDUMP_EXCEPTION_INFORMATION information = {};
        information.ThreadId = request.thread_id;
        information.ExceptionPointers = request.exception;
        information.ClientPointers = FALSE;
        const auto type = static_cast<MINIDUMP_TYPE>(
            MiniDumpNormal | MiniDumpWithThreadInfo | MiniDumpWithUnloadedModules);
        AcquireSRWLockExclusive(&g_dump_lock);
        const BOOL saved = MiniDumpWriteDump(GetCurrentProcess(),
            GetCurrentProcessId(), dump, type, &information, nullptr, nullptr);
        dump_error = saved ? ERROR_SUCCESS : GetLastError();
        ReleaseSRWLockExclusive(&g_dump_lock);
        CloseHandle(dump);
      }
    }
  }
  wchar_t output[1024] = {};
  StringCchPrintfW(output, ARRAYSIZE(output),
      L"\r\nDiagnostic report: %ls\r\nReport write error: %lu\r\n"
      L"Memory dump: %ls\r\nDump write error: %lu\r\n"
      L"(Windows error 0 means success.)",
      text_path, report_error, dump_path, dump_error);
  StringCchCatW(details, ARRAYSIZE(details), output);
  MessageBoxW(nullptr, details, L"Operit2 has stopped",
              MB_OK | MB_ICONERROR | MB_TASKMODAL | MB_SETFOREGROUND);
  return 0;
}

}  // namespace

/// Keeps the faulting context alive until the reporting thread has saved it.
LONG WINAPI ReportOperitWindowsNativeCrash(EXCEPTION_POINTERS* exception) {
  NativeCrashRequest request = {exception, GetCurrentThreadId()};
  const HANDLE reporter = CreateThread(nullptr, 0, WriteNativeCrashReport,
                                        &request, 0, nullptr);
  if (reporter == nullptr) {
    const DWORD error = GetLastError();
    wchar_t details[256] = {};
    StringCchPrintfW(details, ARRAYSIZE(details),
        L"Exception code: 0x%08lX\r\nAddress: 0x%p\r\n"
        L"Could not create the native crash reporter. Windows error: %lu",
        exception->ExceptionRecord->ExceptionCode,
        exception->ExceptionRecord->ExceptionAddress, error);
    MessageBoxW(nullptr, details, L"Operit2 has stopped", MB_OK | MB_ICONERROR);
  } else {
    WaitForSingleObject(reporter, INFINITE);
    CloseHandle(reporter);
  }
  return EXCEPTION_EXECUTE_HANDLER;
}
