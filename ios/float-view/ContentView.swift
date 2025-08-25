//
//  ContentView.swift
//  float-view
//
//  Created by Callum Osmotherly on 25/8/2025.
//

import SwiftUI

struct ContentView: View {
    @ObservedObject var server: LocalServer
    @State private var serverURL: URL?

    // Default initializer for backwards compatibility
    init() {
        self.server = LocalServer()
    }
    
    // New initializer that accepts a server instance
    init(server: LocalServer) {
        self.server = server
        print(self.server.isRunning)
    }

    var body: some View {
        Group {
            if let url = serverURL {
                WebView(url: url)
                    .ignoresSafeArea()
            } else {
                VStack {
                    ProgressView("Starting local server...")
                        .progressViewStyle(CircularProgressViewStyle())
                        .foregroundColor(.white)
                    Text("Please wait...")
                        .padding(.top)
                        .foregroundColor(.white)
                    if server.isRunning {
                        Text("Server running on port \(server.port)")
                            .padding(.top)
                            .foregroundColor(.white)
                            .font(.caption)
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Color.black) // Black background for loading screen
            }
        }
        .background(Color.black) // Black background for the entire view
        .onChange(of: server.isRunning) {
            if server.isRunning && server.port > 0 {
                // Give the server a moment to fully initialize
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
                    serverURL = URL(string: "http://localhost:\(server.port)")
                    print("WebView will load URL: \(serverURL?.absoluteString ?? "nil")")
                }
            }
        }
        .onDisappear {
            server.stop()
        }
    }
}

#Preview {
    ContentView()
}
