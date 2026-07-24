#ifndef RUNNER_CRASH_CHANNEL_H_
#define RUNNER_CRASH_CHANNEL_H_

#include <flutter/flutter_engine.h>

#include <string>

/// Registers the crash presentation channel for one Flutter engine.
void RegisterOperitCrashChannel(flutter::FlutterEngine* engine);

/// Releases the crash channel before the Flutter engine is destroyed.
void ShutdownOperitCrashChannel();

/// Shows the native Windows crash dialog with the supplied detail text.
void ShowOperitWindowsCrashScreen(const std::string& details);

#endif  // RUNNER_CRASH_CHANNEL_H_
