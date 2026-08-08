#if canImport(FloatViewCoreBridge)
  import FloatViewCoreBridge
#endif
import Foundation

public enum ApplePlatformHostError: Error, Equatable {
  case invalidPath
  case unavailable
  case payloadTooLarge
  case ioFailure
}

public protocol ApplePlatformHost: AnyObject, Sendable {
  func setBackgroundLogging(_ enabled: Bool) -> Bool
  func presentNotificationsPermission() -> Bool
}

/// Retains only Apple-owned storage selection, permission presentation, and
/// security-scoped document copying. Portable storage and resource semantics
/// live in Rust.
public final class ApplePlatformHostServices: ApplePlatformHost, @unchecked Sendable {
  public typealias DocumentPresenter = @Sendable (
    _ destination: URL,
    _ maximumBytes: UInt64,
    _ completion: @escaping @Sendable (Result<Void, ApplePlatformHostError>) -> Void
  ) -> Void
  public typealias ExportPresenter = @Sendable (
    _ stagedURL: URL,
    _ completion: @escaping @Sendable (Result<Void, ApplePlatformHostError>) -> Void
  ) -> Void

  private let fileManager: FileManager
  private let durableRoot: URL
  private let backgroundLogging: @Sendable (Bool) -> Bool
  private let notificationPresenter: @Sendable () -> Bool

  public init(
    durableRoot: URL,
    fileManager: FileManager = .default,
    backgroundLogging: @escaping @Sendable (Bool) -> Bool = { _ in false },
    notificationPresenter: @escaping @Sendable () -> Bool = { false }
  ) {
    self.durableRoot = durableRoot.standardizedFileURL
    self.fileManager = fileManager
    self.backgroundLogging = backgroundLogging
    self.notificationPresenter = notificationPresenter
  }

  public static func standard(
    fileManager: FileManager = .default,
    presentation: AppleSystemPresentation? = nil
  ) throws -> ApplePlatformHostServices {
    guard let root = fileManager.urls(
      for: .applicationSupportDirectory,
      in: .userDomainMask
    ).first else { throw ApplePlatformHostError.unavailable }
    let notificationPresenter: @Sendable () -> Bool
    if let presentation {
      notificationPresenter = presentation.notificationPresenter
    } else {
      notificationPresenter = { false }
    }
    return ApplePlatformHostServices(
      durableRoot: root.appending(path: "FloatView", directoryHint: .isDirectory),
      fileManager: fileManager,
      notificationPresenter: notificationPresenter
    )
  }

  public func durableStorageRoot() throws -> URL {
    do {
      try fileManager.createDirectory(at: durableRoot, withIntermediateDirectories: true)
      return durableRoot
    } catch {
      throw ApplePlatformHostError.ioFailure
    }
  }

  public func setBackgroundLogging(_ enabled: Bool) -> Bool {
    backgroundLogging(enabled)
  }

  public func presentNotificationsPermission() -> Bool {
    notificationPresenter()
  }

  /// Copies security-scoped input in bounded chunks and deletes partial output.
  public static func copySecurityScopedDocument(
    source: URL,
    destination: URL,
    maximumBytes: UInt64,
    fileManager: FileManager = .default
  ) throws {
    let access = source.startAccessingSecurityScopedResource()
    defer { if access { source.stopAccessingSecurityScopedResource() } }
    guard maximumBytes > 0, maximumBytes <= 512 * 1024 * 1024 else {
      throw ApplePlatformHostError.payloadTooLarge
    }
    do {
      try fileManager.createDirectory(
        at: destination.deletingLastPathComponent(),
        withIntermediateDirectories: true
      )
      if fileManager.fileExists(atPath: destination.path) {
        try fileManager.removeItem(at: destination)
      }
      guard fileManager.createFile(atPath: destination.path, contents: nil),
            let input = InputStream(url: source),
            let output = OutputStream(url: destination, append: false)
      else { throw ApplePlatformHostError.ioFailure }
      input.open()
      output.open()
      defer {
        input.close()
        output.close()
      }
      var total: UInt64 = 0
      var buffer = [UInt8](repeating: 0, count: 64 * 1024)
      while input.hasBytesAvailable {
        let read = input.read(&buffer, maxLength: buffer.count)
        guard read >= 0 else { throw ApplePlatformHostError.ioFailure }
        if read == 0 { break }
        total += UInt64(read)
        guard total <= maximumBytes else { throw ApplePlatformHostError.payloadTooLarge }
        var written = 0
        while written < read {
          let count = buffer.withUnsafeBufferPointer { pointer in
            output.write(pointer.baseAddress! + written, maxLength: read - written)
          }
          guard count > 0 else { throw ApplePlatformHostError.ioFailure }
          written += count
        }
      }
    } catch let error as ApplePlatformHostError {
      try? fileManager.removeItem(at: destination)
      throw error
    } catch {
      try? fileManager.removeItem(at: destination)
      throw ApplePlatformHostError.ioFailure
    }
  }
}
