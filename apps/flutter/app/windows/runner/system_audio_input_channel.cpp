#include "system_audio_input_channel.h"

#include <flutter/encodable_value.h>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
#include <propkey.h>
#include <functiondiscoverykeys_devpkey.h>
#include <mmdeviceapi.h>
#include <propvarutil.h>
#include <wrl/client.h>

#include "engine_channel_lifetime.h"

#include <algorithm>
#include <memory>
#include <sstream>
#include <string>
#include <vector>

namespace {

constexpr char kSystemAudioInputChannelName[] = "operit/audio-input";
constexpr char kResolveDefaultCaptureDeviceMethod[] =
    "resolveDefaultCaptureDevice";

using SystemAudioInputMethodChannel =
    flutter::MethodChannel<flutter::EncodableValue>;

std::vector<std::shared_ptr<SystemAudioInputMethodChannel>>
    g_system_audio_input_channels;

/// Converts one Windows HRESULT into a compact diagnostic string.
std::string HResultDescription(HRESULT result) {
  std::ostringstream stream;
  stream << "HRESULT 0x" << std::hex << static_cast<unsigned long>(result);
  return stream.str();
}

/// Converts one UTF-16 Windows value into a UTF-8 Flutter string.
bool WideToUtf8(const std::wstring& value, std::string* result) {
  if (result == nullptr) {
    return false;
  }
  if (value.empty()) {
    result->clear();
    return true;
  }
  const int required = ::WideCharToMultiByte(
      CP_UTF8, WC_ERR_INVALID_CHARS, value.data(),
      static_cast<int>(value.size()), nullptr, 0, nullptr, nullptr);
  if (required <= 0) {
    return false;
  }
  std::string utf8(required, '\0');
  const int written = ::WideCharToMultiByte(
      CP_UTF8, WC_ERR_INVALID_CHARS, value.data(),
      static_cast<int>(value.size()), utf8.data(), required, nullptr, nullptr);
  if (written != required) {
    return false;
  }
  *result = std::move(utf8);
  return true;
}

/// Reads the current Windows default capture endpoint as Flutter data.
bool ResolveDefaultCaptureDevice(
    flutter::EncodableValue* device, std::string* error) {
  if (device == nullptr || error == nullptr) {
    return false;
  }
  Microsoft::WRL::ComPtr<IMMDeviceEnumerator> enumerator;
  HRESULT result = ::CoCreateInstance(
      __uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL,
      IID_PPV_ARGS(enumerator.GetAddressOf()));
  if (FAILED(result)) {
    *error = "Unable to create the Windows audio device enumerator: " +
             HResultDescription(result);
    return false;
  }

  Microsoft::WRL::ComPtr<IMMDevice> audio_device;
  result = enumerator->GetDefaultAudioEndpoint(
      eCapture, eConsole, audio_device.GetAddressOf());
  if (FAILED(result)) {
    *error = "Windows has no default capture device: " +
             HResultDescription(result);
    return false;
  }

  LPWSTR raw_device_id = nullptr;
  result = audio_device->GetId(&raw_device_id);
  if (FAILED(result) || raw_device_id == nullptr) {
    *error = "Unable to read the Windows default capture device ID: " +
             HResultDescription(result);
    return false;
  }
  const std::wstring device_id(raw_device_id);
  ::CoTaskMemFree(raw_device_id);

  Microsoft::WRL::ComPtr<IPropertyStore> properties;
  result = audio_device->OpenPropertyStore(STGM_READ, properties.GetAddressOf());
  if (FAILED(result)) {
    *error = "Unable to open the Windows default capture device properties: " +
             HResultDescription(result);
    return false;
  }
  PROPVARIANT friendly_name;
  ::PropVariantInit(&friendly_name);
  result = properties->GetValue(PKEY_Device_FriendlyName, &friendly_name);
  if (FAILED(result) || friendly_name.vt != VT_LPWSTR ||
      friendly_name.pwszVal == nullptr) {
    ::PropVariantClear(&friendly_name);
    *error = "Unable to read the Windows default capture device name: " +
             HResultDescription(result);
    return false;
  }
  const std::wstring label(friendly_name.pwszVal);
  ::PropVariantClear(&friendly_name);

  std::string utf8_device_id;
  std::string utf8_label;
  if (!WideToUtf8(device_id, &utf8_device_id) ||
      !WideToUtf8(label, &utf8_label)) {
    *error = "Unable to convert the Windows default capture device to UTF-8";
    return false;
  }

  flutter::EncodableMap value;
  value[flutter::EncodableValue("id")] =
      flutter::EncodableValue(std::move(utf8_device_id));
  value[flutter::EncodableValue("label")] =
      flutter::EncodableValue(std::move(utf8_label));
  *device = flutter::EncodableValue(std::move(value));
  return true;
}

/// Unregisters and removes one system-audio input channel.
void ShutdownSystemAudioInputChannelInstance(
    const std::shared_ptr<SystemAudioInputMethodChannel>& channel) {
  channel->SetMethodCallHandler(nullptr);
  const auto channel_iterator = std::find(
      g_system_audio_input_channels.begin(),
      g_system_audio_input_channels.end(), channel);
  if (channel_iterator != g_system_audio_input_channels.end()) {
    g_system_audio_input_channels.erase(channel_iterator);
  }
}

}  // namespace

/// Registers the Windows system-audio input channel for one Flutter engine.
void RegisterSystemAudioInputChannel(flutter::FlutterEngine* engine) {
  auto channel =
      std::make_shared<SystemAudioInputMethodChannel>(
          engine->messenger(), kSystemAudioInputChannelName,
          &flutter::StandardMethodCodec::GetInstance());
  channel->SetMethodCallHandler(
      [](const flutter::MethodCall<flutter::EncodableValue>& method_call,
         std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>>
             result) {
        if (method_call.method_name() != kResolveDefaultCaptureDeviceMethod) {
          result->NotImplemented();
          return;
        }
        flutter::EncodableValue device;
        std::string error;
        if (!ResolveDefaultCaptureDevice(&device, &error)) {
          result->Error("WINDOWS_AUDIO_INPUT_ERROR", error);
          return;
        }
        result->Success(std::move(device));
      });
  g_system_audio_input_channels.push_back(channel);
  RegisterOperitEngineChannelShutdown(
      engine,
      [channel]() { ShutdownSystemAudioInputChannelInstance(channel); });
}

/// Releases all system-audio input channels before their engine is destroyed.
void ShutdownSystemAudioInputChannel() {
  while (!g_system_audio_input_channels.empty()) {
    ShutdownSystemAudioInputChannelInstance(
        g_system_audio_input_channels.back());
  }
}
