import AVFoundation
import CoreBluetooth
import CoreMedia
import Flutter
import Foundation
import Network
import Vision
import UIKit

final class AppleRuntimeChannel: NSObject {
  private static var shared: AppleRuntimeChannel?
  private var channel: FlutterMethodChannel
  private let workQueue = DispatchQueue(label: "operit.runtime.apple", qos: .userInitiated)
  private let watchQueue = DispatchQueue(label: "operit.runtime.apple.watch", qos: .utility)
  private let watchLock = NSLock()
  private var watchPumpRunning = false
  private var handle: UnsafeMutableRawPointer?
  private var audioPlayers: [String: AVAudioPlayer] = [:]
  private var musicPlayer: AVPlayer?
  private var musicSource: String?
  private var musicSourceType: String?
  private var musicTitle: String?
  private var musicArtist: String?
  private var musicVolume: Double = 1.0
  private var musicLoopPlayback = false
  private var musicState = "idle"
  private var musicMessage = "apple music player idle"
  private let speechSynthesizer = AVSpeechSynthesizer()
  private var ttsAudioPlayer: AVAudioPlayer?
  private var ttsAudioPaused = false
  private var ttsPath = ""
  private var configuredRuntimeRoot: URL?
  private var configuredWorkspaceRoot: URL?
  private lazy var bluetooth = AppleBluetoothController { [weak self] topic, data in
    self?.emitHostEvent(topic: topic, data: data)
  }
  private let hostEventQueue = DispatchQueue(label: "operit.runtime.apple.host-events", qos: .utility)
  private let networkMonitor = NWPathMonitor()
  private var hostEventObservers: [NSObjectProtocol] = []
  private var hostEventMonitoringInstalled = false
  private var lastBatteryLow: Bool?
  private var lastCalendarDay = Calendar.current.startOfDay(for: Date())
  private var lastTimeZoneIdentifier = TimeZone.current.identifier

  /// Attaches the process-level Runtime channel to the current Flutter engine.
  static func register(binaryMessenger: FlutterBinaryMessenger) {
    AppleCrashChannel.register(binaryMessenger: binaryMessenger)
    if let shared {
      shared.attach(binaryMessenger: binaryMessenger)
      return
    }
    shared = AppleRuntimeChannel(binaryMessenger: binaryMessenger)
  }

  /// Creates the process-level Runtime channel.
  private init(binaryMessenger: FlutterBinaryMessenger) {
    channel = FlutterMethodChannel(name: "operit/runtime", binaryMessenger: binaryMessenger)
    super.init()
    installMethodHandler()
  }

  /// Rebinds the existing Runtime to a replacement Flutter engine.
  private func attach(binaryMessenger: FlutterBinaryMessenger) {
    channel.setMethodCallHandler(nil)
    channel = FlutterMethodChannel(name: "operit/runtime", binaryMessenger: binaryMessenger)
    installMethodHandler()
  }

  /// Installs method dispatch on the currently attached Flutter channel.
  private func installMethodHandler() {
    channel.setMethodCallHandler { [weak self] call, result in
      self?.handle(call: call, result: result)
    }
  }

  deinit {
    networkMonitor.cancel()
    for observer in hostEventObservers {
      NotificationCenter.default.removeObserver(observer)
    }
    if let handle = handle {
      operit_flutter_bridge_destroy(handle)
    }
  }

  private func handle(call: FlutterMethodCall, result: @escaping FlutterResult) {
    switch call.method {
    case "call":
      callRuntime(call: call, result: result, nativeCall: operit_flutter_bridge_native_call)
    case "pushOpen":
      callRuntime(call: call, result: result, nativeCall: operit_flutter_bridge_push_open)
    case "pushItem":
      callRuntime(call: call, result: result, nativeCall: operit_flutter_bridge_push_item)
    case "pushClose":
      pushClose(call: call, result: result)
    case "watchSnapshot":
      callRuntime(call: call, result: result, nativeCall: operit_flutter_bridge_watch_snapshot)
    case "watchStream":
      watchStream(call: call, result: result)
    case "closeWatchStream":
      closeWatchStream(call: call, result: result)
    case "startWebAccessServer":
      startWebAccessServer(call: call, result: result)
    case "localRuntimeStorageDefaults":
      localRuntimeStorageDefaults(result: result)
    case "localRuntimeStoragePaths":
      localRuntimeStoragePaths(call: call, result: result)
    case "setLocalRuntimeStorage":
      setLocalRuntimeStorage(call: call, result: result)
    case "stopWebAccessServer":
      runRuntime(result: result) { handle in
        self.takeString(operit_flutter_bridge_stop_web_access_server(handle))
      }
    case "discoverDevices":
      discoverDevices(call: call, result: result)
    case "remotePairStart":
      remotePairStart(call: call, result: result)
    case "remotePairFinish":
      remotePairFinish(call: call, result: result)
    case "ownerSystemCaptureScreenshot":
      ownerSystemCaptureScreenshot(result: result)
    case "ownerSystemRecognizeText":
      ownerSystemRecognizeText(call: call, result: result)
    case "ownerAudioPlay":
      ownerAudioPlay(call: call, result: result)
    case "ownerMusicPlayback":
      ownerMusicPlayback(call: call, result: result)
    case "ownerBluetooth":
      ownerBluetooth(call: call, result: result)
    case "ownerTtsSynthesize":
      ownerTtsSynthesize(call: call, result: result)
    case "ownerTtsPlayback":
      ownerTtsPlayback(call: call, result: result)
    case "ownerLocalInference":
      ownerLocalInference(call: call, result: result)
    default:
      result(FlutterMethodNotImplemented)
    }
  }

  private func ensureRuntimeHandle() throws -> UnsafeMutableRawPointer {
    if let handle = handle {
      return handle
    }
    guard let runtimeRoot = configuredRuntimeRoot,
          let workspaceRoot = configuredWorkspaceRoot else {
      throw RuntimeChannelError.createFailed("Runtime and workspace roots are not configured")
    }
    guard let created = operit_flutter_bridge_create_with_storage_roots(
      runtimeRoot.path,
      workspaceRoot.path
    ) else {
      let error = takeString(operit_flutter_bridge_create_error())
      throw RuntimeChannelError.createFailed(error)
    }
    handle = created
    installHostEventMonitoring()
    return created
  }

  /// Installs iOS network, battery, session, and Bluetooth event producers once.
  private func installHostEventMonitoring() {
    guard !hostEventMonitoringInstalled else { return }
    hostEventMonitoringInstalled = true
    networkMonitor.pathUpdateHandler = { [weak self] path in
      self?.emitNetworkPath(path)
    }
    networkMonitor.start(queue: hostEventQueue)
    UIDevice.current.isBatteryMonitoringEnabled = true
    hostEventObservers.append(
      NotificationCenter.default.addObserver(
        forName: UIDevice.batteryLevelDidChangeNotification,
        object: nil,
        queue: nil
      ) { [weak self] _ in self?.emitIosBatteryState() }
    )
    hostEventObservers.append(
      NotificationCenter.default.addObserver(
        forName: UIDevice.batteryStateDidChangeNotification,
        object: nil,
        queue: nil
      ) { [weak self] _ in self?.emitIosBatteryState() }
    )
    hostEventObservers.append(
      NotificationCenter.default.addObserver(
        forName: UIApplication.protectedDataWillBecomeUnavailableNotification,
        object: nil,
        queue: nil
      ) { [weak self] _ in
        self?.emitHostEvent(topic: "system.session.lock", data: ["locked": true])
      }
    )
    hostEventObservers.append(
      NotificationCenter.default.addObserver(
        forName: UIApplication.protectedDataDidBecomeAvailableNotification,
        object: nil,
        queue: nil
      ) { [weak self] _ in
        self?.emitHostEvent(topic: "system.session.unlock", data: ["locked": false])
        self?.emitHostEvent(topic: "system.user.present", data: ["present": true])
      }
    )
    hostEventObservers.append(
      NotificationCenter.default.addObserver(
        forName: UIApplication.significantTimeChangeNotification,
        object: nil,
        queue: nil
      ) { [weak self] _ in self?.emitIosClockChanges() }
    )
    hostEventObservers.append(
      NotificationCenter.default.addObserver(
        forName: AVAudioSession.routeChangeNotification,
        object: AVAudioSession.sharedInstance(),
        queue: nil
      ) { [weak self] _ in self?.emitIosHeadsetState() }
    )
    _ = bluetooth
    emitIosBatteryState()
    emitIosHeadsetState()
  }

