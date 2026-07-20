#ifndef RUNNER_CRASH_CHANNEL_H_
#define RUNNER_CRASH_CHANNEL_H_

#include <flutter/flutter_engine.h>

#include <string>

void RegisterOperitCrashChannel(flutter::FlutterEngine* engine);

/// Releases the crash channel before the Flutter engine is destroyed.
void ShutdownOperitCrashChannel();

void ShowOperitWindowsCrashScreen(const std::string& details);

#endif  // RUNNER_CRASH_CHANNEL_H_
