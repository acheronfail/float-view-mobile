import Foundation

/// Route- and family-aware owner for every telemetry poll command.
enum RefloatPolling {
  static func layoutRequest(
    protocolCodec: any VescProtocolPort,
    route: ControllerRoute
  ) throws -> Data {
    try protocolCodec.refloatTelemetryIDsRequest(route: route)
  }

  static func telemetryRequest(
    protocolCodec: any VescProtocolPort,
    route: ControllerRoute,
    family: RefloatTelemetryFamily
  ) throws -> Data {
    switch family {
    case .legacyFixed:
      try protocolCodec.refloatLegacyTelemetryRequest(route: route)
    case .dynamic:
      try protocolCodec.refloatTelemetryRequest(route: route)
    }
  }

  static func compactRequest(
    protocolCodec: any VescProtocolPort,
    route: ControllerRoute
  ) throws -> Data {
    try protocolCodec.refloatCompactTelemetryRequest(route: route)
  }
}
