import Foundation

private enum AppleLocalInferenceRunnerError: LocalizedError {
  case invalidPayload(String)
  case nativeError(String)

  /// Returns the user-visible Apple local inference error.
  var errorDescription: String? {
    switch self {
    case .invalidPayload(let message):
      return message
    case .nativeError(let message):
      return message
    }
  }
}

@_silgen_name("operit_apple_local_inference_run")
private func operitAppleLocalInferenceRun(
  _ method: UnsafePointer<CChar>,
  _ requestJson: UnsafePointer<CChar>
) -> UnsafeMutablePointer<CChar>?

@_silgen_name("operit_apple_local_inference_free")
private func operitAppleLocalInferenceFree(_ value: UnsafeMutablePointer<CChar>?)

final class AppleLocalInferenceRunner {
  static let shared = AppleLocalInferenceRunner()

  /// Keeps runner construction private because it owns a process-level native bridge.
  private init() {}

  /// Runs one owner-host local inference command through the native Sherpa bridge.
  func run(payload: [String: Any]) throws -> [String: Any] {
    guard let method = payload["method"] as? String, !method.isEmpty else {
      throw AppleLocalInferenceRunnerError.invalidPayload("ownerLocalInference expects method")
    }
    guard let requestJson = payload["requestJson"] as? String, !requestJson.isEmpty else {
      throw AppleLocalInferenceRunnerError.invalidPayload("ownerLocalInference expects requestJson")
    }
    let envelopeJson = try callNative(method: method, requestJson: requestJson)
    guard
      let data = envelopeJson.data(using: .utf8),
      let object = try JSONSerialization.jsonObject(with: data) as? [String: Any]
    else {
      throw AppleLocalInferenceRunnerError.nativeError("Apple local inference returned invalid JSON")
    }
    if let error = object["error"] as? String, !error.isEmpty {
      throw AppleLocalInferenceRunnerError.nativeError(error)
    }
    guard let resultJson = object["resultJson"] as? String, !resultJson.isEmpty else {
      throw AppleLocalInferenceRunnerError.nativeError("Apple local inference resultJson is missing")
    }
    return ["resultJson": resultJson]
  }

  /// Calls the native C bridge and owns the returned JSON buffer.
  private func callNative(method: String, requestJson: String) throws -> String {
    guard
      let pointer = method.withCString({ methodPointer in
        requestJson.withCString { requestPointer in
          operitAppleLocalInferenceRun(methodPointer, requestPointer)
        }
      })
    else {
      throw AppleLocalInferenceRunnerError.nativeError("Apple local inference returned no native response")
    }
    defer {
      operitAppleLocalInferenceFree(pointer)
    }
    return String(cString: pointer)
  }
}
