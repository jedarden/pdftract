// swift-tools-version: 5.10
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "pdftract-swift",
    platforms: [
        .macOS(.v13),
        .linux
    ],
    products: [
        .library(
            name: "Pdftract",
            targets: ["Pdftract"])
    ],
    dependencies: [
        // No external dependencies - uses only Foundation for Process/JSONDecoder
    ],
    targets: [
        .target(
            name: "Pdftract",
            dependencies: [],
            path: "Sources/Pdftract"),
        .testTarget(
            name: "PdftractTests",
            dependencies: ["Pdftract"],
            path: "Tests/PdftractTests")
    ],
    swiftLanguageVersions: [.v5]
)
