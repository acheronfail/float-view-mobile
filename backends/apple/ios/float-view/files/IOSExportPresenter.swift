import Foundation
import UIKit

enum IOSExportPresenter {
  static func schedule(_ value: SessionExportRecord) -> Bool {
    DispatchQueue.main.async {
      present(value.fileURL)
    }
    return true
  }

  @MainActor
  private static func present(_ fileURL: URL) {
    guard let scene = UIApplication.shared.connectedScenes
      .compactMap({ $0 as? UIWindowScene })
      .first(where: { $0.activationState == .foregroundActive }),
      let root = scene.windows.first(where: \.isKeyWindow)?.rootViewController
    else { return }
    var presenter = root
    while let presented = presenter.presentedViewController {
      presenter = presented
    }
    let controller = UIActivityViewController(activityItems: [fileURL], applicationActivities: nil)
    if let popover = controller.popoverPresentationController {
      popover.sourceView = presenter.view
      popover.sourceRect = CGRect(
        x: presenter.view.bounds.midX,
        y: presenter.view.bounds.midY,
        width: 1,
        height: 1
      )
    }
    presenter.present(controller, animated: true)
  }
}
