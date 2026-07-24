#ifndef RUNNER_OPERIT_RUNTIME_CHANNEL_H_
#define RUNNER_OPERIT_RUNTIME_CHANNEL_H_

#include <flutter/flutter_engine.h>
#include <windows.h>

/// Registers the runtime channel for one Flutter engine and native window.
void RegisterOperitRuntimeChannel(flutter::FlutterEngine* engine, HWND window);

/// Stops runtime worker activity before bridge-library teardown.
void ShutdownOperitRuntimeChannel();

/// Processes a queued runtime task message on the platform thread.
bool HandleOperitRuntimeChannelWindowMessage(UINT message,
                                             WPARAM wparam,
                                             LPARAM lparam,
                                             LRESULT* result);

#endif  // RUNNER_OPERIT_RUNTIME_CHANNEL_H_
