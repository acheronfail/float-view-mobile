import AppKit
import FloatViewBackend
import Foundation

enum MacExportPresenter {
  static func schedule(_ value: NativeFilePresentation) -> Bool {
    DispatchQueue.main.async {
      let panel = NSSavePanel()
      panel.nameFieldStringValue = value.fileName
      panel.canCreateDirectories = true
      panel.begin { response in
        defer { value.finish() }
        guard response == .OK, let destination = panel.url else { return }
        do {
          if FileManager.default.fileExists(atPath: destination.path) {
            try FileManager.default.removeItem(at: destination)
          }
          try FileManager.default.copyItem(at: value.fileURL, to: destination)
        } catch {
          trace("[FloatViewState] export presentation failed")
        }
      }
    }
    return true
  }
}
