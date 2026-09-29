import AppKit
import FloatViewBackend

@MainActor
enum MacTheme {
  static func apply(_ preference: ThemePreference, to application: NSApplication) {
    switch preference {
    case .system: application.appearance = nil
    case .light: application.appearance = NSAppearance(named: .aqua)
    case .dark: application.appearance = NSAppearance(named: .darkAqua)
    }
  }

  /// Resolve against each window's appearance, including after window recreation.
  static let canvasColor = NSColor(name: nil) { appearance in
    if appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua {
      return NSColor(srgbRed: 2 / 255, green: 6 / 255, blue: 23 / 255, alpha: 1)
    }
    return NSColor(srgbRed: 248 / 255, green: 250 / 255, blue: 252 / 255, alpha: 1)
  }
}
