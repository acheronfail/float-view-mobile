import Foundation

extension AppBackend {
  func enterBackground() {
    service.setForeground(false)
    guard frontend != nil else { return }
    #if DEBUG
      print("[FloatViewState] frontend state=stopped reason=background")
    #endif
    frontend = nil
    frontendServer?.stop()
  }

  func enterForeground() {
    service.setForeground(true)
    guard frontend == nil else { return }
    activateFrontend()
  }

  func recoverFrontend() {
    frontend = nil
    frontendServer?.stop()
    activateFrontend()
  }
}
