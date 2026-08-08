// swift-tools-version: 6.0
import Foundation
import PackageDescription

let coreLibraryDirectory = ProcessInfo.processInfo.environment["FLOAT_VIEW_CORE_LIBRARY_DIR"]
  ?? "../core/target/debug"

let package = Package(
  name: "FloatViewApple",
  platforms: [.iOS(.v17), .macOS(.v14)],
  products: [
    .library(name: "FloatViewBackend", targets: ["FloatViewBackend"]),
    .library(name: "FloatViewCoreBridge", targets: ["FloatViewCoreBridge"]),
    .executable(name: "FloatViewMac", targets: ["FloatViewMac"]),
  ],
  dependencies: [],
  targets: [
    .systemLibrary(name: "CFloatViewCore", path: "common/CFloatViewCore"),
    .target(
      name: "FloatViewCoreBridge",
      dependencies: ["CFloatViewCore"],
      path: "common/FloatViewCoreBridge",
      linkerSettings: [
        .unsafeFlags(["-L", coreLibraryDirectory, "-lhost_c_abi"]),
      ]
    ),
    .target(
      name: "FloatViewBackend",
      dependencies: ["FloatViewCoreBridge"],
      path: "common/FloatViewBackend"
    ),
    .executableTarget(
      name: "FloatViewMac",
      dependencies: ["FloatViewBackend", "FloatViewCoreBridge"],
      path: "macos",
      exclude: ["README.md"],
      resources: [.copy("Resources")]
    ),
    .testTarget(
      name: "FloatViewBackendTests",
      dependencies: ["FloatViewBackend"]
    ),
    .testTarget(
      name: "FloatViewCoreBridgeTests",
      dependencies: ["FloatViewCoreBridge"]
    ),
  ]
)
