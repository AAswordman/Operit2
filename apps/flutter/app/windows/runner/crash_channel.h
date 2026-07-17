#ifndef RUNNER_CRASH_CHANNEL_H_
#define RUNNER_CRASH_CHANNEL_H_

#include <flutter/flutter_engine.h>

#include <string>

void RegisterOperitCrashChannel(flutter::FlutterEngine* engine);

void ShowOperitWindowsCrashScreen(const std::string& details);

#endif  // RUNNER_CRASH_CHANNEL_H_
