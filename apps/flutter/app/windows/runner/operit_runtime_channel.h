#ifndef RUNNER_OPERIT_RUNTIME_CHANNEL_H_
#define RUNNER_OPERIT_RUNTIME_CHANNEL_H_

#include <flutter/flutter_engine.h>
#include <windows.h>

void RegisterOperitRuntimeChannel(flutter::FlutterEngine* engine, HWND window);

void ShutdownOperitRuntimeChannel();

bool HandleOperitRuntimeChannelWindowMessage(UINT message,
                                             WPARAM wparam,
                                             LPARAM lparam,
                                             LRESULT* result);

#endif  // RUNNER_OPERIT_RUNTIME_CHANNEL_H_
