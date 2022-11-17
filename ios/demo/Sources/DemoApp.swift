//
//  demoApp.swift
//  demo
//
//  Created by s1rius on 2022/6/8.
//

import SwiftUI
import EZLog

@main
struct DemoApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
    
    public init() {
        pthread_setname_np("main")    
    }
}

extension URL {
    static var documents: URL {
        return FileManager
            .default
            .urls(for: .documentDirectory, in: .userDomainMask)[0]
    }
}
