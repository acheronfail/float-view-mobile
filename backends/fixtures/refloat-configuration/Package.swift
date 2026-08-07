// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "refloat-configuration",
  platforms: [.iOS(.v17), .macOS(.v14)],
  products: [
    .library(name: "RefloatConfigurationFixtures", targets: ["RefloatConfigurationFixtures"]),
  ],
  targets: [
    .target(
      name: "RefloatConfigurationFixtures",
      path: ".",
      exclude: ["Package.swift", "README.md", "adversarial.json", "routes.json"],
      resources: [.copy("manifest.json"), .copy("schemas"), .copy("values")]
    ),
  ]
)
