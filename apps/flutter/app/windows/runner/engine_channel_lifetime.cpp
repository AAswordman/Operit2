#include "engine_channel_lifetime.h"

#include <flutter_plugin_registrar.h>

#include <map>
#include <mutex>
#include <utility>
#include <vector>

namespace {

constexpr char kOperitEngineChannelLifetimePlugin[] =
    "OperitEngineChannelLifetime";

struct EngineChannelCleanup {
  FlutterDesktopPluginRegistrarRef registrar;
  std::vector<OperitEngineChannelShutdown> shutdowns;
};

std::mutex g_engine_channel_cleanup_mutex;
std::map<flutter::FlutterEngine*, EngineChannelCleanup>
    g_engine_channel_cleanups;
std::map<FlutterDesktopPluginRegistrarRef, flutter::FlutterEngine*>
    g_engine_channel_registrars;

/// Runs the cleanup associated with a registrar before its messenger expires.
void OnOperitEngineChannelRegistrarDestroyed(
    FlutterDesktopPluginRegistrarRef registrar) {
  std::vector<OperitEngineChannelShutdown> shutdowns;
  {
    std::lock_guard<std::mutex> lock(g_engine_channel_cleanup_mutex);
    const auto registrar_entry = g_engine_channel_registrars.find(registrar);
    if (registrar_entry == g_engine_channel_registrars.end()) {
      return;
    }
    const auto engine_entry =
        g_engine_channel_cleanups.find(registrar_entry->second);
    if (engine_entry != g_engine_channel_cleanups.end()) {
      shutdowns = std::move(engine_entry->second.shutdowns);
      g_engine_channel_cleanups.erase(engine_entry);
    }
    g_engine_channel_registrars.erase(registrar_entry);
  }
  for (auto& shutdown : shutdowns) {
    shutdown();
  }
}

/// Removes and returns cleanup associated with one engine.
std::vector<OperitEngineChannelShutdown> TakeOperitEngineChannelShutdowns(
    flutter::FlutterEngine* engine) {
  std::lock_guard<std::mutex> lock(g_engine_channel_cleanup_mutex);
  const auto engine_entry = g_engine_channel_cleanups.find(engine);
  if (engine_entry == g_engine_channel_cleanups.end()) {
    return {};
  }
  std::vector<OperitEngineChannelShutdown> shutdowns =
      std::move(engine_entry->second.shutdowns);
  g_engine_channel_registrars.erase(engine_entry->second.registrar);
  g_engine_channel_cleanups.erase(engine_entry);
  return shutdowns;
}

}  // namespace

/// Registers cleanup that runs while the supplied engine messenger is valid.
void RegisterOperitEngineChannelShutdown(
    flutter::FlutterEngine* engine,
    OperitEngineChannelShutdown shutdown) {
  const FlutterDesktopPluginRegistrarRef registrar =
      engine->GetRegistrarForPlugin(kOperitEngineChannelLifetimePlugin);
  bool install_destruction_handler = false;
  {
    std::lock_guard<std::mutex> lock(g_engine_channel_cleanup_mutex);
    auto [entry, inserted] = g_engine_channel_cleanups.emplace(
        engine, EngineChannelCleanup{registrar, {}});
    if (inserted) {
      g_engine_channel_registrars.emplace(registrar, engine);
      install_destruction_handler = true;
    }
    entry->second.shutdowns.push_back(std::move(shutdown));
  }
  if (install_destruction_handler) {
    FlutterDesktopPluginRegistrarSetDestructionHandler(
        registrar, OnOperitEngineChannelRegistrarDestroyed);
  }
}

/// Runs and removes cleanup registered for one live Flutter engine.
void ShutdownOperitEngineChannels(flutter::FlutterEngine* engine) {
  std::vector<OperitEngineChannelShutdown> shutdowns =
      TakeOperitEngineChannelShutdowns(engine);
  for (auto& shutdown : shutdowns) {
    shutdown();
  }
}

/// Runs and removes cleanup registered for every live Flutter engine.
void ShutdownAllOperitEngineChannels() {
  std::vector<OperitEngineChannelShutdown> shutdowns;
  {
    std::lock_guard<std::mutex> lock(g_engine_channel_cleanup_mutex);
    for (auto& entry : g_engine_channel_cleanups) {
      for (auto& shutdown : entry.second.shutdowns) {
        shutdowns.push_back(std::move(shutdown));
      }
    }
    g_engine_channel_cleanups.clear();
    g_engine_channel_registrars.clear();
  }
  for (auto& shutdown : shutdowns) {
    shutdown();
  }
}
