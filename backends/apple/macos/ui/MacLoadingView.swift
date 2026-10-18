import AppKit
import FloatViewBackend

@MainActor
final class MacLoadingView: NSView {
  private let symbol = NSImageView()
  private let message = NSTextField(labelWithString: "Loading…")

  private let detail = NSTextField(wrappingLabelWithString: "")
  private let retry = NSButton(title: "Try again", target: nil, action: nil)
  private var onRetry: (() -> Void)?

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
    detail.alignment = .center
    detail.textColor = .secondaryLabelColor
    detail.isHidden = true
    retry.isHidden = true
    retry.target = self
    retry.action = #selector(tryAgain)
    let stack = NSStackView(views: [symbol, message, detail, retry])
    stack.orientation = .vertical
    stack.alignment = .centerX
    stack.spacing = 20
    stack.translatesAutoresizingMaskIntoConstraints = false
    addSubview(stack)
    NSLayoutConstraint.activate([
      detail.widthAnchor.constraint(lessThanOrEqualToConstant: 420),
      stack.widthAnchor.constraint(lessThanOrEqualTo: widthAnchor, constant: -48),
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

  func showFailure(_ failure: StartupFailure, onRetry: @escaping () -> Void) {
    message.stringValue = failure.title
    detail.stringValue = failure.recovery(on: .macOS)
    detail.isHidden = false
    retry.isHidden = false
    self.onRetry = onRetry
  }

  @objc private func tryAgain() {
    onRetry?()
  }
}