  /// Emits the canonical iOS battery and external-power topic data.
  private func emitIosBatteryState() {
    let device = UIDevice.current
    let level = device.batteryLevel >= 0 ? Double(device.batteryLevel * 100) : nil
    let charging: Bool?
    switch device.batteryState {
    case .charging, .full:
      charging = true
    case .unplugged:
      charging = false
    case .unknown:
      charging = nil
    @unknown default:
      charging = nil
    }
    if let level {
      let low = level <= 20
      if lastBatteryLow != low {
        lastBatteryLow = low
        emitHostEvent(
          topic: low ? "system.battery.low" : "system.battery.okay",
          data: ["low": low, "level": level, "charging": charging ?? NSNull()]
        )
      }
    }
    if let charging {
      emitHostEvent(
        topic: charging ? "system.power.connected" : "system.power.disconnected",
        data: [
          "connected": charging,
          "source": charging ? "unknown" : "battery",
          "batteryLevel": level ?? NSNull(),
        ]
      )
    }
  }

  /// Emits canonical clock, date, and timezone changes reported by iOS.
  private func emitIosClockChanges() {
    let now = Date()
    let day = Calendar.current.startOfDay(for: now)
    let timeZoneIdentifier = TimeZone.current.identifier
    let data: [String: Any] = [
      "timestampMillis": now.timeIntervalSince1970 * 1000,
      "timezone": timeZoneIdentifier,
    ]
    emitHostEvent(topic: "system.time.tick", data: data)
    if day != lastCalendarDay {
      lastCalendarDay = day
      emitHostEvent(topic: "system.date.changed", data: data)
    }
    if timeZoneIdentifier != lastTimeZoneIdentifier {
      lastTimeZoneIdentifier = timeZoneIdentifier
      emitHostEvent(topic: "system.timezone.changed", data: data)
    }
  }

  /// Emits the canonical headset state derived from the active iOS audio route.
  private func emitIosHeadsetState() {
    let route = AVAudioSession.sharedInstance().currentRoute
    let headsetOutput = route.outputs.first { output in
      switch output.portType {
      case .headphones, .bluetoothA2DP, .bluetoothHFP, .bluetoothLE:
        return true
      default:
        return false
      }
    }
    let hasMicrophone = route.inputs.contains { input in
      switch input.portType {
      case .headsetMic, .bluetoothHFP, .bluetoothLE:
        return true
      default:
        return false
      }
    }
    emitHostEvent(
      topic: "system.headset.plug",
      data: [
        "connected": headsetOutput != nil,
        "deviceName": headsetOutput?.portName ?? NSNull(),
        "hasMicrophone": headsetOutput == nil ? NSNull() : hasMicrophone,
      ]
    )
  }

  /// Converts one Apple Network path into the shared network-change structure.
  private func emitNetworkPath(_ path: NWPath) {
    let networkType: String
    if path.status != .satisfied {
      networkType = "none"
    } else if path.usesInterfaceType(.wifi) {
      networkType = "wifi"
    } else if path.usesInterfaceType(.cellular) {
      networkType = "cellular"
    } else if path.usesInterfaceType(.wiredEthernet) {
      networkType = "ethernet"
    } else if path.usesInterfaceType(.other) {
      networkType = "other"
    } else {
      networkType = "other"
    }
    let interfaceName = path.availableInterfaces.first(where: { path.usesInterfaceType($0.type) })?.name
    emitHostEvent(
      topic: "system.network.changed",
      data: [
        "connected": path.status == .satisfied,
        "networkType": networkType,
        "metered": path.isExpensive,
        "interfaceName": interfaceName ?? NSNull(),
      ]
    )
  }

  /// Serializes and forwards one canonical iOS event through the existing native bridge.
  private func emitHostEvent(topic: String, data: [String: Any]) {
    workQueue.async { [weak self] in
      guard let self, let handle = self.handle else { return }
      let event: [String: Any] = [
        "domain": "host",
        "source": "ios.system",
        "topic": topic,
        "platform": "ios",
        "payload": data,
        "occurredAtMillis": Int64(Date().timeIntervalSince1970 * 1000),
      ]
      do {
        let encoded = try JSONSerialization.data(withJSONObject: event)
        guard let json = String(data: encoded, encoding: .utf8) else {
          throw RuntimeChannelError.invalidState("iOS host event JSON is not UTF-8")
        }
        let response = json.withCString { pointer in
          self.takeString(operit_flutter_bridge_emit_runtime_event(handle, pointer))
        }
        guard let value = try JSONSerialization.jsonObject(with: Data(response.utf8)) as? [String: Any],
              value["ok"] as? Bool == true else {
          throw RuntimeChannelError.invalidState("iOS host event delivery failed: \(response)")
        }
      } catch {
        NSLog("Operit iOS host event failed: %@", error.localizedDescription)
      }
    }
  }

  /// Returns the default Apple runtime and workspace roots.
  private func defaultStorageRoots() -> (runtime: URL, workspace: URL) {
    let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
      .appendingPathComponent("Operit2", isDirectory: true)
    return (
      base.appendingPathComponent("runtime", isDirectory: true),
      base.appendingPathComponent("workspaces", isDirectory: true)
    )
  }

  /// Resolves one required Flutter-provided storage root.
  private func absoluteDirectory(from value: Any?, label: String) throws -> URL {
    guard let value = value as? String else {
      throw RuntimeChannelError.createFailed("\(label) must be a string")
    }
    let path = value.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !path.isEmpty else {
      throw RuntimeChannelError.createFailed("\(label) is required")
    }
    guard NSString(string: path).isAbsolutePath else {
      throw RuntimeChannelError.createFailed("\(label) must be an absolute path")
    }
    return URL(fileURLWithPath: path).standardizedFileURL
  }

  /// Returns the platform default runtime and workspace roots.
  private func localRuntimeStorageDefaults(result: @escaping FlutterResult) {
    let roots = defaultStorageRoots()
    result([
      "runtimeRoot": roots.runtime.path,
      "workspaceRoot": roots.workspace.path,
    ])
  }

  /// Returns normalized local runtime storage paths for requested roots.
  private func localRuntimeStoragePaths(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let arguments = call.arguments as? [String: Any?] else {
      result(FlutterError(code: "INVALID_ARGS", message: "localRuntimeStoragePaths expects arguments", details: nil))
      return
    }
    do {
      let runtimeRoot = try absoluteDirectory(from: arguments["runtimeRoot"] ?? nil, label: "runtimeRoot")
      let workspaceRoot = try absoluteDirectory(from: arguments["workspaceRoot"] ?? nil, label: "workspaceRoot")
      result([
        "runtimeRoot": runtimeRoot.path,
        "workspaceRoot": workspaceRoot.path,
      ])
    } catch {
      result(FlutterError(code: "INVALID_ARGS", message: error.localizedDescription, details: nil))
    }
  }

