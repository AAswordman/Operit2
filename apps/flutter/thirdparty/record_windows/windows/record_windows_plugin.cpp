#include "record_windows_plugin.h"
#include <mfreadwrite.h>
#include <Mferror.h>
#include "record_config.h"
#include <flutter/event_stream_handler_functions.h>
#include <mutex>

using namespace flutter;

namespace record_windows {
	namespace {
		std::map<FlutterDesktopPluginRegistrarRef, RecordWindowsPlugin*> plugins;
	}

	HRESULT AttributeGetString(IMFAttributes* pAttributes, const GUID& guid, LPWSTR value)
	{
		HRESULT hr = S_OK;
		UINT32 cchLength = 0;

		hr = pAttributes->GetStringLength(guid, &cchLength);
		if (SUCCEEDED(hr))
		{
			hr = pAttributes->GetString(guid, value, cchLength + 1, &cchLength);
		}

		return hr;
	}

	static void ErrorFromHR(HRESULT hr, MethodResult<EncodableValue>& result)
	{
		_com_error err(hr);
		std::string errorText = Utf8FromUtf16(err.ErrorMessage());

		result.Error("Record", "", EncodableValue(errorText));
	}

	/// Registers one recorder owner and its explicit pre-engine shutdown entry.
	void RecordWindowsPlugin::RegisterWithRegistrar(flutter::PluginRegistrarWindows* registrar, FlutterDesktopPluginRegistrarRef core_registrar) {
		auto plugin = std::make_unique<RecordWindowsPlugin>(
			registrar->messenger(),
			[registrar](auto delegate) {
				return registrar->RegisterTopLevelWindowProcDelegate(delegate);
			},
			[registrar](auto proc_id) {
				registrar->UnregisterTopLevelWindowProcDelegate(proc_id);
			},
			registrar->GetView()->GetNativeWindow()
		);
		plugin->m_registrar = core_registrar;
		plugins.emplace(core_registrar, plugin.get());

		registrar->AddPlugin(std::move(plugin));
	}

	/// Detaches a registered engine's transport before Flutter stops that engine.
	void RecordWindowsPlugin::ShutdownWithRegistrar(FlutterDesktopPluginRegistrarRef registrar) {
		const auto entry = plugins.find(registrar);
		if (entry != plugins.end()) entry->second->Shutdown();
	}

	/// Binds channels and callback dispatch exclusively to this engine.
	RecordWindowsPlugin::RecordWindowsPlugin(
		BinaryMessenger* messenger,
		WindowProcDelegateRegistrator registrator,
		WindowProcDelegateUnregistrator unregistrator,
		HWND window
	):	m_binaryMessenger(messenger),
		m_dispatcher(std::make_shared<MainThreadDispatcher>(window)),
		m_win_proc_delegate_registrator(registrator),
		m_win_proc_delegate_unregistrator(unregistrator) {
		m_method_channel = std::make_unique<MethodChannel<EncodableValue>>(
			m_binaryMessenger, "com.llfbandit.record/messages",
			&StandardMethodCodec::GetInstance());
		m_method_channel->SetMethodCallHandler(
			[this](const auto& call, auto result) {
				HandleMethodCall(call, std::move(result));
			});

		m_window_proc_id = m_win_proc_delegate_registrator(
			[this](HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam) {
				return HandleWindowProc(hwnd, message, wparam, lparam);
			}
		);
	}

	/// Releases native resources without calling an engine that may already be stopped.
	RecordWindowsPlugin::~RecordWindowsPlugin() {
		m_dispatcher->Close();
		for (const auto& entry : m_recorders) entry.second->Dispose();
		plugins.erase(m_registrar);
	}

