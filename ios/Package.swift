// swift-tools-version:5.3
 
import PackageDescription
 
let package = Package(
    name: "EZLog",
    platforms: [
        .iOS(.v12),
    ],
    products: [
        .library(
            name: "EZLog",
            targets: ["EZLog"]),
    ],
    targets: [
        .binaryTarget(
            name: "EZLogFramework",
            path: "framework/EZLogFramework.xcframework"
        ),
        .target(
            name: "EZLog",
            dependencies: ["EZLogFramework"],
            
            path: "EZLog/Sources",
            publicHeadersPath: "."
            
        )
    ]
)
