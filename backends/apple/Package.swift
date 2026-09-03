// swift-tools-version: 6.0
import Foundation
import PackageDescription

let packageRoot = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path
let rustLibraryPath = "\(packageRoot)/../core/target/apple/macos"

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
    .systemLibrary(name: "CFloatViewApplicationRuntime"),
    .target(
      name: "FloatViewBackend",
      dependencies: [
        "CSQLite",
        "CFloatViewApplicationRuntime",
        .product(name: "RefloatConfigurationFixtures", package: "refloat-configuration"),
        .product(name: "FlyingFox", package: "FlyingFox"),
      ],
      linkerSettings: [
        .unsafeFlags(["-L", rustLibraryPath]),
        .linkedLibrary("float_view_application_runtime_apple_ffi"),
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
      dependencies: [
        "CSQLite", "CFloatViewApplicationRuntime",
        "FloatViewBackend",
      ]
    ),
  ]
)