  /// Installs storage roots and accepts repeated identical configuration.
  private func setLocalRuntimeStorage(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let arguments = call.arguments as? [String: Any?] else {
      result(FlutterError(code: "INVALID_ARGS", message: "setLocalRuntimeStorage expects arguments", details: nil))
      return
    }
    do {
      let runtimeRoot = try absoluteDirectory(from: arguments["runtimeRoot"] ?? nil, label: "runtimeRoot")
      let workspaceRoot = try absoluteDirectory(from: arguments["workspaceRoot"] ?? nil, label: "workspaceRoot")
      if handle != nil {
        if configuredRuntimeRoot == runtimeRoot && configuredWorkspaceRoot == workspaceRoot {
          result(nil)
          return
        }
        result(FlutterError(code: "RUNTIME_ALREADY_CREATED", message: "Runtime and workspace roots cannot change after runtime creation", details: nil))
        return
      }
      configuredRuntimeRoot = runtimeRoot
      configuredWorkspaceRoot = workspaceRoot
      result(nil)
    } catch {
      result(FlutterError(code: "INVALID_ARGS", message: error.localizedDescription, details: nil))
    }
  }

  private func runRuntime(result: @escaping FlutterResult, _ body: @escaping (UnsafeMutableRawPointer) throws -> String) {
    workQueue.async {
      do {
        let handle = try self.ensureRuntimeHandle()
        let response = try body(handle)
        DispatchQueue.main.async { result(response) }
      } catch {
        DispatchQueue.main.async {
          result(FlutterError(code: "OPERIT_RUNTIME_ERROR", message: error.localizedDescription, details: nil))
        }
      }
    }
  }

  /// Runs one binary Link operation and returns Flutter typed data.
  private func runRuntimeBytes(result: @escaping FlutterResult, _ body: @escaping (UnsafeMutableRawPointer) throws -> Data) {
    workQueue.async {
      do {
        let handle = try self.ensureRuntimeHandle()
        let response = try body(handle)
        DispatchQueue.main.async { result(FlutterStandardTypedData(bytes: response)) }
      } catch {
        DispatchQueue.main.async {
          result(FlutterError(code: "OPERIT_RUNTIME_ERROR", message: error.localizedDescription, details: nil))
        }
      }
    }
  }

  private func callRuntime(
    call: FlutterMethodCall,
    result: @escaping FlutterResult,
    nativeCall: @escaping (UnsafeMutableRawPointer?, UnsafePointer<UInt8>?, UInt) -> OperitByteBuffer
  ) {
    guard let request = (call.arguments as? FlutterStandardTypedData)?.data else {
      result(FlutterError(code: "INVALID_ARGS", message: "\(call.method) expects MessagePack bytes", details: nil))
      return
    }
    runRuntimeBytes(result: result) { handle in
      request.withUnsafeBytes { bytes in
        self.takeBytes(nativeCall(handle, bytes.bindMemory(to: UInt8.self).baseAddress, UInt(bytes.count)))
      }
    }
  }

  private func watchStream(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let request = (call.arguments as? FlutterStandardTypedData)?.data else {
      result(FlutterError(code: "INVALID_ARGS", message: "watchStream expects MessagePack bytes", details: nil))
      return
    }
    runRuntimeBytes(result: result) { handle in
      let response = request.withUnsafeBytes { bytes in
        self.takeBytes(operit_flutter_bridge_watch_stream(handle, bytes.bindMemory(to: UInt8.self).baseAddress, UInt(bytes.count)))
      }
      self.ensureWatchPump()
      return response
    }
  }

  private func closeWatchStream(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let subscriptionId = call.arguments as? String else {
      result(FlutterError(code: "INVALID_ARGS", message: "closeWatchStream expects a subscription id", details: nil))
      return
    }
    runRuntimeBytes(result: result) { handle in
      self.takeBytes(operit_flutter_bridge_close_watch_stream(handle, subscriptionId))
    }
  }

  /// Closes one local Link push stream.
  private func pushClose(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let pushId = call.arguments as? String else {
      result(FlutterError(code: "INVALID_ARGS", message: "pushClose expects a push id", details: nil))
      return
    }
    runRuntimeBytes(result: result) { handle in
      self.takeBytes(operit_flutter_bridge_push_close(handle, pushId))
    }
  }

  private func ensureWatchPump() {
    watchLock.lock()
    if watchPumpRunning {
      watchLock.unlock()
      return
    }
    watchPumpRunning = true
    watchLock.unlock()
    watchQueue.async {
      while true {
        self.watchLock.lock()
        let running = self.watchPumpRunning
        self.watchLock.unlock()
        if !running {
          return
        }
        do {
          let handle = try self.ensureRuntimeHandle()
          let frameBuffer = operit_flutter_bridge_next_watch_channel_event(handle)
          guard frameBuffer.ptr != nil else {
            self.stopWatchPump()
            return
          }
          let frame = self.takeBytes(frameBuffer)
          DispatchQueue.main.async {
            self.channel.invokeMethod("watchChannelEvent", arguments: FlutterStandardTypedData(bytes: frame))
          }
        } catch {
          self.stopWatchPump()
          return
        }
      }
    }
  }

  private func stopWatchPump() {
    watchLock.lock()
    watchPumpRunning = false
    watchLock.unlock()
  }

  private func startWebAccessServer(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let args = call.arguments as? [String: Any],
      let bindAddress = args["bindAddress"] as? String,
      let token = args["token"] as? String,
      let shutdownToken = args["shutdownToken"] as? String,
      let webRoot = args["webRoot"] as? String,
      let deviceInfo = args["deviceInfo"] as? String,
      let enableWebAccess = args["enableWebAccess"] as? String,
      let enableDiscovery = args["enableDiscovery"] as? String
    else {
      result(FlutterError(code: "INVALID_ARGS", message: "startWebAccessServer arguments are incomplete", details: nil))
      return
    }
    runRuntime(result: result) { handle in
      self.takeString(operit_flutter_bridge_start_web_access_server(
        handle,
        bindAddress,
        token,
        shutdownToken,
        webRoot,
        deviceInfo,
        enableWebAccess,
        enableDiscovery
      ))
    }
  }

  private func discoverDevices(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let args = call.arguments as? [String: Any],
      let timeoutMs = args["timeoutMs"] as? NSNumber
    else {
      result(FlutterError(code: "INVALID_ARGS", message: "discoverDevices expects timeoutMs", details: nil))
      return
    }
    runRuntime(result: result) { handle in
      self.takeString(operit_flutter_bridge_discover_devices(handle, timeoutMs.stringValue))
    }
  }

  private func remotePairStart(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let args = call.arguments as? [String: Any],
      let baseUrl = args["baseUrl"] as? String,
      let tokenHash = args["tokenHash"] as? String,
      let clientDeviceInfo = args["clientDeviceInfo"] as? String
    else {
      result(FlutterError(code: "INVALID_ARGS", message: "remotePairStart expects baseUrl, tokenHash and clientDeviceInfo", details: nil))
      return
    }
    runRuntime(result: result) { handle in
      self.takeString(operit_flutter_bridge_remote_pair_start(handle, baseUrl, tokenHash, clientDeviceInfo))
    }
  }

  private func remotePairFinish(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let args = call.arguments as? [String: Any],
      let pairingId = args["pairingId"] as? String,
      let pairingCode = args["pairingCode"] as? String
    else {
      result(FlutterError(code: "INVALID_ARGS", message: "remotePairFinish expects pairingId and pairingCode", details: nil))
      return
    }
    runRuntime(result: result) { handle in
      self.takeString(operit_flutter_bridge_remote_pair_finish(handle, pairingId, pairingCode))
    }
  }

  private func ownerSystemCaptureScreenshot(result: @escaping FlutterResult) {
    #if os(iOS)
    result(FlutterError(code: "OWNER_SYSTEM_CAPTURE_SCREENSHOT_ERROR", message: "iOS screenshot capture is not available to this native host", details: nil))
    #else
    result(FlutterError(code: "OWNER_SYSTEM_CAPTURE_SCREENSHOT_ERROR", message: "macOS screenshot capture is handled by the Rust system host", details: nil))
    #endif
  }

