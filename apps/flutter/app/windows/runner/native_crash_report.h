#ifndef RUNNER_NATIVE_CRASH_REPORT_H_
#define RUNNER_NATIVE_CRASH_REPORT_H_

#include <windows.h>

/// Records an unhandled native exception and presents its diagnostic location.
LONG WINAPI ReportOperitWindowsNativeCrash(EXCEPTION_POINTERS* exception);

#endif  // RUNNER_NATIVE_CRASH_REPORT_H_
