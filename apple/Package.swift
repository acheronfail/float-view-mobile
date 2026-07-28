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
  dependencies: [
    .package(url: "https://github.com/swhitty/FlyingFox.git", from: "0.27.1"),
  ],
  targets: [
    .systemLibrary(name: "CSQLite"),
    .target(name: "FloatViewProtocol", dependencies: ["FloatViewCore"]),
    .target(name: "FloatViewLogging", dependencies: ["FloatViewCore", "CSQLite"]),
    .target(name: "FloatViewBluetooth", dependencies: ["FloatViewCore"]),
    .target(
      name: "FloatViewHost",
      dependencies: ["FloatViewCore", .product(name: "FlyingFox", package: "FlyingFox")]
    ),
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
      dependencies: [
        "CSQLite", "FloatViewCore", "FloatViewHost", "FloatViewLogging", "FloatViewProtocol",
      ]
    ),
  ]
)
