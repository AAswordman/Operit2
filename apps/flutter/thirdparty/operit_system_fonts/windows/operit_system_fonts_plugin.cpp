#include "include/operit_system_fonts/operit_system_fonts_plugin.h"
#include <flutter/method_channel.h>
#include <flutter/plugin_registrar_windows.h>
#include <flutter/standard_method_codec.h>
#include <dwrite.h>
#include <wrl/client.h>
#include <filesystem>
#include <fstream>
#include <map>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace {
using Microsoft::WRL::ComPtr;
using flutter::EncodableValue;
struct FontEntry { std::wstring path; int index; };

/// Stops enumeration on an actual DirectWrite failure.
void Check(HRESULT status) {
  if (FAILED(status)) throw std::runtime_error("DirectWrite error " + std::to_string(status));
}

/// Encodes a native font family name for the Flutter channel.
std::string Utf8(const std::wstring& value) {
  const auto length = WideCharToMultiByte(CP_UTF8, 0, value.data(), static_cast<int>(value.size()), nullptr, 0, nullptr, nullptr);
  std::string result(length, '\0');
  WideCharToMultiByte(CP_UTF8, 0, value.data(), static_cast<int>(value.size()), result.data(), length, nullptr, nullptr);
  return result;
}

class SystemFontsPlugin : public flutter::Plugin {
 public:
  /// Registers a client-local channel and retains the catalog for validated reads.
  explicit SystemFontsPlugin(flutter::BinaryMessenger* messenger) {
    channel_ = std::make_unique<flutter::MethodChannel<EncodableValue>>(
        messenger, "operit/system_fonts", &flutter::StandardMethodCodec::GetInstance());
    channel_->SetMethodCallHandler([this](const auto& call, auto result) {
      try {
        if (call.method_name() == "listFamilies") {
          Enumerate();
          flutter::EncodableList names;
          for (const auto& entry : fonts_) names.emplace_back(entry.first);
          result->Success(EncodableValue(names));
        } else if (call.method_name() == "readFamily") {
          const auto& entry = fonts_.at(std::get<std::string>(*call.arguments()));
          std::ifstream file(std::filesystem::path(entry.path), std::ios::binary | std::ios::ate);
          if (!file) throw std::runtime_error("Cannot open installed font");
          const auto size = file.tellg();
          if (size <= 0 || size > 256 * 1024 * 1024) throw std::runtime_error("Installed font size is invalid");
          std::vector<uint8_t> bytes(static_cast<size_t>(size));
          file.seekg(0);
          if (!file.read(reinterpret_cast<char*>(bytes.data()), static_cast<std::streamsize>(bytes.size()))) throw std::runtime_error("Cannot read installed font");
          result->Success(EncodableValue(flutter::EncodableMap{
            {EncodableValue("bytes"), EncodableValue(bytes)},
            {EncodableValue("index"), EncodableValue(entry.index)},
          }));
        } else {
          result->NotImplemented();
        }
      } catch (const std::exception& error) {
        result->Error("device_font_error", error.what());
      }
    });
  }

 private:
  /// Uses DirectWrite's installed catalog and regular-face collection index.
  void Enumerate() {
    ComPtr<IDWriteFactory> factory;
    Check(DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, __uuidof(IDWriteFactory), reinterpret_cast<IUnknown**>(factory.GetAddressOf())));
    ComPtr<IDWriteFontCollection> collection;
    Check(factory->GetSystemFontCollection(&collection, TRUE));
    std::map<std::string, FontEntry> fonts;
    for (UINT32 i = 0; i < collection->GetFontFamilyCount(); ++i) {
      ComPtr<IDWriteFontFamily> family;
      Check(collection->GetFontFamily(i, &family));
      ComPtr<IDWriteLocalizedStrings> names;
      Check(family->GetFamilyNames(&names));
      UINT32 length;
      Check(names->GetStringLength(0, &length));
      std::wstring name(length + 1, L'\0');
      Check(names->GetString(0, name.data(), length + 1));
      name.resize(length);
      ComPtr<IDWriteFont> font;
      Check(family->GetFirstMatchingFont(DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, &font));
      ComPtr<IDWriteFontFace> face;
      Check(font->CreateFontFace(&face));
      UINT32 count = 0;
      Check(face->GetFiles(&count, nullptr));
      if (count != 1) continue; // Multi-file legacy faces cannot be registered as SFNT.
      ComPtr<IDWriteFontFile> file;
      Check(face->GetFiles(&count, file.GetAddressOf()));
      const void* key;
      UINT32 key_size;
      Check(file->GetReferenceKey(&key, &key_size));
      ComPtr<IDWriteFontFileLoader> loader;
      Check(file->GetLoader(&loader));
      ComPtr<IDWriteLocalFontFileLoader> local;
      Check(loader.As(&local));
      Check(local->GetFilePathLengthFromKey(key, key_size, &length));
      std::wstring path(length + 1, L'\0');
      Check(local->GetFilePathFromKey(key, key_size, path.data(), length + 1));
      path.resize(length);
      fonts.emplace(Utf8(name), FontEntry{path, static_cast<int>(face->GetIndex())});
    }
    fonts_.swap(fonts);
  }

  std::unique_ptr<flutter::MethodChannel<EncodableValue>> channel_;
  std::map<std::string, FontEntry> fonts_;
};
}

/// Attaches the font catalog to each Flutter engine, including secondary windows.
void OperitSystemFontsPluginRegisterWithRegistrar(FlutterDesktopPluginRegistrarRef registrar) {
  auto* windows = flutter::PluginRegistrarManager::GetInstance()->GetRegistrar<flutter::PluginRegistrarWindows>(registrar);
  windows->AddPlugin(std::make_unique<SystemFontsPlugin>(windows->messenger()));
}
