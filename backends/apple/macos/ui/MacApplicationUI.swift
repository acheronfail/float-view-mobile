import AppKit

@MainActor
func installApplicationIcon(on application: NSApplication) {
  guard
    let iconURL = MacResources.url(
      forResource: "FloatView",
      withExtension: "icns"
    ),
    let icon = NSImage(contentsOf: iconURL)
  else { return }
  application.applicationIconImage = icon
}

@MainActor
func installMainMenu(on application: NSApplication) {
  let applicationName = "Float View"
  let mainMenu = NSMenu()
  let applicationMenuItem = NSMenuItem(title: applicationName, action: nil, keyEquivalent: "")
  let applicationMenu = NSMenu(title: applicationName)

  applicationMenu.addItem(
    withTitle: "About \(applicationName)",
    action: #selector(NSApplication.orderFrontStandardAboutPanel(_:)),
    keyEquivalent: ""
  )
  applicationMenu.addItem(.separator())

  let servicesMenu = NSMenu(title: "Services")
  let servicesMenuItem = NSMenuItem(title: "Services", action: nil, keyEquivalent: "")
  servicesMenuItem.submenu = servicesMenu
  applicationMenu.addItem(servicesMenuItem)
  application.servicesMenu = servicesMenu

  applicationMenu.addItem(.separator())
  applicationMenu.addItem(
    withTitle: "Hide \(applicationName)",
    action: #selector(NSApplication.hide(_:)),
    keyEquivalent: "h"
  )
  let hideOthersItem = applicationMenu.addItem(
    withTitle: "Hide Others",
    action: #selector(NSApplication.hideOtherApplications(_:)),
    keyEquivalent: "h"
  )
  hideOthersItem.keyEquivalentModifierMask = [.command, .option]
  applicationMenu.addItem(
    withTitle: "Show All",
    action: #selector(NSApplication.unhideAllApplications(_:)),
    keyEquivalent: ""
  )

  applicationMenu.addItem(.separator())
  applicationMenu.addItem(
    withTitle: "Quit \(applicationName)",
    action: #selector(NSApplication.terminate(_:)),
    keyEquivalent: "q"
  )

  applicationMenuItem.submenu = applicationMenu
  mainMenu.addItem(applicationMenuItem)

  let fileMenu = NSMenu(title: "File")
  fileMenu.addItem(
    withTitle: "Close Window",
    action: #selector(NSWindow.performClose(_:)),
    keyEquivalent: "w"
  )

  let editMenu = NSMenu(title: "Edit")
  // Nil targets let AppKit route editing commands to the focused WebView or native text field.
  editMenu.addItem(withTitle: "Undo", action: Selector(("undo:")), keyEquivalent: "z")
  let redoItem = editMenu.addItem(withTitle: "Redo", action: Selector(("redo:")), keyEquivalent: "z")
  redoItem.keyEquivalentModifierMask = [.command, .shift]
  editMenu.addItem(.separator())
  editMenu.addItem(withTitle: "Cut", action: #selector(NSText.cut(_:)), keyEquivalent: "x")
  editMenu.addItem(withTitle: "Copy", action: #selector(NSText.copy(_:)), keyEquivalent: "c")
  editMenu.addItem(withTitle: "Paste", action: #selector(NSText.paste(_:)), keyEquivalent: "v")
  editMenu.addItem(withTitle: "Select All", action: #selector(NSText.selectAll(_:)), keyEquivalent: "a")

  let viewMenu = NSMenu(title: "View")
  let fullScreenItem = viewMenu.addItem(
    withTitle: "Enter Full Screen",
    action: #selector(NSWindow.toggleFullScreen(_:)),
    keyEquivalent: "f"
  )
  fullScreenItem.keyEquivalentModifierMask = [.command, .control]

  let windowMenu = NSMenu(title: "Window")
  windowMenu.addItem(
    withTitle: "Minimize",
    action: #selector(NSWindow.performMiniaturize(_:)),
    keyEquivalent: "m"
  )
  windowMenu.addItem(withTitle: "Zoom", action: #selector(NSWindow.performZoom(_:)), keyEquivalent: "")
  windowMenu.addItem(.separator())
  windowMenu.addItem(
    withTitle: "Bring All to Front",
    action: #selector(NSApplication.arrangeInFront(_:)),
    keyEquivalent: ""
  )
  application.windowsMenu = windowMenu

  let helpMenu = NSMenu(title: "Help")
  helpMenu.addItem(
    withTitle: "Float View Help",
    action: #selector(MacAppDelegate.openHelp(_:)),
    keyEquivalent: ""
  )
  application.helpMenu = helpMenu

  for menu in [fileMenu, editMenu, viewMenu, windowMenu, helpMenu] {
    let item = NSMenuItem(title: menu.title, action: nil, keyEquivalent: "")
    item.submenu = menu
    mainMenu.addItem(item)
  }
  application.mainMenu = mainMenu
}
