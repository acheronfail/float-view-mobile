#if canImport(FloatViewCoreBridge)
  import FloatViewCoreBridge
#endif
import Foundation
import UserNotifications
#if os(iOS)
  import UIKit
  import UniformTypeIdentifiers
#elseif os(macOS)
  import AppKit
#endif

/// UIKit/AppKit-only presentation owner used by `ApplePlatformHostServices`.
/// It copies or presents files but never interprets archive or export contents.
public final class AppleSystemPresentation: NSObject, @unchecked Sendable {
  private let lock = NSLock()
  #if os(macOS)
    private weak var activePanel: NSPanel?
  #elseif os(iOS)
    private enum Pending {
      case document(
        destination: URL,
        maximumBytes: UInt64,
        completion: @Sendable (Result<Void, ApplePlatformHostError>) -> Void
      )
      case export(completion: @Sendable (Result<Void, ApplePlatformHostError>) -> Void)
    }

    private var pending: Pending?
    private weak var activePicker: UIDocumentPickerViewController?
  #endif

  public var documentPresenter: ApplePlatformHostServices.DocumentPresenter {
    { [weak self] destination, maximumBytes, completion in
      self?.presentDocument(
        destination: destination,
        maximumBytes: maximumBytes,
        completion: completion
      ) ?? completion(.failure(.unavailable))
    }
  }

  public var exportPresenter: ApplePlatformHostServices.ExportPresenter {
    { [weak self] stagedURL, completion in
      self?.presentExport(stagedURL: stagedURL, completion: completion)
        ?? completion(.failure(.unavailable))
    }
  }

