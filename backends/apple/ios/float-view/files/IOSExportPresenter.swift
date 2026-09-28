import Foundation
import UIKit

enum IOSExportPresenter {
  static func schedule(_ value: NativeFilePresentation) -> Bool {
    DispatchQueue.main.async {
      present(value)
    }
    return true
  }

  @MainActor
  private static func present(_ value: NativeFilePresentation) {
    guard let scene = UIApplication.shared.connectedScenes
      .compactMap({ $0 as? UIWindowScene })
      .first(where: { $0.activationState == .foregroundActive }),
      let root = scene.windows.first(where: \.isKeyWindow)?.rootViewController
    else { value.finish(); return }
    var presenter = root
    while let presented = presenter.presentedViewController {
      presenter = presented
    }
    let controller = UIActivityViewController(activityItems: [value.fileURL], applicationActivities: nil)
    controller.completionWithItemsHandler = { _, _, _, _ in value.finish() }
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
