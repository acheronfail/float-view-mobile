import Foundation

@main
struct ContractSmoke {
  static func main() throws {
    guard CommandLine.arguments.count == 2 else {
      throw SmokeError.missingExamplePath
    }

    let data = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))
    let decoded = try JSONDecoder().decode(HelloResponse.self, from: data)
    guard decoded.result.api == ApiVersion(major: 1, minor: 0) else {
      throw SmokeError.unexpectedVersion
    }
    guard decoded.result.backend.platform == .mock else {
      throw SmokeError.unexpectedPlatform
    }
    guard decoded.result.snapshot.bluetooth.radio == .poweredOn else {
      throw SmokeError.unexpectedBluetoothState
    }

    let roundTripped = try JSONDecoder().decode(
      HelloResponse.self,
      from: JSONEncoder().encode(decoded)
    )
    guard roundTripped == decoded else {
      throw SmokeError.roundTripMismatch
    }
  }
}

enum SmokeError: Error {
  case missingExamplePath
  case unexpectedVersion
  case unexpectedPlatform
  case unexpectedBluetoothState
  case roundTripMismatch
}
