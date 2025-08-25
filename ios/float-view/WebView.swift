//
//  WebView.swift
//  float-view
//
//  Created on 25/8/2025.
//

import SwiftUI
import WebKit

struct WebView: UIViewRepresentable {
    let url: URL

    func makeUIView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()

        // Configure WebView settings for local development
        configuration.preferences.setValue(true, forKey: "allowFileAccessFromFileURLs")
        configuration.setValue(true, forKey: "allowUniversalAccessFromFileURLs")
        configuration.preferences.setValue(true, forKey: "developerExtrasEnabled")

        let webView = WKWebView(frame: .zero, configuration: configuration)
        
        // Set black background using KVO-compliant methods
        webView.setValue(false, forKey: "opaque")
        
        // Set background color via CSS injection
        let css = """
        body { 
            background-color: black !important; 
            margin: 0;
            padding: 0;
        }
        html {
            background-color: black !important;
        }
        """
        
        let script = WKUserScript(
            source: "var style = document.createElement('style'); style.innerHTML = '\(css)'; document.head.appendChild(style);",
            injectionTime: .atDocumentStart,
            forMainFrameOnly: false
        )
        
        webView.configuration.userContentController.addUserScript(script)

        return webView
    }

    func updateUIView(_ webView: WKWebView, context: Context) {
        let request = URLRequest(url: url)
        webView.load(request)
    }
}
