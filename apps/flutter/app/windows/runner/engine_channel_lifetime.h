#ifndef RUNNER_ENGINE_CHANNEL_LIFETIME_H_
#define RUNNER_ENGINE_CHANNEL_LIFETIME_H_

#include <flutter/flutter_engine.h>

#include <functional>

using OperitEngineChannelShutdown = std::function<void()>;

/// Registers cleanup that runs while the supplied engine messenger is valid.
void RegisterOperitEngineChannelShutdown(
    flutter::FlutterEngine* engine,
    OperitEngineChannelShutdown shutdown);

/// Runs and removes cleanup registered for one live Flutter engine.
void ShutdownOperitEngineChannels(flutter::FlutterEngine* engine);

/// Runs and removes cleanup registered for every live Flutter engine.
void ShutdownAllOperitEngineChannels();

#endif  // RUNNER_ENGINE_CHANNEL_LIFETIME_H_
