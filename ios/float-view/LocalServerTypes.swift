import CommonCrypto
import Foundation
import Network

// MARK: - HTTP Context

class HttpContext {
    let connection: NWConnection
    let method: String
    let path: String
    let headers: [String: String]

    init(connection: NWConnection, method: String, path: String, headers: [String: String]) {
        self.connection = connection
        self.method = method
        self.path = path
        self.headers = headers
    }

    func sendResponse(status: String, contentType: String, body: String) {
        let bodyData = body.data(using: .utf8) ?? Data()
        let response = """
            HTTP/1.1 \(status)\r
            Content-Type: \(contentType); charset=utf-8\r
            Content-Length: \(bodyData.count)\r
            Connection: close\r
            \r
            \(body)
            """

        guard let responseData = response.data(using: .utf8) else {
            connection.cancel()
            return
        }

        connection.send(
            content: responseData,
            completion: .contentProcessed { error in
                if let error = error {
                    print("Error sending response: \(error)")
                }
                // Close connection after response is sent
                self.connection.cancel()
            }
        )
    }
}

// MARK: - WebSocket Frame

struct WebSocketFrame {
    let opcode: Int
    let payload: String
}

// Extension to add SHA-1 hashing
extension Data {
    var sha1: Data {
        var digest = [UInt8](repeating: 0, count: Int(CC_SHA1_DIGEST_LENGTH))
        _ = self.withUnsafeBytes { bytes in
            CC_SHA1(bytes.bindMemory(to: UInt8.self).baseAddress, CC_LONG(self.count), &digest)
        }
        return Data(digest)
    }
}

// MARK: - WebSocket Types

struct WebSocketCommand: Codable {
    let cmd: String
    let data: [String: Any]

    init(cmd: String, data: [String: Any] = [:]) {
        self.cmd = cmd
        self.data = data
    }

    enum CodingKeys: String, CodingKey {
        case cmd, data
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        cmd = try container.decode(String.self, forKey: .cmd)

        // Decode data as a flexible dictionary
        if let dataContainer = try? container.nestedContainer(keyedBy: AnyCodingKey.self, forKey: .data) {
            var decodedData: [String: Any] = [:]
            for key in dataContainer.allKeys {
                if let stringValue = try? dataContainer.decode(String.self, forKey: key) {
                    decodedData[key.stringValue] = stringValue
                } else if let intValue = try? dataContainer.decode(Int.self, forKey: key) {
                    decodedData[key.stringValue] = intValue
                } else if let doubleValue = try? dataContainer.decode(Double.self, forKey: key) {
                    decodedData[key.stringValue] = doubleValue
                } else if let boolValue = try? dataContainer.decode(Bool.self, forKey: key) {
                    decodedData[key.stringValue] = boolValue
                }
            }
            data = decodedData
        } else {
            data = [:]
        }
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(cmd, forKey: .cmd)
        try container.encode(AnyCodableDict(data), forKey: .data)
    }
}



// Helper types for flexible JSON encoding/decoding
private struct AnyCodingKey: CodingKey {
    let stringValue: String
    let intValue: Int?

    init?(stringValue: String) {
        self.stringValue = stringValue
        self.intValue = nil
    }

    init?(intValue: Int) {
        self.stringValue = String(intValue)
        self.intValue = intValue
    }
}

private struct AnyCodableDict: Codable {
    let dictionary: [String: Any]

    init(_ dictionary: [String: Any]) {
        self.dictionary = dictionary
    }

    init(from decoder: Decoder) throws {
        // This won't be used in our case, but needed for Codable conformance
        dictionary = [:]
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: AnyCodingKey.self)
        for (key, value) in dictionary {
            guard let codingKey = AnyCodingKey(stringValue: key) else { continue }

            if let stringValue = value as? String {
                try container.encode(stringValue, forKey: codingKey)
            } else if let intValue = value as? Int {
                try container.encode(intValue, forKey: codingKey)
            } else if let doubleValue = value as? Double {
                try container.encode(doubleValue, forKey: codingKey)
            } else if let boolValue = value as? Bool {
                try container.encode(boolValue, forKey: codingKey)
            }
        }
    }
}

// MARK: - Handler Types

typealias WsHandler = ([String: Any]) -> Void

// MARK: - WebSocket Client

class WebSocketClient {
    let connection: NWConnection
    let id: String
    private var handlers: [String: WsHandler] = [:]

    init(connection: NWConnection, id: String) {
        self.connection = connection
        self.id = id
    }

    func addHandler(_ commandName: String, handler: @escaping WsHandler) {
        handlers[commandName] = handler
    }

    func getHandler(_ commandName: String) -> WsHandler? {
        return handlers[commandName]
    }
}

import Network
