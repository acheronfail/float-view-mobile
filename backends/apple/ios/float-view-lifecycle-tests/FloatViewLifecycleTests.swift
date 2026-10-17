import UIKit
import XCTest

final class FloatViewLifecycleTests: XCTestCase {
  private let application = XCUIApplication(bundleIdentifier: "fail.acheron.floatview")

  private func acceptSafetyIfNeeded() {
    let agreement = application.buttons["I agree"]
    let ready = NSPredicate { [self] _, _ in
      agreement.exists || application.links["settings"].isHittable
    }
    expectation(for: ready, evaluatedWith: nil)
    waitForExpectations(timeout: 15)
    if agreement.exists {
      agreement.tap()
    }
  }

  func testStartupAndForegroundPresentation() {
    for _ in 0 ..< 3 {
      application.terminate()
      application.launch()
      acceptSafetyIfNeeded()
      XCTAssertTrue(
        application.links["settings"].waitForExistence(timeout: 15),
        "Startup did not expose frontend navigation"
      )
      XCTAssertFalse(application.staticTexts["Unable to start Float View"].exists)
      XCUIDevice.shared.press(.home)
      application.activate()
      XCTAssertTrue(
        application.links["settings"].waitForExistence(timeout: 15),
        "Foreground reconstruction did not expose navigation"
      )
      XCTAssertFalse(application.staticTexts["Unable to start Float View"].exists)
    }
  }

  #if targetEnvironment(simulator)
    /// Run only on a disposable iPad simulator; requires no radio or private data.
    func testIPadRotationKeepsLicencesUsable() throws {
      guard UIDevice.current.userInterfaceIdiom == .pad else {
        throw XCTSkip("Requires an iPad simulator")
      }
      continueAfterFailure = false
      defer { XCUIDevice.shared.orientation = .portrait }
      application.launch()
      acceptSafetyIfNeeded()
      application.links["settings"].tap()
      let link = application.links["View licences and notices"]
      for _ in 0..<15 {
        if link.isHittable, link.frame.height > 1,
          link.frame.minY > application.frame.minY + 120,
          link.frame.maxY < application.links["settings"].frame.minY - 12
        {
          break
        }
        application.swipeUp()
      }
      XCTAssertTrue(link.isHittable)
      link.tap()
      let panel = application.buttons["Project licence"]
      XCTAssertTrue(panel.waitForExistence(timeout: 10))
      for orientation in [
        UIDeviceOrientation.portrait, .landscapeLeft, .portraitUpsideDown, .landscapeRight,
        .portrait,
      ] {
        XCUIDevice.shared.orientation = orientation
        let landscape = orientation.isLandscape
        let rotated = NSPredicate { [self] _, _ in
          let frame = application.frame
          return landscape ? frame.width > frame.height : frame.height > frame.width
        }
        expectation(for: rotated, evaluatedWith: nil)
        waitForExpectations(timeout: 10)
        XCTAssertTrue(panel.isHittable)
        XCTAssertTrue(
          application.frame.contains(panel.frame), "Licence control escaped the resized viewport")
        XCTAssertTrue(application.links["Settings"].isHittable)
      }
      application.links["Settings"].tap()
      XCTAssertTrue(application.links["settings"].isHittable)
    }

    /// Run on a disposable simulator with the synthetic baseline catalog and
    /// Safari as its default browser. No personal agreement/contact is submitted.
    func testExternalPrivacySupportAndSourceLinks() throws {
      continueAfterFailure = false
      application.launch()
      if application.buttons["I agree"].waitForExistence(timeout: 3) {
        throw XCTSkip("Requires a disposable simulator with the synthetic accepted baseline catalog")
      }
      XCTAssertTrue(application.links["settings"].waitForExistence(timeout: 15))
      application.links["settings"].tap()
      let safari = XCUIApplication(bundleIdentifier: "com.apple.mobilesafari")
      for label in ["Privacy policy", "Support", "Source code and build instructions"] {
        let link = application.links[label]
        // XCTest can report covered WebView links as hittable. Keep the whole
        // link above the fixed navigation bar before synthesizing its tap.
        let navigation = application.links["settings"]
        for _ in 0 ..< 12 {
          if link.isHittable, link.frame.height > 1,
             link.frame.minY > application.frame.minY + 120,
             link.frame.maxY < navigation.frame.minY - 12
          {
            break
          }
          application.swipeUp()
        }
        XCTAssertTrue(link.isHittable, "Missing external link: \(label)")
        XCTAssertGreaterThan(link.frame.height, 1, "Link has no visible frame")
        XCTAssertGreaterThan(link.frame.minY, application.frame.minY + 120, "Link is above the visible document")
        XCTAssertLessThan(link.frame.maxY, navigation.frame.minY - 12, "Link is covered by navigation")
        let tapFrame = link.frame
        link.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
        XCTAssertTrue(safari.wait(for: .runningForeground, timeout: 15),
                      "Link did not open Safari: \(label); link=\(tapFrame), tabs=\(navigation.frame)")
        application.activate()
        XCTAssertTrue(link.waitForExistence(timeout: 10), "Returning lost the Settings page")
      }
    }
  #endif

