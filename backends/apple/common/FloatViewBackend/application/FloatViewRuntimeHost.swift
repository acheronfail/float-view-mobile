#if canImport(FloatViewCoreBridge)
  import FloatViewCoreBridge
#endif
import Foundation

public enum FloatViewDocumentError: Error, Equatable {
  case invalidPath
  case cancelled
  case unavailable
  case payloadTooLarge
  case ioFailure
}

public protocol FloatViewRuntimeHostProtocol: AnyObject, Sendable {
  func setBackgroundLogging(_ enabled: Bool) -> Bool
  func presentNotificationsPermission() -> Bool
  func presentImport(
    stagingPath: String,
    allowedContentTypes: [String],
    maximumBytes: UInt64,
    completion: @escaping @Sendable (Result<Void, FloatViewDocumentError>) -> Void
  )
  func presentExport(
    stagingPath: String,
    displayName: String,
    contentType: String,
    completion: @escaping @Sendable (Result<Void, FloatViewDocumentError>) -> Void
  )
}

public extension FloatViewRuntimeHostProtocol {
  func presentImport(
    stagingPath _: String,
    allowedContentTypes _: [String],
    maximumBytes _: UInt64,
    completion: @escaping @Sendable (Result<Void, FloatViewDocumentError>) -> Void
  ) {
    completion(.failure(.unavailable))
  }

  func presentExport(
    stagingPath _: String,
    displayName _: String,
    contentType _: String,
    completion: @escaping @Sendable (Result<Void, FloatViewDocumentError>) -> Void
  ) {
    completion(.failure(.unavailable))
  }
}

/// Retains only Apple-owned storage selection, permission presentation, and
/// security-scoped document copying. Portable storage and resource semantics
/// live in Rust.
public final class FloatViewRuntimeHost: FloatViewRuntimeHostProtocol, @unchecked Sendable {
  public typealias DocumentPresenter = @Sendable (
    _ destination: URL,
    _ allowedContentTypes: [String],
    _ maximumBytes: UInt64,
    _ completion: @escaping @Sendable (Result<Void, FloatViewDocumentError>) -> Void
  ) -> Void
  public typealias ExportPresenter = @Sendable (
    _ stagedURL: URL,
    _ completion: @escaping @Sendable (Result<Void, FloatViewDocumentError>) -> Void
  ) -> Void

  private let fileManager: FileManager
  private let durableRoot: URL
  private let backgroundLogging: @Sendable (Bool) -> Bool
  private let notificationPresenter: @Sendable () -> Bool
  private let documentPresenter: DocumentPresenter?
  private let exportPresenter: ExportPresenter?

  public init(
    durableRoot: URL,
    fileManager: FileManager = .default,
    backgroundLogging: @escaping @Sendable (Bool) -> Bool = { _ in false },
    notificationPresenter: @escaping @Sendable () -> Bool = { false },
    documentPresenter: DocumentPresenter? = nil,
    exportPresenter: ExportPresenter? = nil
  ) {
    self.durableRoot = durableRoot.standardizedFileURL
    self.fileManager = fileManager
    self.backgroundLogging = backgroundLogging
    self.notificationPresenter = notificationPresenter
    self.documentPresenter = documentPresenter
    self.exportPresenter = exportPresenter
  }

  public static func standard(
    durableRoot configuredRoot: URL? = nil,
    fileManager: FileManager = .default,
    backgroundLogging: @escaping @Sendable (Bool) -> Bool = { _ in false },
    presentation: AppleSystemPresentation? = nil
  ) throws -> FloatViewRuntimeHost {
    let durableRoot: URL
    if let configuredRoot {
      durableRoot = configuredRoot
    } else {
      guard let root = fileManager.urls(
        for: .applicationSupportDirectory,
        in: .userDomainMask
      ).first else { throw FloatViewDocumentError.unavailable }
      durableRoot = root.appending(path: "FloatView", directoryHint: .isDirectory)
    }
    let notificationPresenter: @Sendable () -> Bool
    if let presentation {
      notificationPresenter = presentation.notificationPresenter
    } else {
      notificationPresenter = { false }
    }
    return FloatViewRuntimeHost(
      durableRoot: durableRoot,
      fileManager: fileManager,
      backgroundLogging: backgroundLogging,
      notificationPresenter: notificationPresenter,
      documentPresenter: presentation?.documentPresenter,
      exportPresenter: presentation?.exportPresenter
    )
  }

  public func durableStorageRoot() throws -> URL {
    do {
      try fileManager.createDirectory(at: durableRoot, withIntermediateDirectories: true)
      return durableRoot
    } catch {
      throw FloatViewDocumentError.ioFailure
    }
  }

  public func setBackgroundLogging(_ enabled: Bool) -> Bool {
    backgroundLogging(enabled)
  }

  public func presentNotificationsPermission() -> Bool {
    notificationPresenter()
  }

  public func presentImport(
    stagingPath: String,
    allowedContentTypes: [String],
    maximumBytes: UInt64,
    completion: @escaping @Sendable (Result<Void, FloatViewDocumentError>) -> Void
  ) {
    guard let documentPresenter,
          let destination = approvedDocumentURL(stagingPath, mustExist: false)
    else {
      completion(.failure(.invalidPath))
      return
    }
    documentPresenter(destination, allowedContentTypes, maximumBytes, completion)
  }

  public func presentExport(
    stagingPath: String,
    displayName: String,
    contentType _: String,
    completion: @escaping @Sendable (Result<Void, FloatViewDocumentError>) -> Void
  ) {
    guard !displayName.isEmpty, !displayName.contains("/"), !displayName.contains("\\"),
          let exportPresenter,
          let stagedURL = approvedDocumentURL(stagingPath, mustExist: true)
    else {
      completion(.failure(.invalidPath))
      return
    }
    exportPresenter(stagedURL, completion)
  }

  private func approvedDocumentURL(_ path: String, mustExist: Bool) -> URL? {
    guard path.utf8.count <= 4096 else { return nil }
    do {
      try fileManager.createDirectory(at: durableRoot, withIntermediateDirectories: true)
      let root = durableRoot.resolvingSymlinksInPath().standardizedFileURL
      let candidate = URL(fileURLWithPath: path).standardizedFileURL
      let parent = candidate.deletingLastPathComponent().resolvingSymlinksInPath()
      guard parent.path == root.path || parent.path.hasPrefix(root.path + "/"),
            !mustExist || fileManager.fileExists(atPath: candidate.path)
      else { return nil }
      return candidate
    } catch {
      return nil
    }
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
      throw FloatViewDocumentError.payloadTooLarge
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
      else { throw FloatViewDocumentError.ioFailure }
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
        guard read >= 0 else { throw FloatViewDocumentError.ioFailure }
        if read == 0 { break }
        total += UInt64(read)
        guard total <= maximumBytes else { throw FloatViewDocumentError.payloadTooLarge }
        var written = 0
        while written < read {
          let count = buffer.withUnsafeBufferPointer { pointer in
            output.write(pointer.baseAddress! + written, maxLength: read - written)
          }
          guard count > 0 else { throw FloatViewDocumentError.ioFailure }
          written += count
        }
      }
      guard total > 0 else { throw FloatViewDocumentError.ioFailure }
    } catch let error as FloatViewDocumentError {
      try? fileManager.removeItem(at: destination)
      throw error
    } catch {
      try? fileManager.removeItem(at: destination)
      throw FloatViewDocumentError.ioFailure
    }
  }
}