	/// Cancels callbacks and unregisters channels while the owning engine is alive.
	void RecordWindowsPlugin::Shutdown() {
		if (m_shutdown) return;
		m_shutdown = true;
		m_dispatcher->Close();
		while (!m_recorders.empty()) RemoveRecorder(m_recorders.begin()->first);
		m_method_channel->SetMethodCallHandler(nullptr);
		m_win_proc_delegate_unregistrator(m_window_proc_id);
	}

	/// Delivers callbacks only through their original engine's window procedure.
	std::optional<LRESULT> RecordWindowsPlugin::HandleWindowProc(HWND hwnd,
		UINT message,
		WPARAM wparam,
		LPARAM lparam) {
		if (message == WM_RUN_DELEGATE && hwnd == m_dispatcher->window()) {
			m_dispatcher->RunNext();
			return 0;
		}
		return std::nullopt;
	}

	// Called when a method is called on this plugin's channel from Dart.
	void RecordWindowsPlugin::RecordWindowsPlugin::HandleMethodCall(
		const MethodCall<EncodableValue>& method_call,
		std::unique_ptr<MethodResult<EncodableValue>> result
	) {
		const auto args = method_call.arguments();
		const auto* mapArgs = std::get_if<EncodableMap>(args);
		if (!mapArgs) {
			result->Error("Record", "Call missing parameters");
			return;
		}

		std::string recorderId;
		GetValueFromEncodableMap(mapArgs, "recorderId", recorderId);
		if (recorderId.empty()) {
			result->Error("Record", "Call missing mandatory parameter recorderId");
			return;
		}

		if (method_call.method_name().compare("create") == 0) {
			HRESULT hr = CreateRecorder(recorderId);

			if (SUCCEEDED(hr)) {
				result->Success(EncodableValue(NULL));
			}
			else {
				ErrorFromHR(hr, *result);
			}
			return;
		}

		auto recorder = GetRecorder(recorderId);
		if (!recorder) {
			result->Error(
				"Record",
				"Recorder has not yet been created or has already been disposed."
			);
			return;
		}

		if (method_call.method_name().compare("hasPermission") == 0)
		{
			result->Success(EncodableValue(true));
		}
		else if (method_call.method_name().compare("isPaused") == 0)
		{
			result->Success(EncodableValue(recorder->IsPaused()));
		}
		else if (method_call.method_name().compare("isRecording") == 0)
		{
			result->Success(EncodableValue(recorder->IsRecording()));
		}
		else if (method_call.method_name().compare("pause") == 0)
		{
			HRESULT hr = recorder->Pause();

			if (SUCCEEDED(hr)) { result->Success(EncodableValue()); }
			else { ErrorFromHR(hr, *result); }
		}
		else if (method_call.method_name().compare("resume") == 0)
		{
			HRESULT hr = recorder->Resume();

			if (SUCCEEDED(hr)) { result->Success(EncodableValue()); }
			else { ErrorFromHR(hr, *result); }
		}
		else if (method_call.method_name().compare("start") == 0)
		{
			auto config = InitRecordConfig(mapArgs);

			std::string path;
			GetValueFromEncodableMap(mapArgs, "path", path);

			HRESULT hr = recorder->Start(std::move(config), Utf16FromUtf8(path));

			if (SUCCEEDED(hr)) { result->Success(EncodableValue()); }
			else { ErrorFromHR(hr, *result); }
		}
		else if (method_call.method_name().compare("startStream") == 0)
		{
			auto config = InitRecordConfig(mapArgs);

			HRESULT hr = recorder->StartStream(std::move(config));

			if (SUCCEEDED(hr)) { result->Success(EncodableValue()); }
			else { ErrorFromHR(hr, *result); }
		}
		else if (method_call.method_name().compare("stop") == 0)
		{
			auto recordingPath = recorder->GetRecordingPath();
			HRESULT hr = recorder->Stop();

			if (SUCCEEDED(hr))
			{
				result->Success(recordingPath.empty() ? EncodableValue() : EncodableValue(Utf8FromUtf16(recordingPath)));
			}
			else {
				ErrorFromHR(hr, *result);
			}
		}
		else if (method_call.method_name().compare("cancel") == 0)
		{
			HRESULT hr = recorder->Cancel();

			if (SUCCEEDED(hr))
			{
				result->Success(EncodableValue());
			}
			else
			{
				ErrorFromHR(hr, *result);
			}
		}
		else if (method_call.method_name().compare("dispose") == 0)
		{
			RemoveRecorder(recorderId);
			result->Success(EncodableValue());
		}
		else if (method_call.method_name().compare("getAmplitude") == 0)
		{
			auto amp = recorder->GetAmplitude();

			result->Success(EncodableValue(
				EncodableMap({
					{EncodableValue("current"), EncodableValue(amp["current"])},
					{EncodableValue("max"), EncodableValue(amp["max"])}
					}
				))
			);
		}
		else if (method_call.method_name().compare("isEncoderSupported") == 0)
		{
			std::string encoderName;
			if (!GetValueFromEncodableMap(mapArgs, "encoder", encoderName))
			{
				result->Error("Bad arguments", "Expected encoder name.");
				return;
			}

			bool supported = false;
			HRESULT hr = recorder->isEncoderSupported(encoderName, &supported);

			if (SUCCEEDED(hr))
			{
				result->Success(EncodableValue(supported));
			}
			else
			{
				ErrorFromHR(hr, *result);
			}
		}
		else if (method_call.method_name().compare("listInputDevices") == 0)
		{
			ListInputDevices(*result);
		}
	}

