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
    .library(name: "FloatViewBluetooth", targets: ["FloatViewBluetooth"]),
    .library(name: "FloatViewHost", targets: ["FloatViewHost"]),
    .executable(name: "FloatViewMac", targets: ["FloatViewMac"]),
  ],
  targets: [
    .systemLibrary(name: "CSQLite"),
    .target(name: "FloatViewProtocol", dependencies: ["FloatViewCore"]),
    .target(name: "FloatViewLogging", dependencies: ["FloatViewCore", "CSQLite"]),
    .target(name: "FloatViewBluetooth", dependencies: ["FloatViewCore"]),
    .target(name: "FloatViewHost", dependencies: ["FloatViewCore"]),
    .target(name: "FloatViewCore"),
    .executableTarget(
      name: "FloatViewMac",
      dependencies: [
        "FloatViewBluetooth",
        "FloatViewCore",
        "FloatViewHost",
        "FloatViewLogging",
        "FloatViewProtocol",
      ],
      resources: [.copy("Resources")]
    ),
    .testTarget(
      name: "FloatViewCoreTests",
      dependencies: ["FloatViewCore", "FloatViewHost", "FloatViewLogging", "FloatViewProtocol"]
    ),
  ]
)