  private func ownerSystemRecognizeText(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let payload = call.arguments as? [String: Any],
      let imagePath = payload["imagePath"] as? String
    else {
      result(FlutterError(code: "INVALID_ARGS", message: "ownerSystemRecognizeText expects imagePath", details: nil))
      return
    }
    workQueue.async {
      do {
        let text = try self.recognizeText(imagePath: imagePath)
        DispatchQueue.main.async { result(["text": text]) }
      } catch {
        DispatchQueue.main.async {
          result(FlutterError(code: "OWNER_SYSTEM_RECOGNIZE_TEXT_ERROR", message: error.localizedDescription, details: nil))
        }
      }
    }
  }

  private func ownerAudioPlay(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let payload = call.arguments as? [String: Any],
      let path = payload["path"] as? String
    else {
      result(FlutterError(code: "INVALID_ARGS", message: "ownerAudioPlay expects path", details: nil))
      return
    }
    do {
      let url = URL(fileURLWithPath: path)
      let player = try AVAudioPlayer(contentsOf: url)
      let key = UUID().uuidString
      audioPlayers[key] = player
      player.delegate = self
      player.prepareToPlay()
      player.play()
      result(["path": url.path, "started": true, "details": "av_audio_player_started"])
    } catch {
      result(FlutterError(code: "OWNER_AUDIO_PLAY_ERROR", message: error.localizedDescription, details: nil))
    }
  }

  private func ownerMusicPlayback(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let payload = call.arguments as? [String: Any],
      let command = payload["command"] as? String
    else {
      result(FlutterError(code: "INVALID_ARGS", message: "ownerMusicPlayback expects command", details: nil))
      return
    }
    do {
      result(try musicPlayback(command: command, payload: payload))
    } catch {
      result(FlutterError(code: "OWNER_MUSIC_PLAYBACK_ERROR", message: error.localizedDescription, details: nil))
    }
  }

  private func ownerBluetooth(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let payload = call.arguments as? [String: Any],
      let command = payload["command"] as? String
    else {
      result(FlutterError(code: "INVALID_ARGS", message: "ownerBluetooth expects command", details: nil))
      return
    }
    workQueue.async {
      do {
        let params = try self.dictionaryFromJson(payload["paramsJson"] as? String)
        let value = try self.bluetooth.handle(command: command, params: params)
        DispatchQueue.main.async {
          result(["resultJson": self.jsonString(value)])
        }
      } catch {
        DispatchQueue.main.async {
          result(FlutterError(code: "OWNER_BLUETOOTH_ERROR", message: error.localizedDescription, details: nil))
        }
      }
    }
  }

  private func ownerTtsSynthesize(call: FlutterMethodCall, result: @escaping FlutterResult) {
    result(FlutterError(code: "OWNER_TTS_SYNTHESIZE_ERROR", message: "Apple TTS file synthesis is not implemented", details: nil))
  }

  /// Handles owner-host local inference commands.
  private func ownerLocalInference(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let payload = call.arguments as? [String: Any] else {
      result(FlutterError(code: "INVALID_ARGS", message: "ownerLocalInference expects payload", details: nil))
      return
    }
    workQueue.async {
      do {
        let response = try AppleLocalInferenceRunner.shared.run(payload: payload)
        DispatchQueue.main.async {
          result(response)
        }
      } catch {
        DispatchQueue.main.async {
          result(FlutterError(code: "OWNER_LOCAL_INFERENCE_ERROR", message: error.localizedDescription, details: nil))
        }
      }
    }
  }

  /// Handles owner-host system speech playback commands.
  private func ownerTtsPlayback(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let payload = call.arguments as? [String: Any],
      let command = payload["command"] as? String
    else {
      result(FlutterError(code: "INVALID_ARGS", message: "ownerTtsPlayback expects command", details: nil))
      return
    }
    switch command {
    case "play":
      guard let path = payload["audioPath"] as? String, !path.isEmpty else {
        result(FlutterError(code: "INVALID_ARGS", message: "ownerTtsPlayback play expects audioPath", details: nil))
        return
      }
      do {
        speechSynthesizer.stopSpeaking(at: .immediate)
        stopTtsAudioPlayback()
        let player = try AVAudioPlayer(contentsOf: URL(fileURLWithPath: path))
        player.delegate = self
        guard player.prepareToPlay(), player.play() else {
          throw RuntimeChannelError.invalidArgs("Apple TTS audio player failed to start")
        }
        ttsAudioPlayer = player
        ttsAudioPaused = false
        ttsPath = path
        result(ttsStatus(details: "apple_tts_audio_started"))
      } catch {
        result(FlutterError(code: "OWNER_TTS_PLAYBACK_ERROR", message: error.localizedDescription, details: nil))
      }
    case "speak":
      guard let text = payload["text"] as? String,
        let speed = payload["speed"] as? NSNumber,
        let pitch = payload["pitch"] as? NSNumber
      else {
        result(FlutterError(code: "INVALID_ARGS", message: "ownerTtsPlayback speak expects text, speed and pitch", details: nil))
        return
      }
      let utterance = AVSpeechUtterance(string: text)
      do {
        try configureSpeechUtterance(utterance, payload: payload, speed: speed, pitch: pitch)
      } catch {
        result(FlutterError(code: "OWNER_TTS_PLAYBACK_ERROR", message: error.localizedDescription, details: nil))
        return
      }
      let interrupt = (payload["interrupt"] as? Bool) == true
      if !interrupt, ttsAudioPlayer != nil || speechSynthesizer.isSpeaking {
        result(FlutterError(code: "OWNER_TTS_PLAYBACK_ERROR", message: "Apple TTS playback is busy", details: nil))
        return
      }
      if interrupt {
        speechSynthesizer.stopSpeaking(at: .immediate)
        stopTtsAudioPlayback()
      }
      ttsPath = "apple-tts"
      speechSynthesizer.speak(utterance)
      result(ttsStatus(details: "apple_tts_started"))
    case "pause":
      if let player = ttsAudioPlayer {
        player.pause()
        ttsAudioPaused = true
      } else {
        speechSynthesizer.pauseSpeaking(at: .word)
      }
      result(ttsStatus(details: "apple_tts_paused"))
    case "resume":
      if let player = ttsAudioPlayer {
        player.play()
        ttsAudioPaused = false
      } else {
        speechSynthesizer.continueSpeaking()
      }
      result(ttsStatus(details: "apple_tts_resumed"))
    case "stop":
      speechSynthesizer.stopSpeaking(at: .immediate)
      stopTtsAudioPlayback()
      result(ttsStatus(details: "apple_tts_stopped"))
    case "status":
      result(ttsStatus(details: "apple_tts_status"))
    default:
      result(FlutterError(code: "OWNER_TTS_PLAYBACK_ERROR", message: "unsupported tts command: \(command)", details: nil))
    }
  }

  /// Applies validated cross-platform voice settings to one Apple utterance.
  private func configureSpeechUtterance(
    _ utterance: AVSpeechUtterance,
    payload: [String: Any],
    speed: NSNumber,
    pitch: NSNumber
  ) throws {
    let speedMultiplier = speed.doubleValue
    guard speedMultiplier.isFinite, speedMultiplier > 0 else {
      throw RuntimeChannelError.invalidArgs("tts speed must be positive and finite")
    }
    let pitchMultiplier = pitch.doubleValue
    guard pitchMultiplier.isFinite, pitchMultiplier >= 0.5, pitchMultiplier <= 2.0 else {
      throw RuntimeChannelError.invalidArgs("tts pitch must be between 0.5 and 2.0")
    }
    let voiceName = (payload["voice"] as? String)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
    let locale = (payload["locale"] as? String)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
    if !voiceName.isEmpty {
      guard let selectedVoice = AVSpeechSynthesisVoice.speechVoices().first(where: {
        $0.identifier == voiceName || $0.name == voiceName
      }) else {
        throw RuntimeChannelError.invalidArgs("tts voice not found: \(voiceName)")
      }
      if !locale.isEmpty,
        Locale.canonicalIdentifier(from: selectedVoice.language)
          != Locale.canonicalIdentifier(from: locale)
      {
        throw RuntimeChannelError.invalidArgs(
          "tts voice language \(selectedVoice.language) does not match locale \(locale)"
        )
      }
      utterance.voice = selectedVoice
    } else if !locale.isEmpty {
      guard let selectedVoice = AVSpeechSynthesisVoice(language: locale) else {
        throw RuntimeChannelError.invalidArgs("tts locale not supported: \(locale)")
      }
      utterance.voice = selectedVoice
    }
    let scaledRate = Double(AVSpeechUtteranceDefaultSpeechRate) * speedMultiplier
    guard scaledRate >= Double(AVSpeechUtteranceMinimumSpeechRate),
      scaledRate <= Double(AVSpeechUtteranceMaximumSpeechRate)
    else {
      throw RuntimeChannelError.invalidArgs("tts speed is outside the Apple speech rate range")
    }
    utterance.rate = Float(scaledRate)
    utterance.pitchMultiplier = Float(pitchMultiplier)
  }

