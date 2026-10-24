import Foundation
import ImageIO
import Vision
import os

/// Bounds image decoding and Vision work before entering Apple's OCR pipeline.
final class AppleTextRecognizer {
  // Vision may allocate many intermediate buffers per input pixel. These are
  // admission limits, not a guarantee about private framework allocations.
  static let maximumDimension = 2048
  static let maximumPixels = 3_000_000
  private static let maximumSourcePixels = 64_000_000.0
  private static let maximumFileBytes = 64 * 1024 * 1024
  private static let memoryReserve = 128 * 1024 * 1024
  // Conservative headroom estimate for Vision's image maps and text boxes.
  // Recheck after decoding because other app work may consume memory meanwhile.
  private static let workingBytesPerPixel = 128
  private static let minimumPixelBudget = 512 * 512
  private static let recognitionLock = NSLock()

  private let availableMemory: () -> Int

  init(availableMemory: @escaping () -> Int = { os_proc_available_memory() }) {
    self.availableMemory = availableMemory
  }

  func recognizeText(imagePath: String) throws -> String {
    // Shared across channel instances. Each request's autoreleased ImageIO and
    // Vision objects are released before the next recognition can start.
    Self.recognitionLock.lock()
    defer { Self.recognitionLock.unlock() }
    return try autoreleasepool {
      let image = try prepareImage(imagePath: imagePath)
      try checkMemory(forPixels: image.width * image.height)
      let request = VNRecognizeTextRequest()
      request.recognitionLevel = .accurate
      // prepareImage applies EXIF rotation/mirroring while decoding.
      let handler = VNImageRequestHandler(cgImage: image, orientation: .up, options: [:])
      try handler.perform([request])
      return (request.results ?? []).compactMap { $0.topCandidates(1).first?.string }
        .joined(separator: "\n")
    }
  }

  /// Reads metadata without decoding the original, then decodes a bounded image.
  func prepareImage(imagePath: String) throws -> CGImage {
    let url = URL(fileURLWithPath: imagePath)
    let values = try url.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
    guard values.isRegularFile == true, let bytes = values.fileSize,
      bytes > 0, bytes <= Self.maximumFileBytes else {
      throw RecognitionError.invalidImage("OCR requires a nonempty image file of at most 64 MiB")
    }
    guard let source = CGImageSourceCreateWithURL(url as CFURL,
      [kCGImageSourceShouldCache: false] as CFDictionary),
      CGImageSourceGetCount(source) > 0,
      let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
      let width = (properties[kCGImagePropertyPixelWidth] as? NSNumber)?.doubleValue,
      let height = (properties[kCGImagePropertyPixelHeight] as? NSNumber)?.doubleValue,
      width.isFinite, height.isFinite, width >= 1, height >= 1,
      width <= 65_535, height <= 65_535,
      width * height <= Self.maximumSourcePixels else {
      throw RecognitionError.invalidImage(
        "OCR image is invalid or exceeds 64 megapixels or 65535 pixels on either side")
    }
    let available = availableMemory()
    // A thumbnail does not guarantee that a codec avoids a full-size working
    // buffer (for example, progressive images). Admit source decoding as well
    // as the much smaller Vision input, regardless of the file format.
    let sourceWorkingBytes = Int(width * height) * 8
    guard available >= Self.memoryReserve + sourceWorkingBytes else {
      throw RecognitionError.insufficientMemory
    }
    let budget = min(Self.maximumPixels,
      max(0, available - Self.memoryReserve) / Self.workingBytesPerPixel)
    guard budget >= Self.minimumPixelBudget else {
      throw RecognitionError.insufficientMemory
    }
    let scale = min(1, Double(Self.maximumDimension) / max(width, height),
      sqrt(Double(budget) / (width * height)))
    // Round down so the decoded image stays inside both limits. Never upscale.
    let edge = max(1, Int(floor(max(width, height) * scale)))
    let options: [CFString: Any] = [
      kCGImageSourceCreateThumbnailFromImageAlways: true,
      kCGImageSourceCreateThumbnailWithTransform: true,
      kCGImageSourceThumbnailMaxPixelSize: edge,
      kCGImageSourceShouldCache: false,
      kCGImageSourceShouldCacheImmediately: true,
    ]
    guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary),
      image.width <= Self.maximumDimension, image.height <= Self.maximumDimension,
      image.width <= budget / max(1, image.height) else {
      throw RecognitionError.invalidImage("OCR image could not be decoded within its pixel budget")
    }
    if scale < 1 {
      NSLog("OperitOCR: bounded image %.0fx%.0f -> %dx%d", width, height, image.width, image.height)
    }
    return image
  }

  private func checkMemory(forPixels pixels: Int) throws {
    guard availableMemory() >= Self.memoryReserve + pixels * Self.workingBytesPerPixel else {
      throw RecognitionError.insufficientMemory
    }
  }

  enum RecognitionError: LocalizedError {
    case invalidImage(String)
    case insufficientMemory

    var errorDescription: String? {
      switch self {
      case .invalidImage(let message): return message
      case .insufficientMemory:
        return "Not enough memory for OCR. Close other tasks and retry with a smaller image."
      }
    }
  }
}
