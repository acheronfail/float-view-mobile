import Foundation

public enum RefloatConfigurationFixtureResources {
  public static func data(_ relativePath: String) -> Data? {
    guard relativePath.count <= 160,
          !relativePath.contains(".."),
          let root = Bundle.module.resourceURL
    else { return nil }
    return try? Data(contentsOf: root.appending(path: relativePath), options: [.mappedIfSafe])
  }
}
