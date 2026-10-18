import AppKit
import FloatViewBackend
@testable import FloatViewMac
import Testing

@Suite("macOS native startup appearance", .serialized)
@MainActor
struct MacThemeTests {
  @Test("explicit theme updates existing and recreated windows; system removes the override")
  func windowAppearance() {
    let app = NSApplication.shared
    let original = app.appearance
    defer { app.appearance = original }
    let window = NSWindow(
      contentRect: NSRect(x: 0, y: 0, width: 200, height: 200),
      styleMask: [.titled], backing: .buffered, defer: false
    )
    window.isReleasedWhenClosed = false
    defer { window.close() }
    MacTheme.apply(.dark, to: app)
    #expect(window.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua)
    MacTheme.apply(.light, to: app)
    #expect(window.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .aqua)
    for _ in 0 ..< 10 {
      let replacement = NSWindow(
        contentRect: NSRect(x: 0, y: 0, width: 200, height: 200),
        styleMask: [.titled], backing: .buffered, defer: false
      )
      replacement.isReleasedWhenClosed = false
      #expect(replacement.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .aqua)
      replacement.close()
    }
    MacTheme.apply(.system, to: app)
    #expect(app.appearance == nil)
    #expect(window.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == NSAppearance.currentDrawing()
      .bestMatch(from: [
        .aqua,
        .darkAqua,
      ]))
  }

  @Test("loading artwork repaints in the same window when appearance changes")
  func loadingArtwork() throws {
    let app = NSApplication.shared
    let original = app.appearance
    defer { app.appearance = original }
    let window = NSWindow(
      contentRect: NSRect(x: 0, y: 0, width: 400, height: 300),
      styleMask: [.titled], backing: .buffered, defer: false
    )
    window.isReleasedWhenClosed = false
    defer { window.close() }
    let loading = MacLoadingView(frame: NSRect(x: 0, y: 0, width: 400, height: 300))
    window.contentView = loading
    let stack = try #require(loading.subviews.first as? NSStackView)
    let symbol = try #require(stack.arrangedSubviews.first as? NSImageView)
    #expect(symbol.image?.isTemplate == true)

    var captures: [Data] = []
    for (preference, expected) in [
      (ThemePreference.light, [15.0, 118.0, 110.0]),
      (.dark, [162.0, 212.0, 216.0]),
      (.light, [15.0, 118.0, 110.0]),
    ] {
      MacTheme.apply(preference, to: app)
      loading.layoutSubtreeIfNeeded()
      let tint = try #require(symbol.contentTintColor?.usingColorSpace(.sRGB))
      for (component, value) in zip([tint.redComponent, tint.greenComponent, tint.blueComponent], expected) {
        #expect(abs(component * 255 - value) < 0.01)
      }
      let bitmap = try #require(symbol.bitmapImageRepForCachingDisplay(in: symbol.bounds))
      symbol.cacheDisplay(in: symbol.bounds, to: bitmap)
      try captures.append(#require(bitmap.representation(using: .png, properties: [:])))
    }
    // Check actual image output too: changing theme must repaint the template,
    // and switching back must restore the same artwork without recreating it.
    #expect(captures[0] != captures[1])
    #expect(captures[0] == captures[2])
  }

  @Test("canvas resolves in the appearance of each window")
  func canvas() throws {
    for (name, expected) in [(NSAppearance.Name.aqua, [248.0, 250.0, 252.0]), (.darkAqua, [2.0, 6.0, 23.0])] {
      let appearance = try #require(NSAppearance(named: name))
      var color: NSColor?
      appearance.performAsCurrentDrawingAppearance {
        color = MacTheme.canvasColor.usingColorSpace(.sRGB)
      }
      let resolved = try #require(color)
      #expect(abs(resolved.redComponent * 255 - expected[0]) < 0.01)
      #expect(abs(resolved.greenComponent * 255 - expected[1]) < 0.01)
      #expect(abs(resolved.blueComponent * 255 - expected[2]) < 0.01)
    }
  }
}
