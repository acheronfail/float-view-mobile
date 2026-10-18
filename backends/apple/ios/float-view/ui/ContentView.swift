import SwiftUI
import UIKit

struct ContentView: View {
  @StateObject private var backend = AppBackend()
  @Environment(\.scenePhase) private var scenePhase
  @State private var loadedGeneration: Int?
  @State private var failedGeneration: Int?

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
          .opacity(loadedGeneration == frontend.generation ? 1 : 0)
          .allowsHitTesting(loadedGeneration == frontend.generation)
          .accessibilityHidden(loadedGeneration != frontend.generation)
          .ignoresSafeArea()
      }
      if (backend.serviceStartupFailed || backend.frontendStartupFailed), scenePhase == .active {
        ContentUnavailableView {
          Label("Unable to start Float View", systemImage: "exclamationmark.triangle")
        } description: {
          Text("Float View could not open its data. If this data came from a newer build, install a compatible version. Otherwise check available storage and try again. Keep this installation until you have a backup.")
        } actions: {
          Button("Try again") { backend.retryStartup() }
        }
      } else if backend.frontend == nil || loadedGeneration != backend.frontend?.generation {
        VStack(spacing: 20) {
          Image("LoadingSymbol")
            .resizable()
            .frame(width: 72, height: 96)
            .foregroundStyle(Color(uiColor: .loadingSymbol))
            .accessibilityHidden(true)
          Text(
            failedGeneration != nil && failedGeneration == backend.frontend?.generation
              ? "Unable to open Float View" : "Loading…"
          )
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
      ? UIColor(red: 162 / 255, green: 212 / 255, blue: 216 / 255, alpha: 1)
      : UIColor(red: 15 / 255, green: 118 / 255, blue: 110 / 255, alpha: 1)
  }
}