  func testPermissionsRecovery() {
    continueAfterFailure = false
    application.terminate()
    application.resetAuthorizationStatus(for: .location)
    application.launch()
    acceptSafetyIfNeeded()
    XCTAssertTrue(application.links["settings"].waitForExistence(timeout: 15))
    application.links["settings"].tap()
    let request = application.buttons["Request access"].firstMatch
    if !request.isHittable {
      application.swipeUp()
    }
    if !request.waitForExistence(timeout: 3) {
      // Some physical iOS versions retain a prior denied choice through XCTest's reset.
      let open = application.buttons["Open settings"].firstMatch
      XCTAssertTrue(open.waitForExistence(timeout: 10))
      open.tap()
      let settings = XCUIApplication(bundleIdentifier: "com.apple.Preferences")
      XCTAssertTrue(settings.wait(for: .runningForeground, timeout: 10))
      let location = settings.cells.containing(.staticText, identifier: "Location").firstMatch
      XCTAssertTrue(location.waitForExistence(timeout: 10), settings.debugDescription)
      location.tap()
      let ask = settings.cells.matching(NSPredicate(format: "label CONTAINS[c] %@", "Ask Next Time")).firstMatch
      XCTAssertTrue(ask.waitForExistence(timeout: 10), settings.debugDescription)
      ask.tap()
      application.activate()
      XCTAssertTrue(application.links["settings"].waitForExistence(timeout: 15))
      application.links["settings"].tap()
      if !request.isHittable {
        application.swipeUp()
      }
    }
    XCTAssertTrue(request.waitForExistence(timeout: 10), application.debugDescription)
    request.tap()
    let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")
    let deny = springboard.buttons.matching(NSPredicate(format: "label MATCHES %@", "Don.t Allow")).firstMatch
    XCTAssertTrue(deny.waitForExistence(timeout: 10), springboard.debugDescription)
    deny.tap()
    let open = application.buttons["Open settings"].firstMatch
    XCTAssertTrue(open.waitForExistence(timeout: 10), application.debugDescription)
    open.tap()
    let settings = XCUIApplication(bundleIdentifier: "com.apple.Preferences")
    XCTAssertTrue(settings.wait(for: .runningForeground, timeout: 10))
    application.activate()
    XCTAssertTrue(application.links["settings"].waitForExistence(timeout: 15))
    application.links["settings"].tap()
    if !open.isHittable {
      application.swipeUp()
    }
    XCTAssertTrue(open.waitForExistence(timeout: 10), "Returning without change must retain denied recovery")
    open.tap()
    XCTAssertTrue(settings.wait(for: .runningForeground, timeout: 10))
    let location = settings.cells.containing(.staticText, identifier: "Location").firstMatch
    XCTAssertTrue(location.waitForExistence(timeout: 10), settings.debugDescription)
    location.tap()
    let always = settings.cells.containing(.staticText, identifier: "Always").firstMatch
    XCTAssertTrue(always.waitForExistence(timeout: 10), settings.debugDescription)
    always.tap()
    application.activate()
    XCTAssertTrue(application.links["settings"].waitForExistence(timeout: 15))
    application.links["settings"].tap()
    if !application.staticTexts["Always"].isHittable {
      application.swipeUp()
    }
    XCTAssertTrue(application.staticTexts["Always"].waitForExistence(timeout: 10), application.debugDescription)
    XCTAssertFalse(application.buttons["Request access"].exists)
  }

  func testBackground() {
    application.activate()
    XCUIDevice.shared.press(.home)
  }

  func testForeground() {
    application.activate()
  }

  func testFilePresentation() {
    application.activate()
    let close = application.buttons["Close"]
    XCTAssertTrue(close.waitForExistence(timeout: 10), "Native export sheet did not appear")
    close.tap()
    XCTAssertTrue(close.waitForNonExistence(timeout: 5), "Native export sheet did not dismiss")
  }
}
