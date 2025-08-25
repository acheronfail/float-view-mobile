//
//  ContentView.swift
//  float-view
//
//  Created by Callum Osmotherly on 25/8/2025.
//

import SwiftUI

struct ContentView: View {
    @ObservedObject var server: LocalServer

    init(server: LocalServer) {
        self.server = server
    }

    var body: some View {
        Group {
            if server.isRunning {
                WebView(url: URL(string: "http://localhost:\(server.port)")!).ignoresSafeArea()
            } else {
                VStack {
                    ProgressView("Starting FloatView...")
                        .progressViewStyle(CircularProgressViewStyle())
                        .foregroundColor(.white)
                    if server.isRunning {
                        Text("Server running on port \(server.port)")
                            .padding(.top)
                            .foregroundColor(.white)
                            .font(.caption)
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Color.black)
            }
        }
        .background(Color.black)
        .onDisappear {
            server.stop()
        }
    }
}

#Preview {
    ContentView(server: LocalServer.shared)
}
