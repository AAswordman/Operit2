#include <flutter/dart_project.h>
#include <flutter/flutter_view_controller.h>
#include <windows.h>

#include <exception>

#include "crash_channel.h"
#include "engine_channel_lifetime.h"
#include "flutter_window.h"
#include "operit_runtime_channel.h"
#include "system_audio_input_channel.h"
#include "utils.h"

/// Presents the native crash dialog for unhandled Windows exceptions.
LONG WINAPI OperitUnhandledExceptionFilter(EXCEPTION_POINTERS*) {
  ShowOperitWindowsCrashScreen("Unhandled Windows exception outside Flutter.");
  return EXCEPTION_EXECUTE_HANDLER;
}

/// Runs the Windows Flutter application message loop.
int APIENTRY wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE prev,
                      _In_ wchar_t *command_line, _In_ int show_command) {
  ::SetUnhandledExceptionFilter(OperitUnhandledExceptionFilter);
  try {
    // Attach to console when present (e.g., 'flutter run') or create a
    // new console when running with a debugger.
    if (!::AttachConsole(ATTACH_PARENT_PROCESS) && ::IsDebuggerPresent()) {
      CreateAndAttachConsole();
    }

    // Initialize COM, so that it is available for use in the library and/or
    // plugins.
    ::CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);

    flutter::DartProject project(L"data");

    std::vector<std::string> command_line_arguments =
        GetCommandLineArguments();

    project.set_dart_entrypoint_arguments(std::move(command_line_arguments));

    FlutterWindow window(project);
    Win32Window::Point origin(10, 10);
    Win32Window::Size size(1280, 720);
    if (!window.Create(L"Operit2", origin, size)) {
      return EXIT_FAILURE;
    }
    window.SetQuitOnClose(true);

    ::MSG msg;
    while (::GetMessage(&msg, nullptr, 0, 0)) {
      ::TranslateMessage(&msg);
      ::DispatchMessage(&msg);
    }

    ShutdownOperitRuntimeChannel();
    ShutdownAllOperitEngineChannels();
    ShutdownSystemAudioInputChannel();
    ShutdownOperitCrashChannel();
    ::CoUninitialize();
    return EXIT_SUCCESS;
  } catch (const std::exception& error) {
    ShowOperitWindowsCrashScreen(error.what());
    ShutdownOperitRuntimeChannel();
    ShutdownAllOperitEngineChannels();
    ShutdownSystemAudioInputChannel();
    ShutdownOperitCrashChannel();
    ::CoUninitialize();
    return EXIT_FAILURE;
  } catch (...) {
    ShowOperitWindowsCrashScreen("Unhandled C++ exception outside Flutter.");
    ShutdownOperitRuntimeChannel();
    ShutdownAllOperitEngineChannels();
    ShutdownSystemAudioInputChannel();
    ShutdownOperitCrashChannel();
    ::CoUninitialize();
    return EXIT_FAILURE;
  }
}
