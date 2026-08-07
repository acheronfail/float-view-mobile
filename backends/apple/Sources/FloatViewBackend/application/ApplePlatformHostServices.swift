#if canImport(FloatViewCoreBridge)
  import FloatViewCoreBridge
#endif
import Foundation
import Security

public enum ApplePlatformHostError: Error, Equatable {
  case invalidPath
  case unavailable
  case payloadTooLarge
  case ioFailure
}

public protocol ApplePlatformHost: AnyObject, Sendable {
  func setBackgroundLogging(_ enabled: Bool) -> Bool
  func readResource(
    namespace: FloatViewResourceNamespace,
    logicalName: String,
    maximumBytes: UInt64
  ) throws -> Data
  func stageDocument(
    destination: URL,
    maximumBytes: UInt64,
    completion: @escaping @Sendable (Result<Void, ApplePlatformHostError>) -> Void
  )
  func storageRoot(_ kind: FloatViewStorageRootKind) throws -> URL
  func availableSpace(at root: URL) throws -> UInt64
  func monotonicMilliseconds() -> UInt64
  func wallMilliseconds() -> Int64
  func secureRandom(count: UInt16) throws -> Data
  func diagnostic(stage: String, code: String)
  func presentNotificationsPermission() -> Bool
  func presentTheme(_ theme: FloatViewThemePreference) -> Bool
  func presentServiceState(active: Bool) -> Bool
  func presentExport(
    stagedURL: URL,
    completion: @escaping @Sendable (Result<Void, ApplePlatformHostError>) -> Void
  )
  func cancelPendingPresentation()
}

