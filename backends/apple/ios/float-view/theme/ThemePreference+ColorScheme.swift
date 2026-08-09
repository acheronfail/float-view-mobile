import SwiftUI

extension ThemePreference {
  var preferredColorScheme: ColorScheme? {
    switch self {
    case .system: nil
    case .light: .light
    case .dark: .dark
    }
  }
}
