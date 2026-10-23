import SwiftUI
import UIKit

struct ContentView: View {
  @StateObject private var backend = AppBackend()
  @Environment(\.scenePhase) private var scenePhase
  @State private var loadedGeneration: Int?
  @State private var failedGeneration: Int?

  private var visibleFailure: StartupFailure? {
    if let failure = backend.startupFailure {
      return failure
    }
    if let failedGeneration, failedGeneration == backend.frontend?.generation {
      return .interface
    }
    return nil
  }

  var body: some View {
    ZStack {
      Color("Canvas").ignoresSafeArea()
      if let frontend = backend.frontend {
        WebView(
          url: frontend.origin,
          onContentProcessTerminated: backend.recoverFrontend,
          onLoadComplete: { loadedGeneration = frontend.generation },
          onLoadFailure: { failedGeneration = frontend.generation }
        )
        .id(frontend.generation)
        .opacity(loadedGeneration == frontend.generation && visibleFailure == nil ? 1 : 0)
        .allowsHitTesting(loadedGeneration == frontend.generation && visibleFailure == nil)
        .accessibilityHidden(loadedGeneration != frontend.generation || visibleFailure != nil)
        .ignoresSafeArea()
      }
      if let failure = visibleFailure, scenePhase == .active {
        ScrollView {
          ContentUnavailableView {
            Label(failure.title, systemImage: "exclamationmark.triangle")
          } description: {
            Text(failure.recovery(on: .iOS))
          } actions: {
            Button("Try again") {
              failedGeneration = nil
              backend.retryStartup()
            }
          }
        }
      } else if backend.frontend == nil || loadedGeneration != backend.frontend?.generation {
        VStack(spacing: 20) {
          Image("LoadingSymbol")
            .resizable()
            .frame(width: 72, height: 96)
            .foregroundStyle(Color(uiColor: .loadingSymbol))
            .accessibilityHidden(true)
          Text("Loading…")
            .font(.system(size: 15))
            .foregroundStyle(.secondary)
        }
        .accessibilityElement(children: .combine)
      }
    }
    .preferredColorScheme(backend.themePreference.preferredColorScheme)
    .onChange(of: scenePhase, initial: true) { _, phase in
      switch phase {
      case .active:
        backend.enterForeground()
      case .background:
        backend.enterBackground()
      case .inactive:
        break
      @unknown default:
        break
      }
    }
  }
}

private extension UIColor {
  static let loadingSymbol = UIColor { traits in
    traits.userInterfaceStyle == .dark
      ? UIColor(red: 45 / 255, green: 212 / 255, blue: 191 / 255, alpha: 1)
      : UIColor(red: 20 / 255, green: 184 / 255, blue: 166 / 255, alpha: 1)
  }
}
