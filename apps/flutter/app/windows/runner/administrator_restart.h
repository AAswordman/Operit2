#ifndef RUNNER_ADMINISTRATOR_RESTART_H_
#define RUNNER_ADMINISTRATOR_RESTART_H_

#include <windows.h>

#include <string>
#include <vector>

/// Reports whether the current process has an elevated Windows token.
bool IsOperitProcessElevated();

/// Starts an elevated replacement and quits only after it acknowledges readiness.
void RequestOperitAdministratorRestart(HWND window, DWORD platform_thread_id);

/// Consumes restart arguments and waits for the old process before any app startup.
bool CompleteOperitAdministratorRestart(std::vector<std::string>* arguments);

#endif  // RUNNER_ADMINISTRATOR_RESTART_H_
