import ImageIO
import UniformTypeIdentifiers
import UIKit
import XCTest

@testable import Runner

@MainActor
final class AppleTextRecognizerTests: XCTestCase {
  private var directory: URL!
  private let ampleMemory = 2 * 1024 * 1024 * 1024

  override func setUpWithError() throws {
    directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
  }

  override func tearDownWithError() throws {
    try FileManager.default.removeItem(at: directory)
  }

  private func grayscaleImage(width: Int, height: Int) throws -> CGImage {
    let context = try XCTUnwrap(CGContext(data: nil, width: width, height: height,
      bitsPerComponent: 8, bytesPerRow: width, space: CGColorSpaceCreateDeviceGray(),
      bitmapInfo: CGImageAlphaInfo.none.rawValue))
    context.setFillColor(gray: 1, alpha: 1)
    context.fill(CGRect(x: 0, y: 0, width: width, height: height))
    return try XCTUnwrap(context.makeImage())
  }

  private func save(_ image: CGImage, name: String = "image.jpg", orientation: Int = 1) throws -> URL {
    let url = directory.appendingPathComponent(name)
    let destination = try XCTUnwrap(CGImageDestinationCreateWithURL(
      url as CFURL, UTType.jpeg.identifier as CFString, 1, nil))
    CGImageDestinationAddImage(destination, image, [
      kCGImagePropertyOrientation: orientation,
      kCGImageDestinationLossyCompressionQuality: 0.95,
    ] as CFDictionary)
    XCTAssertTrue(CGImageDestinationFinalize(destination))
    return url
  }

  func testCameraSizedImageIsDecodedWithinDimensionAndPixelLimits() throws {
    let url = try autoreleasepool {
      try save(grayscaleImage(width: 8000, height: 6000))
    }
    let recognizer = AppleTextRecognizer(availableMemory: { self.ampleMemory })
    let image = try recognizer.prepareImage(imagePath: url.path)
    XCTAssertLessThanOrEqual(max(image.width, image.height), 2048)
    XCTAssertLessThanOrEqual(image.width * image.height, 3_000_000)
    XCTAssertEqual(Double(image.width) / Double(image.height), 4.0 / 3.0, accuracy: 0.005)
  }

  func testSmallImagesAreNotUpscaledAndExifOrientationIsApplied() throws {
    let normal = try save(grayscaleImage(width: 160, height: 80), name: "normal.jpg")
    let rotated = try save(grayscaleImage(width: 160, height: 80), name: "rotated.jpg", orientation: 6)
    let recognizer = AppleTextRecognizer(availableMemory: { self.ampleMemory })
    let image = try recognizer.prepareImage(imagePath: normal.path)
    XCTAssertEqual(image.width, 160)
    XCTAssertEqual(image.height, 80)
    let oriented = try recognizer.prepareImage(imagePath: rotated.path)
    XCTAssertEqual(oriented.width, 80)
    XCTAssertEqual(oriented.height, 160)
  }

  func testLowMemoryReturnsAnErrorBeforeStartingVision() throws {
    let url = try save(grayscaleImage(width: 640, height: 480))
    let recognizer = AppleTextRecognizer(availableMemory: { 140 * 1024 * 1024 })
    XCTAssertThrowsError(try recognizer.recognizeText(imagePath: url.path)) { error in
      guard case AppleTextRecognizer.RecognitionError.insufficientMemory = error else {
        return XCTFail("Expected memory admission failure, got \(error)")
      }
    }
  }

  func testZeroMemoryHeadroomIsRejectedRatherThanBypassingTheLimit() throws {
    let url = try save(grayscaleImage(width: 640, height: 480))
    let recognizer = AppleTextRecognizer(availableMemory: { 0 })
    XCTAssertThrowsError(try recognizer.recognizeText(imagePath: url.path)) { error in
      guard case AppleTextRecognizer.RecognitionError.insufficientMemory = error else {
        return XCTFail("Expected memory admission failure, got \(error)")
      }
    }
  }

  func testAllFormatsReserveMemoryForSourceDecode() throws {
    let image = try grayscaleImage(width: 4000, height: 4000)
    let url = directory.appendingPathComponent("large.png")
    let destination = try XCTUnwrap(CGImageDestinationCreateWithURL(
      url as CFURL, UTType.png.identifier as CFString, 1, nil))
    CGImageDestinationAddImage(destination, image, nil)
    XCTAssertTrue(CGImageDestinationFinalize(destination))
    // Enough for a small Vision input, but not a full source-sized decode.
    let jpeg = try save(image, name: "large.jpg")
    let constrained = AppleTextRecognizer(availableMemory: { 192 * 1024 * 1024 })
    for source in [url, jpeg] {
      XCTAssertThrowsError(try constrained.prepareImage(imagePath: source.path)) { error in
        guard case AppleTextRecognizer.RecognitionError.insufficientMemory = error else {
          return XCTFail("Expected source decode admission failure, got \(error)")
        }
      }
    }
    let admitted = try AppleTextRecognizer(availableMemory: { self.ampleMemory })
      .prepareImage(imagePath: url.path)
    XCTAssertLessThanOrEqual(admitted.width * admitted.height, 3_000_000)
  }

  func testPixelBudgetShrinksWithAvailableMemory() throws {
    let url = try save(grayscaleImage(width: 3000, height: 2000))
    let recognizer = AppleTextRecognizer(availableMemory: { 192 * 1024 * 1024 })
    let image = try recognizer.prepareImage(imagePath: url.path)
    XCTAssertLessThanOrEqual(image.width * image.height, 524_288)
    XCTAssertEqual(Double(image.width) / Double(image.height), 1.5, accuracy: 0.01)
  }