/// Foundation-owned implementation. Platform app targets inject only the
/// actual UIKit/AppKit presentation closures; resource and file semantics stay
/// bounded and shared by the two Apple hosts.
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
  private let resourceRoots: [FloatViewResourceNamespace: URL]
  private let storageRoots: [FloatViewStorageRootKind: URL]
  private let backgroundLogging: @Sendable (Bool) -> Bool
  private let documentPresenter: DocumentPresenter?
  private let notificationPresenter: @Sendable () -> Bool
  private let themePresenter: @Sendable (FloatViewThemePreference) -> Bool
  private let servicePresenter: @Sendable (Bool) -> Bool
  private let exportPresenter: ExportPresenter?
  private let presentationCanceller: @Sendable () -> Void
  private let diagnosticHandler: @Sendable (String, String) -> Void

  public init(
    resourceRoots: [FloatViewResourceNamespace: URL],
    storageRoots: [FloatViewStorageRootKind: URL],
    fileManager: FileManager = .default,
    backgroundLogging: @escaping @Sendable (Bool) -> Bool = { _ in false },
    documentPresenter: DocumentPresenter? = nil,
    notificationPresenter: @escaping @Sendable () -> Bool = { false },
    themePresenter: @escaping @Sendable (FloatViewThemePreference) -> Bool = { _ in false },
    servicePresenter: @escaping @Sendable (Bool) -> Bool = { _ in false },
    exportPresenter: ExportPresenter? = nil,
    presentationCanceller: @escaping @Sendable () -> Void = {},
    diagnosticHandler: @escaping @Sendable (String, String) -> Void = { _, _ in }
  ) {
    self.resourceRoots = resourceRoots
    self.storageRoots = storageRoots
    self.fileManager = fileManager
    self.backgroundLogging = backgroundLogging
    self.documentPresenter = documentPresenter
    self.notificationPresenter = notificationPresenter
    self.themePresenter = themePresenter
    self.servicePresenter = servicePresenter
    self.exportPresenter = exportPresenter
    self.presentationCanceller = presentationCanceller
    self.diagnosticHandler = diagnosticHandler
  }

  public static func standard(
    resourceRoots: [FloatViewResourceNamespace: URL],
    fileManager: FileManager = .default,
    presentation: AppleSystemPresentation? = nil
  ) throws -> ApplePlatformHostServices {
    guard let durable = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first,
          let cache = fileManager.urls(for: .cachesDirectory, in: .userDomainMask).first
    else { throw ApplePlatformHostError.unavailable }
    let roots: [FloatViewStorageRootKind: URL] = [
      .durable: durable.appending(path: "FloatView", directoryHint: .isDirectory),
      .cache: cache.appending(path: "FloatView", directoryHint: .isDirectory),
      .temporary: fileManager.temporaryDirectory.appending(
        path: "FloatView", directoryHint: .isDirectory
      ),
    ]
    guard let presentation else {
      return ApplePlatformHostServices(
        resourceRoots: resourceRoots,
        storageRoots: roots,
        fileManager: fileManager
      )
    }
    return ApplePlatformHostServices(
      resourceRoots: resourceRoots,
      storageRoots: roots,
      fileManager: fileManager,
      documentPresenter: presentation.documentPresenter,
      notificationPresenter: presentation.notificationPresenter,
      themePresenter: presentation.themePresenter,
      exportPresenter: presentation.exportPresenter,
      presentationCanceller: { presentation.cancel() }
    )
  }

  public func setBackgroundLogging(_ enabled: Bool) -> Bool {
    backgroundLogging(enabled)
  }

  public func readResource(
    namespace: FloatViewResourceNamespace,
    logicalName: String,
    maximumBytes: UInt64
  ) throws -> Data {
    guard maximumBytes > 0,
          maximumBytes <= UInt64(FloatViewCoreRuntime.maximumCompletionPayloadBytes),
          Self.validLogicalName(logicalName),
          let root = resourceRoots[namespace]
    else { throw ApplePlatformHostError.invalidPath }
    let resource = root.appending(path: logicalName).standardizedFileURL
    guard Self.isDescendant(resource, of: root),
          fileManager.isReadableFile(atPath: resource.path)
    else { throw ApplePlatformHostError.invalidPath }
    let size = try resource.resourceValues(forKeys: [.fileSizeKey]).fileSize
    guard size.map({ UInt64($0) <= maximumBytes }) == true else {
      throw ApplePlatformHostError.payloadTooLarge
    }
    let data = try Data(contentsOf: resource, options: .mappedIfSafe)
    guard UInt64(data.count) <= maximumBytes else {
      throw ApplePlatformHostError.payloadTooLarge
    }
    return Data(data)
  }

  public func stageDocument(
    destination: URL,
    maximumBytes: UInt64,
    completion: @escaping @Sendable (Result<Void, ApplePlatformHostError>) -> Void
  ) {
    guard maximumBytes > 0,
          maximumBytes <= 512 * 1024 * 1024,
          storageRoots.values.contains(where: { Self.isDescendant(destination, of: $0) }),
          let documentPresenter
    else {
      completion(.failure(.invalidPath))
      return
    }
    documentPresenter(destination.standardizedFileURL, maximumBytes, completion)
  }

  public func storageRoot(_ kind: FloatViewStorageRootKind) throws -> URL {
    guard let root = storageRoots[kind] else { throw ApplePlatformHostError.unavailable }
    do {
      try fileManager.createDirectory(at: root, withIntermediateDirectories: true)
      return root.standardizedFileURL
    } catch {
      throw ApplePlatformHostError.ioFailure
    }
  }

  public func availableSpace(at root: URL) throws -> UInt64 {
    let root = root.standardizedFileURL
    guard storageRoots.values.contains(where: {
      root == $0.standardizedFileURL || Self.isDescendant(root, of: $0)
    }) else { throw ApplePlatformHostError.invalidPath }
    do {
      let values = try root.resourceValues(forKeys: [.volumeAvailableCapacityForImportantUsageKey])
      guard let capacity = values.volumeAvailableCapacityForImportantUsage, capacity >= 0 else {
        throw ApplePlatformHostError.unavailable
      }
      return UInt64(capacity)
    } catch let error as ApplePlatformHostError {
      throw error
    } catch {
      throw ApplePlatformHostError.ioFailure
    }
  }

  public func monotonicMilliseconds() -> UInt64 {
    DispatchTime.now().uptimeNanoseconds / 1_000_000
  }

  public func wallMilliseconds() -> Int64 {
    Int64(Date().timeIntervalSince1970 * 1_000)
  }

  public func secureRandom(count: UInt16) throws -> Data {
    guard count > 0, count <= 256 else { throw ApplePlatformHostError.payloadTooLarge }
    var bytes = [UInt8](repeating: 0, count: Int(count))
    guard SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes) == errSecSuccess else {
      throw ApplePlatformHostError.unavailable
    }
    return Data(bytes)
  }

  public func diagnostic(stage: String, code: String) {
    diagnosticHandler(stage, code)
  }

  public func presentNotificationsPermission() -> Bool {
    notificationPresenter()
  }

  public func presentTheme(_ theme: FloatViewThemePreference) -> Bool {
    themePresenter(theme)
  }

  public func presentServiceState(active: Bool) -> Bool {
    servicePresenter(active)
  }

  public func presentExport(
    stagedURL: URL,
    completion: @escaping @Sendable (Result<Void, ApplePlatformHostError>) -> Void
  ) {
    guard storageRoots.values.contains(where: { Self.isDescendant(stagedURL, of: $0) }),
          fileManager.isReadableFile(atPath: stagedURL.path),
          let exportPresenter
    else {
      completion(.failure(.invalidPath))
      return
    }
    exportPresenter(stagedURL.standardizedFileURL, completion)
  }

  public func cancelPendingPresentation() {
    presentationCanceller()
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

  private static func validLogicalName(_ value: String) -> Bool {
    !value.isEmpty && value.utf8.count <= 128 && !value.hasPrefix("/")
      && !value.hasPrefix("\\") && !value.contains("..") && !value.contains(":")
      && value.split(separator: "/").allSatisfy { !$0.isEmpty && $0 != "." }
      && value.utf8.allSatisfy {
        ($0 >= 48 && $0 <= 57) || ($0 >= 65 && $0 <= 90) || ($0 >= 97 && $0 <= 122)
          || $0 == 45 || $0 == 46 || $0 == 47 || $0 == 95
      }
  }

  private static func isDescendant(_ candidate: URL, of root: URL) -> Bool {
    let candidate = candidate.standardizedFileURL.path
    let root = root.standardizedFileURL.path
    return candidate == root || candidate.hasPrefix(root.hasSuffix("/") ? root : root + "/")
  }
}
