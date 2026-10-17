enum MacStartupStage {
  case data
  case connection

  var title: String {
    switch self {
    case .data: "Unable to open Float View data"
    case .connection: "Unable to start Float View’s local connection"
    }
  }

  var recovery: String {
    switch self {
    case .data:
      "Float View could not open its data. Quit any other copies of Float View and try again. If this data came from a newer build, install a compatible version. Otherwise check available storage. Keep this installation until you have a backup."
    case .connection:
      "Another copy of Float View may already be running. Quit any other copies, including test builds, then choose Try again. If this keeps happening, quit Float View and restart your Mac."
    }
  }
}