  public var notificationPresenter: @Sendable () -> Bool {
    {
      UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound]) { _, _ in }
      return true
    }
  }

  public var themePresenter: @Sendable (FloatViewThemePreference) -> Bool {
    { theme in
      DispatchQueue.main.async { Self.apply(theme) }
      return true
    }
  }

  public func cancel() {
    #if os(macOS)
      DispatchQueue.main.async { [weak self] in self?.activePanel?.close() }
    #elseif os(iOS)
      DispatchQueue.main.async { [weak self] in
        guard let self else { return }
        activePicker?.dismiss(animated: true)
        finish(.failure(.unavailable))
      }
    #endif
  }

  private func presentDocument(
    destination: URL,
    maximumBytes: UInt64,
    completion: @escaping @Sendable (Result<Void, ApplePlatformHostError>) -> Void
  ) {
    #if os(macOS)
      DispatchQueue.main.async { [weak self] in
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = false
        self?.activePanel = panel
        panel.begin { response in
          self?.activePanel = nil
          guard response == .OK, let source = panel.url else {
            completion(.failure(.unavailable))
            return
          }
          DispatchQueue.global(qos: .utility).async {
            completion(Result {
              try ApplePlatformHostServices.copySecurityScopedDocument(
                source: source,
                destination: destination,
                maximumBytes: maximumBytes
              )
            }.mapError(Self.map))
          }
        }
      }
    #elseif os(iOS)
      DispatchQueue.main.async { [weak self] in
        guard let self, let presenter = Self.presentingViewController(),
              self.lock.withLock({ self.pending == nil })
        else {
          completion(.failure(.unavailable))
          return
        }
        let picker = UIDocumentPickerViewController(forOpeningContentTypes: [.data], asCopy: false)
        picker.allowsMultipleSelection = false
        picker.delegate = self
        self.lock.withLock {
          self.pending = .document(
            destination: destination,
            maximumBytes: maximumBytes,
            completion: completion
          )
          self.activePicker = picker
        }
        presenter.present(picker, animated: true)
      }
    #endif
  }

  private func presentExport(
    stagedURL: URL,
    completion: @escaping @Sendable (Result<Void, ApplePlatformHostError>) -> Void
  ) {
    #if os(macOS)
      DispatchQueue.main.async { [weak self] in
        let panel = NSSavePanel()
        panel.nameFieldStringValue = stagedURL.lastPathComponent
        self?.activePanel = panel
        panel.begin { response in
          self?.activePanel = nil
          guard response == .OK, let destination = panel.url else {
            completion(.failure(.unavailable))
            return
          }
          DispatchQueue.global(qos: .utility).async {
            do {
              if FileManager.default.fileExists(atPath: destination.path) {
                try FileManager.default.removeItem(at: destination)
              }
              try FileManager.default.copyItem(at: stagedURL, to: destination)
              completion(.success(()))
            } catch {
              completion(.failure(.ioFailure))
            }
          }
        }
      }
    #elseif os(iOS)
      DispatchQueue.main.async { [weak self] in
        guard let self, let presenter = Self.presentingViewController(),
              self.lock.withLock({ self.pending == nil })
        else {
          completion(.failure(.unavailable))
          return
        }
        let picker = UIDocumentPickerViewController(forExporting: [stagedURL], asCopy: true)
        picker.delegate = self
        self.lock.withLock {
          self.pending = .export(completion: completion)
          self.activePicker = picker
        }
        presenter.present(picker, animated: true)
      }
    #endif
  }

  private static func map(_ error: Error) -> ApplePlatformHostError {
    error as? ApplePlatformHostError ?? .ioFailure
  }

  @MainActor private static func apply(_ theme: FloatViewThemePreference) {
    #if os(macOS)
      NSApp.appearance = switch theme {
      case .system: nil
      case .light: NSAppearance(named: .aqua)
      case .dark: NSAppearance(named: .darkAqua)
      }
    #elseif os(iOS)
      let style: UIUserInterfaceStyle = switch theme {
      case .system: .unspecified
      case .light: .light
      case .dark: .dark
      }
      for scene in UIApplication.shared.connectedScenes.compactMap({ $0 as? UIWindowScene }) {
        for window in scene.windows {
          window.overrideUserInterfaceStyle = style
        }
      }
    #endif
  }

  #if os(iOS)
    @MainActor private static func presentingViewController() -> UIViewController? {
      let root = UIApplication.shared.connectedScenes
        .compactMap { $0 as? UIWindowScene }
        .flatMap(\.windows)
        .first(where: \.isKeyWindow)?.rootViewController
      var current = root
      while let presented = current?.presentedViewController {
        current = presented
      }
      if let navigation = current as? UINavigationController { return navigation.visibleViewController }
      if let tabs = current as? UITabBarController { return tabs.selectedViewController }
      return current
    }

    private func finish(_ result: Result<Void, ApplePlatformHostError>) {
      let completion: (@Sendable (Result<Void, ApplePlatformHostError>) -> Void)? = lock.withLock {
        defer {
          pending = nil
          activePicker = nil
        }
        guard let pending else { return nil }
        switch pending {
        case let .document(_, _, completion): return completion
        case let .export(completion): return completion
        }
      }
      completion?(result)
    }
  #endif
}

#if os(iOS)
  extension AppleSystemPresentation: UIDocumentPickerDelegate {
    public func documentPickerWasCancelled(_: UIDocumentPickerViewController) {
      finish(.failure(.unavailable))
    }

    public func documentPicker(
      _: UIDocumentPickerViewController,
      didPickDocumentsAt urls: [URL]
    ) {
      let pending = lock.withLock { self.pending }
      switch pending {
      case let .document(destination, maximumBytes, _):
        guard let source = urls.first else {
          finish(.failure(.unavailable))
          return
        }
        DispatchQueue.global(qos: .utility).async { [weak self] in
          let result: Result<Void, ApplePlatformHostError> = Result {
            try ApplePlatformHostServices.copySecurityScopedDocument(
              source: source,
              destination: destination,
              maximumBytes: maximumBytes
            )
          }.mapError(Self.map)
          self?.finish(result)
        }
      case .export:
        finish(.success(()))
      case nil:
        break
      }
    }
  }
#endif
