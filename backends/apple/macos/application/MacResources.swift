import Foundation

enum MacResources {
  static func url(forResource name: String, withExtension extensionName: String) -> URL? {
    // Packaged apps must resolve their own resources, not SwiftPM's build path.
    if Bundle.main.bundleURL.pathExtension == "app" {
      return Bundle.main.url(forResource: name, withExtension: extensionName)
    }
    return Bundle.module.url(forResource: name, withExtension: extensionName, subdirectory: "Resources")
  }
}