  private func recognizeText(imagePath: String) throws -> String {
    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    let handler = VNImageRequestHandler(url: URL(fileURLWithPath: imagePath), options: [:])
    try handler.perform([request])
    guard let results = request.results else {
      return ""
    }
    return results.compactMap { $0.topCandidates(1).first?.string }.joined(separator: "\n")
  }

  private func musicPlayback(command: String, payload: [String: Any]) throws -> [String: Any?] {
    switch command {
    case "play":
      guard let source = payload["source"] as? String,
        let sourceType = payload["sourceType"] as? String
      else {
        throw RuntimeChannelError.invalidArgs("source and sourceType are required")
      }
      let url: URL
      switch sourceType {
      case "path":
        url = URL(fileURLWithPath: source)
      case "uri", "url":
        guard let parsed = URL(string: source) else {
          throw RuntimeChannelError.invalidArgs("music source URL is invalid")
        }
        url = parsed
      default:
        throw RuntimeChannelError.invalidArgs("unsupported music sourceType: \(sourceType)")
      }
      let player = AVPlayer(url: url)
      musicPlayer = player
      musicSource = source
      musicSourceType = sourceType
      musicTitle = payload["title"] as? String
      musicArtist = payload["artist"] as? String
      guard let volume = payload["volume"] as? NSNumber,
        let loopPlayback = payload["loopPlayback"] as? Bool,
        let position = payload["positionMs"] as? NSNumber
      else {
        throw RuntimeChannelError.invalidArgs("volume, loopPlayback and positionMs are required")
      }
      musicVolume = volume.doubleValue
      musicLoopPlayback = loopPlayback
      musicState = "playing"
      musicMessage = "apple music playback started"
      player.volume = Float(musicVolume)
      let startPositionMs = position.int64Value
      if startPositionMs > 0 {
        player.seek(to: CMTime(value: CMTimeValue(startPositionMs), timescale: 1000))
      }
      player.play()
      return musicStatus(message: musicMessage)
    case "pause":
      guard let player = musicPlayer else {
        throw RuntimeChannelError.invalidState("apple music player is not initialized")
      }
      player.pause()
      musicState = "paused"
      return musicStatus(message: "apple music playback paused")
    case "resume":
      guard let player = musicPlayer else {
        throw RuntimeChannelError.invalidState("apple music player is not initialized")
      }
      player.play()
      musicState = "playing"
      return musicStatus(message: "apple music playback resumed")
    case "stop":
      musicPlayer?.pause()
      musicPlayer = nil
      musicState = "stopped"
      return musicStatus(message: "apple music playback stopped")
    case "seek":
      guard let player = musicPlayer else {
        throw RuntimeChannelError.invalidState("apple music player is not initialized")
      }
      guard let position = payload["positionMs"] as? NSNumber else {
        throw RuntimeChannelError.invalidArgs("positionMs is required")
      }
      let positionMs = position.int64Value
      player.seek(to: CMTime(value: CMTimeValue(max(positionMs, 0)), timescale: 1000))
      return musicStatus(message: "apple music playback seeked")
    case "set_volume":
      guard let player = musicPlayer else {
        throw RuntimeChannelError.invalidState("apple music player is not initialized")
      }
      guard let volume = payload["volume"] as? NSNumber else {
        throw RuntimeChannelError.invalidArgs("volume is required")
      }
      musicVolume = volume.doubleValue
      player.volume = Float(musicVolume)
      return musicStatus(message: "apple music playback volume changed")
    case "status":
      return musicStatus(message: "apple music player status")
    default:
      throw RuntimeChannelError.invalidArgs("unsupported music command: \(command)")
    }
  }

  private func musicStatus(message: String) -> [String: Any?] {
    let positionSeconds: Double
    if let player = musicPlayer {
      positionSeconds = player.currentTime().seconds
    } else {
      positionSeconds = 0
    }
    let durationSeconds = musicPlayer?.currentItem?.duration.seconds
    return [
      "state": musicState,
      "source": musicSource,
      "sourceType": musicSourceType,
      "title": musicTitle,
      "artist": musicArtist,
      "durationMs": durationSeconds?.isFinite == true ? Int64(durationSeconds! * 1000) : nil,
      "positionMs": positionSeconds.isFinite ? Int64(positionSeconds * 1000) : 0,
      "bufferedPositionMs": positionSeconds.isFinite ? Int64(positionSeconds * 1000) : 0,
      "volume": musicVolume,
      "loopPlayback": musicLoopPlayback,
      "message": message,
    ]
  }

  /// Builds an authoritative Apple speech status snapshot.
  private func ttsStatus(details: String) -> [String: Any] {
    let audioActive = ttsAudioPlayer != nil
    return [
      "path": ttsPath,
      "active": audioActive || speechSynthesizer.isSpeaking,
      "paused": audioActive ? ttsAudioPaused : speechSynthesizer.isPaused,
      "details": details,
    ]
  }

  /// Stops and releases the active Apple TTS audio player.
  private func stopTtsAudioPlayback() {
    ttsAudioPlayer?.stop()
    ttsAudioPlayer = nil
    ttsAudioPaused = false
  }

  private func hasSubscriptionId(_ text: String) -> Bool {
    guard let data = text.data(using: .utf8),
      let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
    else {
      return false
    }
    return object["subscriptionId"] is String
  }

  private func jsonString(_ value: Any) -> String {
    do {
      let data = try JSONSerialization.data(withJSONObject: value, options: [.fragmentsAllowed])
      guard let text = String(data: data, encoding: .utf8) else {
        return "{\"error\":\"json text encoding failed\"}"
      }
      return text
    } catch {
      return "{\"error\":\"json serialization failed: \(error.localizedDescription)\"}"
    }
  }

  private func dictionaryFromJson(_ text: String?) throws -> [String: Any] {
    guard let text = text, let data = text.data(using: .utf8) else {
      return [:]
    }
    let value = try JSONSerialization.jsonObject(with: data)
    guard let object = value as? [String: Any] else {
      throw RuntimeChannelError.invalidArgs("ownerBluetooth paramsJson must be an object")
    }
    return object
  }

  private func withUtf8Bytes<T>(_ text: String, _ body: (UnsafePointer<UInt8>?, Int) throws -> T) throws -> T {
    let bytes = Array(text.utf8)
    return try bytes.withUnsafeBufferPointer { buffer in
      try body(buffer.baseAddress, buffer.count)
    }
  }

  private func takeString(_ pointer: UnsafeMutablePointer<CChar>?) -> String {
    guard let pointer = pointer else {
      return ""
    }
    let value = String(cString: pointer)
    operit_flutter_bridge_free_string(pointer)
    return value
  }

  /// Copies and releases one owned Rust Link byte buffer.
  private func takeBytes(_ buffer: OperitByteBuffer) -> Data {
    guard let pointer = buffer.ptr else {
      return Data()
    }
    let data = Data(bytes: pointer, count: Int(buffer.len))
    operit_flutter_bridge_free_bytes(buffer)
    return data
  }
}