	std::unique_ptr<RecordConfig> RecordWindowsPlugin::InitRecordConfig(const EncodableMap* args)
	{
		std::string path;
		GetValueFromEncodableMap(args, "path", path);
		std::string encoderName;
		GetValueFromEncodableMap(args, "encoder", encoderName);
		int bitRate;
		GetValueFromEncodableMap(args, "bitRate", bitRate);
		int sampleRate;
		GetValueFromEncodableMap(args, "sampleRate", sampleRate);
		int numChannels;
		GetValueFromEncodableMap(args, "numChannels", numChannels);
		EncodableMap device;
		std::string deviceId;
		if (GetValueFromEncodableMap(args, "device", device))
		{
			GetValueFromEncodableMap(&device, "id", deviceId);
		}
		bool autoGain;
		GetValueFromEncodableMap(args, "autoGain", autoGain);
		bool echoCancel;
		GetValueFromEncodableMap(args, "echoCancel", echoCancel);
		bool noiseSuppress;
		GetValueFromEncodableMap(args, "noiseSuppress", noiseSuppress);

		auto config = std::make_unique<RecordConfig>(
			encoderName,
			deviceId,
			bitRate,
			sampleRate,
			numChannels,
			autoGain,
			echoCancel,
			noiseSuppress
		);

		return config;
	}

	/// Registers recorder streams using this engine's messenger and dispatcher.
	HRESULT RecordWindowsPlugin::CreateRecorder(std::string recorderId)
	{
		if (m_recorders.find(recorderId) != m_recorders.end()) return E_INVALIDARG;
		// State event channel
		auto eventChannel = std::make_unique<EventChannel<EncodableValue>>(
			m_binaryMessenger, "com.llfbandit.record/events/" + recorderId,
			&StandardMethodCodec::GetInstance());

		auto eventHandler = new EventStreamHandler<>();
		std::unique_ptr<StreamHandler<EncodableValue>> pStateEventHandler{static_cast<StreamHandler<EncodableValue>*>(eventHandler)};
		eventChannel->SetStreamHandler(std::move(pStateEventHandler));

		// Record stream event channel
		auto eventRecordChannel = std::make_unique<EventChannel<EncodableValue>>(
			m_binaryMessenger, "com.llfbandit.record/eventsRecord/" + recorderId,
			&StandardMethodCodec::GetInstance());

		auto eventRecordHandler = new EventStreamHandler<>();
		std::unique_ptr<StreamHandler<EncodableValue>> pRecordEventHandler{static_cast<StreamHandler<EncodableValue>*>(eventRecordHandler)};
		eventRecordChannel->SetStreamHandler(std::move(pRecordEventHandler));

		Recorder* pRecorder = NULL;

		HRESULT hr = Recorder::CreateInstance(eventHandler->state(), eventRecordHandler->state(), m_dispatcher, &pRecorder);
		if (SUCCEEDED(hr))
		{
			Microsoft::WRL::ComPtr<Recorder> recorder;
			recorder.Attach(pRecorder);
			m_recorders.emplace(recorderId, std::move(recorder));
			m_state_event_channels.emplace(recorderId, std::move(eventChannel));
			m_record_event_channels.emplace(recorderId, std::move(eventRecordChannel));
		}
		else {
			eventChannel->SetStreamHandler(nullptr);
			eventRecordChannel->SetStreamHandler(nullptr);
		}

		return hr;
	}

