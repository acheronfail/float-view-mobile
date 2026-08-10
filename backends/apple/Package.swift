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
    .systemLibrary(name: "CVescBmsValues"),
    .systemLibrary(name: "CVescControllerData"),
    .systemLibrary(name: "CVescCustomConfiguration"),
    .systemLibrary(name: "CVescFrame"),
    .systemLibrary(name: "CVescFirmwareIdentity"),
    .systemLibrary(name: "CVescRefloatControl"),
    .target(
      name: "FloatViewBackend",
      dependencies: [
        "CSQLite",
        "CVescBmsValues",
        "CVescControllerData",
        "CVescCustomConfiguration",
        "CVescFrame",
        "CVescFirmwareIdentity",
        "CVescRefloatControl",
        .product(name: "RefloatConfigurationFixtures", package: "refloat-configuration"),
        .product(name: "FlyingFox", package: "FlyingFox"),
      ],
      linkerSettings: [
        .unsafeFlags(["-L", rustLibraryPath]),
        .linkedLibrary("float_view_vesc_frame_apple_ffi"),
        .linkedLibrary("float_view_vesc_firmware_identity_apple_ffi"),
        .linkedLibrary("float_view_vesc_custom_configuration_apple_ffi"),
        .linkedLibrary("float_view_vesc_bms_values_apple_ffi"),
        .linkedLibrary("float_view_vesc_controller_data_apple_ffi"),
        .linkedLibrary("float_view_refloat_control_apple_ffi"),
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
        "CSQLite", "CVescBmsValues", "CVescControllerData", "CVescCustomConfiguration", "CVescFrame",
        "CVescFirmwareIdentity",
        "CVescRefloatControl",
        "FloatViewBackend",
      ]
    ),
  ]
)
