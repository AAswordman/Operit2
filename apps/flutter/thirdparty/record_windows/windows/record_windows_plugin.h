#ifndef FLUTTER_PLUGIN_RECORD_WINDOWS_PLUGIN_H_
#define FLUTTER_PLUGIN_RECORD_WINDOWS_PLUGIN_H_


#include <flutter/plugin_registrar_windows.h>
#include <flutter/encodable_value.h>
#include <flutter/event_channel.h>
#include <flutter/event_stream_handler.h>
#include <flutter/event_stream_handler_functions.h>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
#include <memory>
#include <mutex>

#include <windows.h>
#include <mfidl.h>
#include <mfapi.h>
#include <mferror.h>

#include "utils.h"
#include "record.h"
#include "main_thread_dispatcher.h"
#include <wrl/client.h>

using namespace flutter;

namespace record_windows {
	typedef flutter::EventSink<flutter::EncodableValue> FlEventSink;
	typedef flutter::StreamHandlerError<flutter::EncodableValue> FlStreamHandlerError;

	using WindowProcDelegate = std::function<std::optional<LRESULT>(HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam)>;
	using WindowProcDelegateRegistrator = std::function<int(WindowProcDelegate delegate)>;
	using WindowProcDelegateUnregistrator = std::function<void(int proc_id)>;

	class RecordWindowsPlugin : public flutter::Plugin {
	public:
		static void RegisterWithRegistrar(flutter::PluginRegistrarWindows* registrar, FlutterDesktopPluginRegistrarRef core_registrar);
		static void ShutdownWithRegistrar(FlutterDesktopPluginRegistrarRef registrar);
		void Shutdown();

		RecordWindowsPlugin(
			BinaryMessenger* messenger,
			WindowProcDelegateRegistrator registrator,
			WindowProcDelegateUnregistrator unregistrator,
			HWND window
		);
		virtual ~RecordWindowsPlugin();

		// Disallow copy and assign.
		RecordWindowsPlugin(const RecordWindowsPlugin&) = delete;
		RecordWindowsPlugin& operator=(const RecordWindowsPlugin&) = delete;

	private:
		BinaryMessenger* const m_binaryMessenger;
		std::unique_ptr<MethodChannel<EncodableValue>> m_method_channel;
		std::shared_ptr<MainThreadDispatcher> m_dispatcher;
		FlutterDesktopPluginRegistrarRef m_registrar = nullptr;
		bool m_shutdown = false;

		// Called when a method is called on this plugin's channel from Dart.
		void HandleMethodCall(const MethodCall<EncodableValue>& method_call,
			std::unique_ptr<MethodResult<EncodableValue>> result);

		HRESULT CreateRecorder(std::string recorderId);
		void RemoveRecorder(const std::string& recorderId);
		Recorder* GetRecorder(std::string recorderId);
		HRESULT ListInputDevices(MethodResult<EncodableValue>& result);

		std::unique_ptr<RecordConfig> InitRecordConfig(const EncodableMap* args);

		std::map<std::string, Microsoft::WRL::ComPtr<Recorder>> m_recorders{};

		// Keep event channels alive for each recorder so StreamHandler pointers
		// stored by Recorder remain valid while the recorder exists.
		std::map<std::string, std::unique_ptr<EventChannel<EncodableValue>>> m_state_event_channels{};
		std::map<std::string, std::unique_ptr<EventChannel<EncodableValue>>> m_record_event_channels{};

		// Called for top-level WindowProc delegation.
		std::optional<LRESULT> HandleWindowProc(HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam);

		// The registrar for this plugin, for registering top-level WindowProc delegates.
		WindowProcDelegateRegistrator m_win_proc_delegate_registrator;
		WindowProcDelegateUnregistrator m_win_proc_delegate_unregistrator;

		// The ID of the WindowProc delegate registration.
		int m_window_proc_id = -1;
	};

}  // namespace record_windows

#endif  // FLUTTER_PLUGIN_RECORD_WINDOWS_PLUGIN_H_
