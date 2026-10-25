import AppKit
@testable import FloatViewMac
import XCTest

final class MacWindowMenuTests: XCTestCase {
  @MainActor
  func testReopenCommandRemainsEnabledWithoutAWindow() throws {
    let application = NSApplication.shared
    let originalDelegate = application.delegate
    let originalMainMenu = application.mainMenu
    let originalWindowsMenu = application.windowsMenu
    let originalServicesMenu = application.servicesMenu
    let originalHelpMenu = application.helpMenu
    let target = WindowMenuReceiver()
    application.delegate = target
    defer {
      application.delegate = originalDelegate
      application.mainMenu = originalMainMenu
      application.windowsMenu = originalWindowsMenu
      application.servicesMenu = originalServicesMenu
      application.helpMenu = originalHelpMenu
    }
    installMainMenu(on: application)
    let menu = try XCTUnwrap(application.windowsMenu)
    let item = try XCTUnwrap(menu.item(withTitle: "Show Float View"))
    XCTAssertTrue(item.target === target)
    XCTAssertEqual(item.keyEquivalent, "0")
    XCTAssertEqual(item.keyEquivalentModifierMask, .command)
    menu.update()
    XCTAssertTrue(item.isEnabled)
    menu.performActionForItem(at: menu.index(of: item))
    XCTAssertEqual(target.requests, 1)
    // Closing the frontmost window must not remove or disable the explicit command.
    let window = NSWindow(
      contentRect: NSRect(x: 0, y: 0, width: 200, height: 200),
      styleMask: [.titled, .closable], backing: .buffered, defer: false
    )
    window.isReleasedWhenClosed = false
    window.makeKeyAndOrderFront(nil)
    window.close()
    menu.update()
    XCTAssertTrue(item.isEnabled)
    menu.performActionForItem(at: menu.index(of: item))
    XCTAssertEqual(target.requests, 2)
  }

  @MainActor
  func testApplicationDelegateHandlesTheExplicitReopenCommand() {
    let delegate = MacAppDelegate(loadHTML: { Data() })
    XCTAssertTrue(delegate.responds(to: #selector(MacAppDelegate.showMainWindow(_:))))
    XCTAssertFalse(delegate.applicationShouldTerminateAfterLastWindowClosed(NSApplication.shared))
  }
}

@MainActor
private final class WindowMenuReceiver: NSObject, NSApplicationDelegate {
  var requests = 0

  @objc func showMainWindow(_: Any?) {
    requests += 1
  }
}
