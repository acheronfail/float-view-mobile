// swift-tools-version:5.2
import PackageDescription

let packageName = "float-view"
let package = Package(
  name: packageName,
  platforms: [.iOS("14.0")],
  products: [
    .library(name: packageName, targets: [packageName])
  ],
  targets: [
    .target(
      name: packageName,
      path: packageName
    )
  ]
)