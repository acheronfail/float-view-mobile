//
//  float_viewApp.swift
//  float-view
//
//  Created by Callum Osmotherly on 25/8/2025.
//

import SwiftUI

@main
struct FloatViewApp: App {
    init() {
        print("App initializing - starting server immediately...")
        // Start server synchronously right now, before any views are created
        LocalServer.shared.start()
    }
    
    var body: some Scene {
        WindowGroup {
            ContentView(server: LocalServer.shared)
                .preferredColorScheme(.dark) // Force dark mode for black backgrounds
        }
    }
}