private enum AppleCrashChannel {
  private static var channel: FlutterMethodChannel?

  static func register(binaryMessenger: FlutterBinaryMessenger) {
    channel?.setMethodCallHandler(nil)
    let crashChannel = FlutterMethodChannel(name: "operit/crash", binaryMessenger: binaryMessenger)
    crashChannel.setMethodCallHandler { call, result in
      guard call.method == "present" else {
        result(FlutterMethodNotImplemented)
        return
      }
      guard let arguments = call.arguments as? [String: Any],
            let details = arguments["details"] as? String else {
        result(FlutterError(code: "INVALID_ARGS", message: "present requires crash details", details: nil))
        return
      }
      DispatchQueue.main.async {
        guard let windowScene = UIApplication.shared.connectedScenes.compactMap({ $0 as? UIWindowScene }).first,
              let viewController = windowScene.windows.first(where: { $0.isKeyWindow })?.rootViewController else {
          result(FlutterError(code: "CRASH_VIEW_UNAVAILABLE", message: "native crash view is unavailable", details: nil))
          return
        }
        let alert = UIAlertController(title: "Operit2 has stopped", message: details, preferredStyle: .alert)
        alert.addAction(UIAlertAction(title: "Close", style: .destructive))
        viewController.present(alert, animated: true) {
          result(nil)
        }
      }
    }
    channel = crashChannel
  }
}

extension AppleRuntimeChannel: AVAudioPlayerDelegate {
  func audioPlayerDidFinishPlaying(_ player: AVAudioPlayer, successfully flag: Bool) {
    audioPlayers = audioPlayers.filter { $0.value !== player }
    if ttsAudioPlayer === player {
      ttsAudioPlayer = nil
      ttsAudioPaused = false
    }
  }
}

private final class AppleBluetoothController: NSObject, CBCentralManagerDelegate, CBPeripheralDelegate {
  private let callbackQueue = DispatchQueue(label: "operit.runtime.apple.bluetooth", qos: .userInitiated)
  private lazy var central = CBCentralManager(delegate: self, queue: callbackQueue)
  private let lock = NSLock()
  private var discovered: [UUID: CBPeripheral] = [:]
  private var sessions: [String: AppleBleSession] = [:]
  private var pendingConnects: [UUID: AppleBluetoothWaiter<CBPeripheral>] = [:]
  private var pendingDiscoveries: [String: AppleBluetoothWaiter<Void>] = [:]
  private var pendingReads: [String: AppleBluetoothWaiter<Data>] = [:]
  private var pendingWrites: [String: AppleBluetoothWaiter<Void>] = [:]
  private var connectedPeripheralIds: Set<UUID> = []
  private let eventSink: (String, [String: Any]) -> Void

  /// Creates the Apple Bluetooth controller with normalized event delivery.
  init(eventSink: @escaping (String, [String: Any]) -> Void) {
    self.eventSink = eventSink
    super.init()
  }

  func handle(command: String, params: [String: Any]) throws -> Any {
    switch command {
    case "request_permission":
      _ = central
      return "apple_bluetooth_permission_requested"
    case "state":
      return stateData()
    case "request_enable":
      _ = central
      return "apple_bluetooth_enable_controlled_by_system"
    case "bonded_devices":
      return ["devices": []]
    case "scan":
      return try scan(params: params)
    case "classic_connect", "classic_listen", "classic_accept", "classic_send", "classic_read", "classic_send_and_read":
      throw RuntimeChannelError.invalidState("Apple public Bluetooth API does not expose RFCOMM classic sessions")
    case "close":
      return try close(params: params)
    case "ble_connect":
      return try bleConnect(params: params)
    case "ble_discover_services":
      return try bleDiscoverServices(params: params)
    case "ble_read_characteristic":
      return try bleReadCharacteristic(params: params)
    case "ble_write_characteristic":
      return try bleWriteCharacteristic(params: params)
    case "ble_write_and_read_characteristic":
      try bleWriteCharacteristic(params: [
        "sessionId": requireString(params, "sessionId"),
        "serviceUuid": requireString(params, "writeServiceUuid"),
        "characteristicUuid": requireString(params, "writeCharacteristicUuid"),
        "text": params["text"] as Any,
        "dataBase64": params["dataBase64"] as Any,
      ])
      return try bleReadCharacteristic(params: [
        "sessionId": requireString(params, "sessionId"),
        "serviceUuid": requireString(params, "readServiceUuid"),
        "characteristicUuid": requireString(params, "readCharacteristicUuid"),
        "timeoutMs": params["timeoutMs"] as Any,
      ])
    case "ble_subscribe_characteristic":
      return try bleSubscribeCharacteristic(params: params)
    case "ble_read_notifications":
      return try bleReadNotifications(params: params)
    default:
      throw RuntimeChannelError.invalidArgs("unsupported Apple Bluetooth command: \(command)")
    }
  }

  /// Emits the normalized Bluetooth adapter power state.
  func centralManagerDidUpdateState(_ central: CBCentralManager) {
    eventSink(
      "bluetooth.adapter.powered_changed",
      ["powered": central.state == .poweredOn, "connected": !connectedPeripheralIds.isEmpty]
    )
  }

  func centralManager(_ central: CBCentralManager, didDiscover peripheral: CBPeripheral, advertisementData: [String: Any], rssi RSSI: NSNumber) {
    withLock {
      discovered[peripheral.identifier] = peripheral
    }
    eventSink(
      "bluetooth.device.found",
      bluetoothEventData(peripheral: peripheral, connected: peripheral.state == .connected, rssi: RSSI)
    )
  }

  func centralManager(_ central: CBCentralManager, didConnect peripheral: CBPeripheral) {
    withLock { connectedPeripheralIds.insert(peripheral.identifier) }
    eventSink(
      "bluetooth.device.connected",
      bluetoothEventData(peripheral: peripheral, connected: true, rssi: nil)
    )
    emitAdapterConnectionState()
    let waiter = withLock {
      pendingConnects.removeValue(forKey: peripheral.identifier)
    }
    waiter?.succeed(peripheral)
  }

  func centralManager(_ central: CBCentralManager, didFailToConnect peripheral: CBPeripheral, error: Error?) {
    let waiter = withLock {
      pendingConnects.removeValue(forKey: peripheral.identifier)
    }
    waiter?.fail(error?.localizedDescription ?? "Apple BLE connect failed")
  }

  /// Emits normalized device and adapter state after a BLE disconnection.
  func centralManager(_ central: CBCentralManager, didDisconnectPeripheral peripheral: CBPeripheral, error: Error?) {
    withLock { connectedPeripheralIds.remove(peripheral.identifier) }
    eventSink(
      "bluetooth.device.disconnected",
      bluetoothEventData(peripheral: peripheral, connected: false, rssi: nil)
    )
    emitAdapterConnectionState()
  }

  /// Builds the shared Bluetooth device event structure from CoreBluetooth state.
  private func bluetoothEventData(
    peripheral: CBPeripheral,
    connected: Bool,
    rssi: NSNumber?
  ) -> [String: Any] {
    return [
      "deviceAddress": peripheral.identifier.uuidString,
      "deviceName": peripheral.name ?? NSNull(),
      "connected": connected,
      "bonded": NSNull(),
      "rssi": rssi ?? NSNull(),
    ]
  }

  /// Emits whether any CoreBluetooth peripheral remains connected.
  private func emitAdapterConnectionState() {
    let connected = withLock { !connectedPeripheralIds.isEmpty }
    eventSink(
      "bluetooth.adapter.connection_state_changed",
      ["powered": central.state == .poweredOn, "connected": connected]
    )
  }

