import Foundation
#if os(iOS)
  import UIKit
#else
  import AppKit
#endif

/// Closed native access observations; public inventory and actions are Rust-owned.
public struct LocationAccess: Sendable {
  public let grant: String
  public let accuracy: String
  public let scope: String
  public let services: Bool?
  public let requestable: Bool
  public init(
    grant: String,
    accuracy: String = "unknown",
    scope: String = "unknown",
    services: Bool? = nil,
    requestable: Bool = false
  ) {
    self.grant = grant; self.accuracy = accuracy; self.scope = scope; self.services = services; self
      .requestable = requestable
  }
}

public extension LocationProviderPort {
  func access() -> LocationAccess {
    LocationAccess(grant: "unknown")
  }

  func refreshAuthorization(_ completion: @escaping @Sendable () -> Void) {
    completion()
  }

  func recoverAuthorization(_ completion: @escaping @Sendable (Bool) -> Void) {
    completion(false)
  }
}

func openPermissionSettings(_ permission: String, completion: @escaping @Sendable (Bool) -> Void) {
  DispatchQueue.main.async {
    #if os(iOS)
      guard UIApplication.shared.applicationState == .active,
            let url = URL(string: UIApplication.openSettingsURLString) else { completion(false); return }
      UIApplication.shared.open(url, options: [:], completionHandler: completion)
    #else
      let pane = permission == "bluetooth" ? "Privacy_Bluetooth" : "Privacy_LocationServices"
      guard let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?\(pane)")
      else { completion(false); return }
      completion(NSWorkspace.shared.open(url))
    #endif
  }
}
