// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "FloatViewApple",
  platforms: [
    .iOS(.v17),
    .macOS(.v14),
  ],
  products: [
    .library(name: "FloatViewCore", targets: ["FloatViewCore"]),
    .library(name: "FloatViewProtocol", targets: ["FloatViewProtocol"]),
    .library(name: "FloatViewLogging", targets: ["FloatViewLogging"]),
    .library(name: "FloatViewHost", targets: ["FloatViewHost"]),
    .executable(name: "FloatViewMac", targets: ["FloatViewMac"]),
  ],
  targets: [
    .systemLibrary(name: "CSQLite"),
    .target(name: "FloatViewProtocol"),
    .target(name: "FloatViewLogging", dependencies: ["FloatViewCore", "CSQLite"]),
    .target(name: "FloatViewHost"),
    .target(name: "FloatViewCore"),
    .executableTarget(
      name: "FloatViewMac",
      dependencies: ["FloatViewCore", "FloatViewHost"],
      resources: [.copy("Resources")]
    ),
    .testTarget(
      name: "FloatViewCoreTests",
      dependencies: ["FloatViewCore", "FloatViewLogging", "FloatViewProtocol"]
    ),
  ]
)
