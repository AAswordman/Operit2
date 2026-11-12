#include "include/operit_system_fonts/operit_system_fonts_plugin.h"
#include <fontconfig/fontconfig.h>
#include <glib/gstdio.h>
#include <cmath>
#include <map>
#include <stdexcept>
#include <string>

struct FontEntry { std::string path; int index; int score; };
struct _OperitSystemFontsPlugin { GObject parent_instance; std::map<std::string, FontEntry>* fonts; };
typedef struct _OperitSystemFontsPlugin OperitSystemFontsPlugin;
typedef struct { GObjectClass parent_class; } OperitSystemFontsPluginClass;
G_DEFINE_TYPE(OperitSystemFontsPlugin, operit_system_fonts_plugin, g_object_get_type())

/// Enumerates Fontconfig's actual readable font files, retaining a regular face per family.
static void enumerate(OperitSystemFontsPlugin* self) {
  if (!FcInit()) throw std::runtime_error("Cannot initialize Fontconfig");
  FcPattern* pattern = FcPatternCreate();
  FcObjectSet* objects = FcObjectSetBuild(FC_FAMILY, FC_FILE, FC_INDEX, FC_WEIGHT, FC_SLANT, nullptr);
  FcFontSet* set = FcFontList(nullptr, pattern, objects);
  FcPatternDestroy(pattern);
  FcObjectSetDestroy(objects);
  if (!set) throw std::runtime_error("Cannot enumerate Fontconfig fonts");
  self->fonts->clear();
  for (int i = 0; i < set->nfont; ++i) {
    FcChar8* family;
    FcChar8* path;
    int index = 0, weight = FC_WEIGHT_REGULAR, slant = FC_SLANT_ROMAN;
    if (FcPatternGetString(set->fonts[i], FC_FAMILY, 0, &family) != FcResultMatch ||
        FcPatternGetString(set->fonts[i], FC_FILE, 0, &path) != FcResultMatch) continue;
    FcPatternGetInteger(set->fonts[i], FC_INDEX, 0, &index);
    FcPatternGetInteger(set->fonts[i], FC_WEIGHT, 0, &weight);
    FcPatternGetInteger(set->fonts[i], FC_SLANT, 0, &slant);
    const int score = std::abs(weight - FC_WEIGHT_REGULAR) + std::abs(slant - FC_SLANT_ROMAN) * 10;
    const std::string name(reinterpret_cast<char*>(family));
    const auto existing = self->fonts->find(name);
    if (existing == self->fonts->end() || score < existing->second.score) {
      (*self->fonts)[name] = {reinterpret_cast<char*>(path), index & 0xffff, score};
    }
  }
  FcFontSetDestroy(set);
}

/// Serves only catalog-owned files and returns exact collection indices.
static void handle_method(FlMethodChannel*, FlMethodCall* call, gpointer data) {
  auto* self = static_cast<OperitSystemFontsPlugin*>(data);
  try {
    const std::string method(fl_method_call_get_name(call));
    if (method == "listFamilies") {
      enumerate(self);
      g_autoptr(FlValue) names = fl_value_new_list();
      for (const auto& entry : *self->fonts) fl_value_append_take(names, fl_value_new_string(entry.first.c_str()));
      fl_method_call_respond_success(call, names, nullptr);
    } else if (method == "readFamily") {
      FlValue* args = fl_method_call_get_args(call);
      if (fl_value_get_type(args) != FL_VALUE_TYPE_STRING) throw std::runtime_error("Expected font family");
      const auto& entry = self->fonts->at(fl_value_get_string(args));
      GStatBuf file_info;
      if (g_stat(entry.path.c_str(), &file_info) != 0 || file_info.st_size <= 0 || file_info.st_size > 256 * 1024 * 1024) throw std::runtime_error("Invalid installed font size");
      gchar* bytes = nullptr;
      gsize length = 0;
      g_autoptr(GError) error = nullptr;
      if (!g_file_get_contents(entry.path.c_str(), &bytes, &length, &error)) throw std::runtime_error(error->message);
      g_autoptr(FlValue) result = fl_value_new_map();
      fl_value_set_string_take(result, "bytes", fl_value_new_uint8_list(reinterpret_cast<uint8_t*>(bytes), length));
      fl_value_set_string_take(result, "index", fl_value_new_int(entry.index));
      g_free(bytes);
      fl_method_call_respond_success(call, result, nullptr);
    } else {
      fl_method_call_respond_not_implemented(call, nullptr);
    }
  } catch (const std::exception& error) {
    fl_method_call_respond_error(call, "device_font_error", error.what(), nullptr, nullptr);
  }
}

/// Releases the native catalog with its owning engine.
static void operit_system_fonts_plugin_dispose(GObject* object) {
  auto* self = reinterpret_cast<OperitSystemFontsPlugin*>(object);
  delete self->fonts;
  self->fonts = nullptr;
  G_OBJECT_CLASS(operit_system_fonts_plugin_parent_class)->dispose(object);
}

/// Installs the GObject lifecycle hook.
static void operit_system_fonts_plugin_class_init(OperitSystemFontsPluginClass* klass) {
  G_OBJECT_CLASS(klass)->dispose = operit_system_fonts_plugin_dispose;
}

/// Allocates the C++ catalog outside GObject's raw instance storage.
static void operit_system_fonts_plugin_init(OperitSystemFontsPlugin* self) {
  self->fonts = new std::map<std::string, FontEntry>();
}

/// Registers the Fontconfig adapter on this Flutter engine.
void operit_system_fonts_plugin_register_with_registrar(FlPluginRegistrar* registrar) {
  auto* plugin = reinterpret_cast<OperitSystemFontsPlugin*>(g_object_new(operit_system_fonts_plugin_get_type(), nullptr));
  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  g_autoptr(FlMethodChannel) channel = fl_method_channel_new(fl_plugin_registrar_get_messenger(registrar), "operit/system_fonts", FL_METHOD_CODEC(codec));
  fl_method_channel_set_method_call_handler(channel, handle_method, g_object_ref(plugin), g_object_unref);
  g_object_unref(plugin);
}
