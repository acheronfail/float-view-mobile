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
  application.mainMenu = mainMenu
}
