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
  dependencies: [],
  targets: [
    .systemLibrary(name: "CFloatViewApplicationRuntime"),
    .target(
      name: "FloatViewBackend",
      dependencies: [
        "CFloatViewApplicationRuntime",
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
        "CFloatViewApplicationRuntime",
        "FloatViewBackend",
      ]
    ),
  ]
)
