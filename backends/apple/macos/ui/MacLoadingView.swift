import AppKit

@MainActor
final class MacLoadingView: NSView {
  private let symbol = NSImageView()
  private let message = NSTextField(labelWithString: "Loading…")

  override init(frame: NSRect) {
    super.init(frame: frame)
    if let url = MacResources.url(forResource: "LoadingSymbol", withExtension: "png"),
       let image = NSImage(contentsOf: url)
    {
      image.isTemplate = true
      symbol.image = image
    }
    updateAppearance()
    symbol.setAccessibilityElement(false)
    message.font = .systemFont(ofSize: 15)
    message.textColor = .secondaryLabelColor
    let stack = NSStackView(views: [symbol, message])
    stack.orientation = .vertical
    stack.alignment = .centerX
    stack.spacing = 20
    stack.translatesAutoresizingMaskIntoConstraints = false
    addSubview(stack)
    NSLayoutConstraint.activate([
      symbol.widthAnchor.constraint(equalToConstant: 72),
      symbol.heightAnchor.constraint(equalToConstant: 96),
      stack.centerXAnchor.constraint(equalTo: centerXAnchor),
      stack.centerYAnchor.constraint(equalTo: centerYAnchor),
    ])
  }

  @available(*, unavailable)
  required init?(coder _: NSCoder) {
    fatalError("init(coder:) is unavailable")
  }

  override func viewDidChangeEffectiveAppearance() {
    super.viewDidChangeEffectiveAppearance()
    updateAppearance()
  }

  private func updateAppearance() {
    let dark = effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
    // Resolve the template tint for this view and refresh it after window or
    // system appearance changes, including while startup is still pending.
    symbol.contentTintColor = dark
      ? NSColor(srgbRed: 162 / 255, green: 212 / 255, blue: 216 / 255, alpha: 1)
      : NSColor(srgbRed: 15 / 255, green: 118 / 255, blue: 110 / 255, alpha: 1)
  }

  func showFailure() {
    message.stringValue = "Unable to open Float View"
  }
}
