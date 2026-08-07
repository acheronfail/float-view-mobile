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
  dependencies: [
    .package(path: "../fixtures/refloat-configuration"),
  ],
  targets: [
    .systemLibrary(name: "CFloatViewCore"),
    .target(
      name: "FloatViewCoreBridge",
      dependencies: ["CFloatViewCore"],
      linkerSettings: [
        .unsafeFlags(["-L", coreLibraryDirectory, "-lfloat_view_native_abi"]),
      ]
    ),
    .target(
      name: "FloatViewBackend",
      dependencies: [
        "FloatViewCoreBridge",
        .product(name: "RefloatConfigurationFixtures", package: "refloat-configuration"),
      ]
    ),
    .executableTarget(
      name: "FloatViewMac",
      dependencies: ["FloatViewBackend", "FloatViewCoreBridge"],
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
