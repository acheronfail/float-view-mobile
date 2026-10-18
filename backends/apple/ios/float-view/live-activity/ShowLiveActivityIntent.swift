import AppIntents

/// Explicit recovery also respects a person's earlier dismissal until invoked.
struct ShowLiveActivityIntent: AppIntent {
  static let title: LocalizedStringResource = "Show Live Activity"
  static let description = IntentDescription("Show the Live Activity for the board connected in Float View.")
  static let openAppWhenRun = true

  @MainActor
  func perform() async throws -> some IntentResult {
    RideActivityController.requestPresentation()
    return .result()
  }
}

struct FloatViewShortcuts: AppShortcutsProvider {
  static var appShortcuts: [AppShortcut] {
    AppShortcut(
      intent: ShowLiveActivityIntent(),
      phrases: ["Show my board in \(.applicationName)"],
      shortTitle: "Show Live Activity",
      systemImageName: "waveform"
    )
  }
}
