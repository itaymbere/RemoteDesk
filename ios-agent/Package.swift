// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "AgRemoteDeskIOS",
    platforms: [.iOS(.v16)],
    dependencies: [
        .package(url: "https://github.com/stasel/WebRTC.git", from: "125.0.0"),
    ],
    targets: [
        .target(
            name: "AgRemoteDeskIOS",
            dependencies: [
                .product(name: "WebRTC", package: "WebRTC"),
            ],
            path: "App"
        ),
        .target(
            name: "BroadcastExtension",
            dependencies: [
                .product(name: "WebRTC", package: "WebRTC"),
            ],
            path: "BroadcastExtension"
        ),
    ]
)
