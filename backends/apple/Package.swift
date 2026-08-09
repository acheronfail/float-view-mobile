// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "FloatViewApple",
  platforms: [.iOS(.v17), .macOS(.v14)],
  products: [
    .library(name: "FloatViewBackend", targets: ["FloatViewBackend"]),
    .executable(name: "FloatViewMac", targets: ["FloatViewMac"]),
  ],
  dependencies: [
    .package(url: "https://github.com/swhitty/FlyingFox.git", from: "0.27.1"),
    .package(path: "../fixtures/refloat-configuration"),
  ],
  targets: [
    .systemLibrary(name: "CSQLite"),
    .target(
      name: "FloatViewBackend",
      dependencies: [
        "CSQLite",
        .product(name: "RefloatConfigurationFixtures", package: "refloat-configuration"),
        .product(name: "FlyingFox", package: "FlyingFox"),
      ]
    ),
    .executableTarget(
      name: "FloatViewMac",
      dependencies: ["FloatViewBackend"],
      path: "macos",
      resources: [.copy("Resources")]
    ),
    .testTarget(
      name: "FloatViewBackendTests",
      dependencies: ["CSQLite", "FloatViewBackend"]
    ),
  ]
)
