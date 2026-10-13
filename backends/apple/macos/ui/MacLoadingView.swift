import AppKit

@MainActor
final class MacLoadingView: NSView {
  private let message = NSTextField(labelWithString: "Loading…")

  override init(frame: NSRect) {
    super.init(frame: frame)
    let symbol = NSImageView()
    if let url = MacResources.url(forResource: "LoadingSymbol", withExtension: "png"),
       let image = NSImage(contentsOf: url)
    {
      image.isTemplate = true
      symbol.image = image
    }
    symbol.contentTintColor = NSColor(name: nil) { appearance in
      if appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua {
        return NSColor(srgbRed: 162 / 255, green: 212 / 255, blue: 216 / 255, alpha: 1)
      }
      return NSColor(srgbRed: 14 / 255, green: 116 / 255, blue: 144 / 255, alpha: 1)
    }
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
  required init?(coder: NSCoder) { fatalError("init(coder:) is unavailable") }

  func showFailure() { message.stringValue = "Unable to open Float View" }
}
