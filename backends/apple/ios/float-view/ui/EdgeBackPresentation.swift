import UIKit
import WebKit

/// One ephemeral viewport image: no second document, route state or persistent screenshots.
final class EdgeBackPresentation {
  private let stage = UIView()
  private let backdrop = UIView()
  private let page: UIImageView
  private let hint = UILabel()
  private let width: CGFloat

  init?(webView: WKWebView, image: UIImage) {
    guard let parent = webView.superview else { return nil }
    width = webView.bounds.width
    page = UIImageView(image: image)
    stage.frame = webView.convert(webView.bounds, to: parent)
    stage.clipsToBounds = true
    stage.accessibilityElementsHidden = true
    backdrop.frame = stage.bounds
    backdrop.backgroundColor = webView.backgroundColor
    stage.addSubview(backdrop)
    hint.text = "‹  Back"
    hint.font = .preferredFont(forTextStyle: .headline)
    hint.textColor = .secondaryLabel
    hint.frame = CGRect(x: 24, y: stage.bounds.midY - 22, width: 140, height: 44)
    backdrop.addSubview(hint)
    page.frame = stage.bounds
    page.layer.shadowColor = UIColor.black.cgColor
    page.layer.shadowOpacity = 0.2
    page.layer.shadowRadius = 12
    page.layer.shadowOffset = CGSize(width: -4, height: 0)
    stage.addSubview(page)
    parent.addSubview(stage)
  }

  func track(_ offset: CGFloat) {
    page.transform = CGAffineTransform(translationX: max(0, min(width, offset)), y: 0)
    hint.alpha = min(1, max(0, offset) / 80)
  }

  func settle(completed: Bool, completion: @escaping () -> Void) {
    UIView.animate(withDuration: UIAccessibility.isReduceMotionEnabled ? 0 : 0.18,
                   delay: 0, options: [.beginFromCurrentState, .curveEaseOut])
    {
      self.page.transform = CGAffineTransform(translationX: completed ? self.width : 0, y: 0)
      // Reveal the restored parent gradually, in the same animation as the outgoing page.
      if completed {
        self.backdrop.alpha = 0
      }
    } completion: { _ in
      self.remove()
      completion()
    }
  }

  func remove() {
    page.layer.removeAllAnimations()
    backdrop.layer.removeAllAnimations()
    stage.removeFromSuperview()
  }
}