  func testMemoryIsCheckedAgainAfterDecoding() throws {
    let url = try save(grayscaleImage(width: 640, height: 480))
    var checks = 0
    let recognizer = AppleTextRecognizer(availableMemory: {
      checks += 1
      return checks == 1 ? self.ampleMemory : 128 * 1024 * 1024
    })
    XCTAssertThrowsError(try recognizer.recognizeText(imagePath: url.path)) { error in
      guard case AppleTextRecognizer.RecognitionError.insufficientMemory = error else {
        return XCTFail("Expected memory admission failure, got \(error)")
      }
    }
    XCTAssertEqual(checks, 2)
  }

  func testMissingCorruptEmptyAndOversizedFilesReturnErrors() throws {
    let recognizer = AppleTextRecognizer(availableMemory: { self.ampleMemory })
    XCTAssertThrowsError(try recognizer.prepareImage(imagePath: directory.appendingPathComponent("missing").path))
    let invalid = directory.appendingPathComponent("invalid.jpg")
    try Data("not an image".utf8).write(to: invalid)
    XCTAssertThrowsError(try recognizer.prepareImage(imagePath: invalid.path))
    let empty = directory.appendingPathComponent("empty.jpg")
    try Data().write(to: empty)
    XCTAssertThrowsError(try recognizer.prepareImage(imagePath: empty.path))
    let oversized = directory.appendingPathComponent("oversized.jpg")
    XCTAssertTrue(FileManager.default.createFile(atPath: oversized.path, contents: nil))
    let handle = try FileHandle(forWritingTo: oversized)
    try handle.truncate(atOffset: 65 * 1024 * 1024)
    try handle.close()
    XCTAssertThrowsError(try recognizer.prepareImage(imagePath: oversized.path))
    XCTAssertThrowsError(try recognizer.prepareImage(imagePath: directory.path))
  }

  func testExtremeSourceDimensionsAreRejectedWithoutFullDecode() throws {
    // A narrow image has a small encoded/decoded footprint but exceeds the
    // admitted source dimension, exercising the metadata gate itself.
    let image = try grayscaleImage(width: 1, height: 65_536)
    let url = directory.appendingPathComponent("too-tall.png")
    let destination = try XCTUnwrap(CGImageDestinationCreateWithURL(
      url as CFURL, UTType.png.identifier as CFString, 1, nil))
    CGImageDestinationAddImage(destination, image, nil)
    XCTAssertTrue(CGImageDestinationFinalize(destination))
    XCTAssertThrowsError(try AppleTextRecognizer(availableMemory: { self.ampleMemory })
      .prepareImage(imagePath: url.path))
  }

  func testConcurrentRequestsSerializeAndReleaseTheLockAfterFailure() throws {
    let url = try save(grayscaleImage(width: 640, height: 480))
    let finished = expectation(description: "All concurrent OCR requests complete")
    finished.expectedFulfillmentCount = 6
    let events = RecognitionMemoryEvents()
    for requestID in 0..<6 {
      DispatchQueue.global(qos: .userInitiated).async {
        var checks = 0
        let recognizer = AppleTextRecognizer(availableMemory: {
          events.append(requestID)
          checks += 1
          // Encourage interleaving if the production lock is removed.
          Thread.sleep(forTimeInterval: 0.01)
          return checks == 1 ? 2 * 1024 * 1024 * 1024 : 128 * 1024 * 1024
        })
        do {
          _ = try recognizer.recognizeText(imagePath: url.path)
          XCTFail("Expected memory admission failure")
        } catch AppleTextRecognizer.RecognitionError.insufficientMemory {
          // Expected failure still releases the serialization lock.
        } catch {
          XCTFail("Expected memory admission failure, got \(error)")
        }
        finished.fulfill()
      }
    }
    wait(for: [finished], timeout: 10)
    let order = events.snapshot()
    XCTAssertEqual(order.count, 12)
    for index in stride(from: 0, to: order.count - 1, by: 2) {
      XCTAssertEqual(order[index], order[index + 1], "OCR memory-intensive sections overlapped: \(order)")
    }
  }

  func testRealVisionRecognizesTextAndSubsequentRequestStillWorks() throws {
    let format = UIGraphicsImageRendererFormat()
    format.scale = 1
    format.opaque = true
    let image = UIGraphicsImageRenderer(size: CGSize(width: 4000, height: 2000), format: format).image { context in
      UIColor.white.setFill()
      context.fill(CGRect(x: 0, y: 0, width: 4000, height: 2000))
      ("OPERIT OCR 12345" as NSString).draw(at: CGPoint(x: 100, y: 500), withAttributes: [
        .font: UIFont.systemFont(ofSize: 200), .foregroundColor: UIColor.black,
      ])
    }
    let url = try save(XCTUnwrap(image.cgImage), name: "text.jpg")
    let recognizer = AppleTextRecognizer(availableMemory: { self.ampleMemory })
    for _ in 0..<2 {
      let text = try recognizer.recognizeText(imagePath: url.path)
      XCTAssertTrue(text.contains("OPERIT"), text)
      XCTAssertTrue(text.contains("12345"), text)
    }
  }
}

private final class RecognitionMemoryEvents: @unchecked Sendable {
  private let lock = NSLock()
  private var events: [Int] = []

  func append(_ value: Int) {
    lock.lock()
    defer { lock.unlock() }
    events.append(value)
  }

  func snapshot() -> [Int] {
    lock.lock()
    defer { lock.unlock() }
    return events
  }
}