  func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
    let waiters = withLock {
      sessions.values
        .filter { $0.peripheral === peripheral }
        .compactMap { pendingDiscoveries.removeValue(forKey: $0.sessionId) }
    }
    for waiter in waiters {
      if let error = error {
        waiter.fail(error.localizedDescription)
      } else {
        waiter.succeed(())
      }
    }
  }

  func peripheral(_ peripheral: CBPeripheral, didDiscoverCharacteristicsFor service: CBService, error: Error?) {
    let waiters = withLock {
      sessions.values
        .filter { $0.peripheral === peripheral }
        .compactMap { session in
          let key = discoveryKey(sessionId: session.sessionId, serviceUuid: service.uuid.uuidString.lowercased())
          return pendingDiscoveries.removeValue(forKey: key)
        }
    }
    for waiter in waiters {
      if let error = error {
        waiter.fail(error.localizedDescription)
      } else {
        waiter.succeed(())
      }
    }
  }

  func peripheral(_ peripheral: CBPeripheral, didUpdateValueFor characteristic: CBCharacteristic, error: Error?) {
    guard let serviceUuid = characteristic.service?.uuid.uuidString else {
      return
    }
    let data = characteristic.value
    let waiters = withLock {
      var collected: [AppleBluetoothWaiter<Data>] = []
      for session in sessions.values where session.peripheral === peripheral {
        let key = characteristicKey(sessionId: session.sessionId, serviceUuid: serviceUuid, characteristicUuid: characteristic.uuid.uuidString)
        if let waiter = pendingReads.removeValue(forKey: key) {
          collected.append(waiter)
        } else if let data = data {
          session.notifications.append(notification(characteristicUuid: characteristic.uuid.uuidString.lowercased(), data: data))
        }
      }
      return collected
    }
    for waiter in waiters {
      if let error = error {
        waiter.fail(error.localizedDescription)
      } else if let data = data {
        waiter.succeed(data)
      } else {
        waiter.fail("Apple BLE characteristic value is missing")
      }
    }
  }

  func peripheral(_ peripheral: CBPeripheral, didWriteValueFor characteristic: CBCharacteristic, error: Error?) {
    guard let serviceUuid = characteristic.service?.uuid.uuidString else {
      return
    }
    let waiters = withLock {
      var collected: [AppleBluetoothWaiter<Void>] = []
      for session in sessions.values where session.peripheral === peripheral {
        let key = characteristicKey(sessionId: session.sessionId, serviceUuid: serviceUuid, characteristicUuid: characteristic.uuid.uuidString)
        if let waiter = pendingWrites.removeValue(forKey: key) {
          collected.append(waiter)
        }
      }
      return collected
    }
    for waiter in waiters {
      if let error = error {
        waiter.fail(error.localizedDescription)
      } else {
        waiter.succeed(())
      }
    }
  }

  private func stateData() -> [String: Any] {
    switch central.state {
    case .poweredOn:
      return ["supported": true, "enabled": true, "state": "powered_on"]
    case .poweredOff:
      return ["supported": true, "enabled": false, "state": "powered_off"]
    case .unauthorized:
      return ["supported": true, "enabled": false, "state": "unauthorized"]
    case .unsupported:
      return ["supported": false, "enabled": false, "state": "unsupported"]
    case .resetting:
      return ["supported": true, "enabled": false, "state": "resetting"]
    case .unknown:
      return ["supported": true, "enabled": false, "state": "unknown"]
    @unknown default:
      return ["supported": true, "enabled": false, "state": "unknown"]
    }
  }

  private func scan(params: [String: Any]) throws -> [String: Any] {
    try ensurePoweredOn()
    let durationMs = intValue(params["durationMs"], name: "durationMs")
    withLock {
      discovered.removeAll()
    }
    central.scanForPeripherals(withServices: nil, options: nil)
    Thread.sleep(forTimeInterval: Double(max(durationMs, 0)) / 1000.0)
    central.stopScan()
    let devices = withLock {
      discovered.values.map { peripheral in
        [
          "name": peripheral.name as Any,
          "address": peripheral.identifier.uuidString,
          "type": "ble",
          "bondState": "unknown",
          "source": "apple.core_bluetooth",
          "rssi": NSNull(),
        ] as [String: Any]
      }
    }
    return ["devices": devices, "durationMs": durationMs, "includesBle": true]
  }

  private func bleConnect(params: [String: Any]) throws -> [String: Any] {
    try ensurePoweredOn()
    let address = try requireString(params, "address")
    guard let uuid = UUID(uuidString: address) else {
      throw RuntimeChannelError.invalidArgs("Apple BLE address must be a peripheral UUID")
    }
    guard let peripheral = central.retrievePeripherals(withIdentifiers: [uuid]).first else {
      throw RuntimeChannelError.invalidState("Apple BLE peripheral is not discovered: \(address)")
    }
    let waiter = AppleBluetoothWaiter<CBPeripheral>()
    withLock {
      pendingConnects[uuid] = waiter
    }
    central.connect(peripheral, options: nil)
    let connected = try waiter.wait(seconds: 20)
    connected.delegate = self
    let sessionId = "apple-ble-\(UUID().uuidString)"
    withLock {
      sessions[sessionId] = AppleBleSession(sessionId: sessionId, peripheral: connected)
    }
    return ["sessionId": sessionId, "address": connected.identifier.uuidString, "mode": "ble"]
  }

  private func bleDiscoverServices(params: [String: Any]) throws -> [String: Any] {
    let sessionId = try requireString(params, "sessionId")
    let timeoutMs = intValue(params["timeoutMs"], name: "timeoutMs")
    let session = try requireSession(sessionId)
    let waiter = AppleBluetoothWaiter<Void>()
    withLock {
      pendingDiscoveries[sessionId] = waiter
    }
    session.peripheral.discoverServices(nil)
    try waiter.wait(seconds: seconds(timeoutMs))
    var services: [[String: Any]] = []
    for service in session.peripheral.services ?? [] {
      let serviceUuid = service.uuid.uuidString.lowercased()
      let key = discoveryKey(sessionId: sessionId, serviceUuid: serviceUuid)
      let characteristicWaiter = AppleBluetoothWaiter<Void>()
      withLock {
        pendingDiscoveries[key] = characteristicWaiter
      }
      session.peripheral.discoverCharacteristics(nil, for: service)
      try characteristicWaiter.wait(seconds: seconds(timeoutMs))
      var characteristicItems: [[String: Any]] = []
      withLock {
        for characteristic in service.characteristics ?? [] {
          session.characteristics[characteristicKey(sessionId: sessionId, serviceUuid: serviceUuid, characteristicUuid: characteristic.uuid.uuidString)] = characteristic
          characteristicItems.append([
            "uuid": characteristic.uuid.uuidString.lowercased(),
            "properties": propertyNames(characteristic.properties),
          ])
        }
      }
      services.append(["uuid": serviceUuid, "characteristics": characteristicItems])
    }
    return ["sessionId": sessionId, "services": services]
  }

  private func bleReadCharacteristic(params: [String: Any]) throws -> [String: Any] {
    let sessionId = try requireString(params, "sessionId")
    let serviceUuid = try requireString(params, "serviceUuid")
    let characteristicUuid = try requireString(params, "characteristicUuid")
    let timeoutMs = intValue(params["timeoutMs"], name: "timeoutMs")
    let session = try requireSession(sessionId)
    let characteristic = try requireCharacteristic(session, serviceUuid: serviceUuid, characteristicUuid: characteristicUuid)
    let key = characteristicKey(sessionId: sessionId, serviceUuid: serviceUuid, characteristicUuid: characteristicUuid)
    let waiter = AppleBluetoothWaiter<Data>()
    withLock {
      pendingReads[key] = waiter
    }
    session.peripheral.readValue(for: characteristic)
    let data = try waiter.wait(seconds: seconds(timeoutMs))
    return readData(sessionId: sessionId, data: data)
  }

  private func bleWriteCharacteristic(params: [String: Any]) throws -> [String: Any] {
    let sessionId = try requireString(params, "sessionId")
    let serviceUuid = try requireString(params, "serviceUuid")
    let characteristicUuid = try requireString(params, "characteristicUuid")
    let data = try payloadData(params)
    let session = try requireSession(sessionId)
    let characteristic = try requireCharacteristic(session, serviceUuid: serviceUuid, characteristicUuid: characteristicUuid)
    let key = characteristicKey(sessionId: sessionId, serviceUuid: serviceUuid, characteristicUuid: characteristicUuid)
    let waiter = AppleBluetoothWaiter<Void>()
    withLock {
      pendingWrites[key] = waiter
    }
    session.peripheral.writeValue(data, for: characteristic, type: .withResponse)
    try waiter.wait(seconds: 20)
    return ["sessionId": sessionId, "bytesWritten": data.count]
  }

  private func bleSubscribeCharacteristic(params: [String: Any]) throws -> [String: Any] {
    let sessionId = try requireString(params, "sessionId")
    let serviceUuid = try requireString(params, "serviceUuid")
    let characteristicUuid = try requireString(params, "characteristicUuid")
    let enable = try requireBool(params, "enable")
    let session = try requireSession(sessionId)
    let characteristic = try requireCharacteristic(session, serviceUuid: serviceUuid, characteristicUuid: characteristicUuid)
    session.peripheral.setNotifyValue(enable, for: characteristic)
    return ["sessionId": sessionId, "bytesWritten": 0]
  }

  private func bleReadNotifications(params: [String: Any]) throws -> [String: Any] {
    let sessionId = try requireString(params, "sessionId")
    let limit = intValue(params["limit"], name: "limit")
    return try withLock {
      guard let session = sessions[sessionId] else {
        throw RuntimeChannelError.invalidState("Apple BLE session is not available: \(sessionId)")
      }
      let count = min(max(limit, 0), session.notifications.count)
      let entries = Array(session.notifications.prefix(count))
      session.notifications.removeFirst(count)
      return ["sessionId": sessionId, "notifications": entries]
    }
  }

  private func close(params: [String: Any]) throws -> String {
    let sessionId = try requireString(params, "sessionId")
    let session = withLock {
      sessions.removeValue(forKey: sessionId)
    }
    if let session = session {
      central.cancelPeripheralConnection(session.peripheral)
    }
    return "apple_bluetooth_session_closed:\(sessionId)"
  }

  private func ensurePoweredOn() throws {
    if central.state != .poweredOn {
      throw RuntimeChannelError.invalidState("Apple Bluetooth is not powered on: \(central.state.rawValue)")
    }
  }

  private func requireSession(_ sessionId: String) throws -> AppleBleSession {
    let session = withLock {
      sessions[sessionId]
    }
    guard let session = session else {
      throw RuntimeChannelError.invalidState("Apple BLE session is not available: \(sessionId)")
    }
    return session
  }

  private func requireCharacteristic(_ session: AppleBleSession, serviceUuid: String, characteristicUuid: String) throws -> CBCharacteristic {
    let key = characteristicKey(sessionId: session.sessionId, serviceUuid: serviceUuid, characteristicUuid: characteristicUuid)
    let characteristic = withLock {
      session.characteristics[key]
    }
    guard let characteristic = characteristic else {
      throw RuntimeChannelError.invalidState("Apple BLE characteristic is not discovered: \(serviceUuid)/\(characteristicUuid)")
    }
    return characteristic
  }

  private func propertyNames(_ properties: CBCharacteristicProperties) -> [String] {
    var names: [String] = []
    if (properties.rawValue & CBCharacteristicProperties.read.rawValue) != 0 { names.append("read") }
    if (properties.rawValue & CBCharacteristicProperties.write.rawValue) != 0 { names.append("write") }
    if (properties.rawValue & CBCharacteristicProperties.writeWithoutResponse.rawValue) != 0 { names.append("write_without_response") }
    if (properties.rawValue & CBCharacteristicProperties.notify.rawValue) != 0 { names.append("notify") }
    if (properties.rawValue & CBCharacteristicProperties.indicate.rawValue) != 0 { names.append("indicate") }
    return names
  }

  private func payloadData(_ params: [String: Any]) throws -> Data {
    let text = params["text"] as? String
    let dataBase64 = params["dataBase64"] as? String
    if let text = text, dataBase64 == nil {
      return Data(text.utf8)
    }
    if text == nil, let dataBase64 = dataBase64, let data = Data(base64Encoded: dataBase64) {
      return data
    }
    throw RuntimeChannelError.invalidArgs("Provide exactly one of text or dataBase64")
  }

  private func readData(sessionId: String, data: Data) -> [String: Any] {
    [
      "sessionId": sessionId,
      "bytesRead": data.count,
      "text": String(data: data, encoding: .utf8) as Any,
      "dataBase64": data.base64EncodedString(),
    ]
  }

  private func notification(characteristicUuid: String, data: Data) -> [String: Any] {
    [
      "characteristicUuid": characteristicUuid,
      "bytesRead": data.count,
      "text": String(data: data, encoding: .utf8) as Any,
      "dataBase64": data.base64EncodedString(),
      "timestamp": Int64(Date().timeIntervalSince1970 * 1000),
    ]
  }

  private func seconds(_ timeoutMs: Int) -> TimeInterval {
    TimeInterval(max(timeoutMs, 1)) / 1000.0
  }

  private func characteristicKey(sessionId: String, serviceUuid: String, characteristicUuid: String) -> String {
    "\(sessionId):\(serviceUuid.lowercased()):\(characteristicUuid.lowercased())"
  }

  private func discoveryKey(sessionId: String, serviceUuid: String) -> String {
    "\(sessionId):\(serviceUuid.lowercased())"
  }

  private func withLock<T>(_ body: () throws -> T) rethrows -> T {
    lock.lock()
    defer { lock.unlock() }
    return try body()
  }
}

