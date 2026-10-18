/// Native recovery is available even when the embedded interface cannot load.
public enum StartupFailure: Sendable {
  case data
  case connection
  case interface

  public var title: String {
    switch self {
    case .data: "Unable to open Float View data"
    case .connection: "Unable to start Float View’s local connection"
    case .interface: "Unable to load Float View"
    }
  }

  public enum Platform: Sendable { case macOS, iOS }

  public func recovery(on platform: Platform) -> String {
    switch self {
    case .data:
      switch platform {
      case .macOS:
        "Float View could not open its saved rides and settings. Quit any other copies of Float View and check available storage, then try again. If you used a newer version of Float View, install that version or later. Keep your app data until you have a backup."
      case .iOS:
        "Float View could not open its saved rides and settings. Check available storage in Settings, then try again. If you used a newer version of Float View, install that version or later. Do not delete the app, as this can erase your saved rides and settings."
      }
    case .connection:
      switch platform {
      case .macOS:
        "Float View could not start the local connection used by its interface. Quit any other copies of Float View, including test builds, then try again. If the problem continues, restart your Mac and reopen Float View."
      case .iOS:
        "Float View could not start the local connection used by its interface. Try again. If the problem continues, restart your iPhone or iPad and reopen Float View."
      }
    case .interface:
      "Float View could not load its interface. Try again. If the problem continues, close and reopen the app."
    }
  }
}
