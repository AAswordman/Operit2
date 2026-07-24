#ifndef RUNNER_SYSTEM_AUDIO_INPUT_CHANNEL_H_
#define RUNNER_SYSTEM_AUDIO_INPUT_CHANNEL_H_

#include <flutter/flutter_engine.h>

/// Registers the Windows system-audio input channel for one Flutter engine.
void RegisterSystemAudioInputChannel(flutter::FlutterEngine* engine);

/// Releases all system-audio input channels before their engine is destroyed.
void ShutdownSystemAudioInputChannel();

#endif  // RUNNER_SYSTEM_AUDIO_INPUT_CHANNEL_H_