private final class AppleBleSession {
  let sessionId: String
  let peripheral: CBPeripheral
  var characteristics: [String: CBCharacteristic] = [:]
  var notifications: [[String: Any]] = []

  init(sessionId: String, peripheral: CBPeripheral) {
    self.sessionId = sessionId
    self.peripheral = peripheral
  }
}

private final class AppleBluetoothWaiter<T> {
  private let semaphore = DispatchSemaphore(value: 0)
  private var value: T?
  private var error: String?

  func succeed(_ value: T) {
    self.value = value
    semaphore.signal()
  }

  func fail(_ message: String) {
    error = message
    semaphore.signal()
  }

  func wait(seconds: TimeInterval) throws -> T {
    if semaphore.wait(timeout: .now() + seconds) == .timedOut {
      throw RuntimeChannelError.invalidState("Apple Bluetooth operation timed out")
    }
    if let error = error {
      throw RuntimeChannelError.invalidState(error)
    }
    guard let value = value else {
      throw RuntimeChannelError.invalidState("Apple Bluetooth operation completed without a result")
    }
    return value
  }
}

private func requireString(_ params: [String: Any], _ key: String) throws -> String {
  guard let value = params[key] as? String, !value.isEmpty else {
    throw RuntimeChannelError.invalidArgs("\(key) is required")
  }
  return value
}

private func requireBool(_ params: [String: Any], _ key: String) throws -> Bool {
  guard let value = params[key] as? Bool else {
    throw RuntimeChannelError.invalidArgs("\(key) is required")
  }
  return value
}

private func intValue(_ value: Any?, name: String) -> Int {
  if let number = value as? NSNumber {
    return number.intValue
  }
  if let string = value as? String, let int = Int(string) {
    return int
  }
  return 0
}

private enum RuntimeChannelError: LocalizedError {
  case createFailed(String)
  case invalidArgs(String)
  case invalidState(String)

  var errorDescription: String? {
    switch self {
    case .createFailed(let message):
      return message
    case .invalidArgs(let message):
      return message
    case .invalidState(let message):
      return message
    }
  }
}
