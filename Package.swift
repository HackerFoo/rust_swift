// swift-tools-version: 6.1
import PackageDescription

let package = Package(
    name: "rust_swift",
    platforms: [
      .macOS(.v15),
      .iOS(.v18),
    ],
    products: [
      .library(name: "rust_swift", type: .static, targets: ["rust_swift"])
    ],
    dependencies: [],
    targets: [
        .target(
            name: "rust_swift",
            path: "src/swift",
        ),
    ],
    swiftLanguageModes: [.v5]
)