	/// Stops capture and detaches stream handlers before releasing recorder ownership.
	void RecordWindowsPlugin::RemoveRecorder(const std::string& recorderId)
	{
		m_recorders.at(recorderId)->Dispose();
		m_state_event_channels.at(recorderId)->SetStreamHandler(nullptr);
		m_record_event_channels.at(recorderId)->SetStreamHandler(nullptr);
		m_state_event_channels.erase(recorderId);
		m_record_event_channels.erase(recorderId);
		m_recorders.erase(recorderId);
	}

	Recorder* RecordWindowsPlugin::GetRecorder(std::string recorderId)
	{
		auto searchedRecorder = m_recorders.find(recorderId);
		if (searchedRecorder == m_recorders.end()) {
			return nullptr;
		}
		return searchedRecorder->second.Get();
	}

	HRESULT RecordWindowsPlugin::ListInputDevices(MethodResult<EncodableValue>& result)
	{
		EncodableList devices;

		IMFAttributes* pDeviceAttributes = NULL;
		IMFActivate** ppDevices = NULL;
		UINT32 deviceCount = 0;

		HRESULT hr = MFCreateAttributes(&pDeviceAttributes, 1);
		if (SUCCEEDED(hr))
		{
			// Request audio capture devices
			hr = pDeviceAttributes->SetGUID(
				MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
				MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_AUDCAP_GUID);
		}

		if (SUCCEEDED(hr))
		{
			hr = MFEnumDeviceSources(pDeviceAttributes, &ppDevices, &deviceCount);
		}

		for (UINT32 i = 0; i < deviceCount; i++)
		{
			LPWSTR friendlyName = NULL;
			UINT32 friendlyNameLength = 0;
			LPWSTR id;
			UINT32 idLength = 0;

			hr = ppDevices[i]->GetAllocatedString(MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_AUDCAP_ENDPOINT_ID, &id, &idLength);
			if (SUCCEEDED(hr))
			{
				hr = ppDevices[i]->GetAllocatedString(MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME, &friendlyName, &friendlyNameLength);
			}
			if (SUCCEEDED(hr))
			{
				devices.push_back(EncodableMap({
				{EncodableValue("id"), EncodableValue(toString(id))},
				{EncodableValue("label"), EncodableValue(toString(friendlyName))}
					}));

				CoTaskMemFree(id);
				CoTaskMemFree(friendlyName);
			}
		}

		if (SUCCEEDED(hr))
		{
			result.Success(std::move(EncodableValue(devices)));
		}
		else
		{
			ErrorFromHR(hr, result);
		}

		for (UINT32 i = 0; i < deviceCount; i++)
		{
			SafeRelease(ppDevices[i]);
		}
		SafeRelease(pDeviceAttributes);
		CoTaskMemFree(ppDevices);

		return hr;
	}
}  // namespace record_windows
