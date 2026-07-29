import FlyingFox
import Foundation
import Security

public enum PlatformLoopbackPort {
  public static let macOS: UInt16 = 11_010
  public static let iOS: UInt16 = 11_011
}

public func loopbackStartupFailure(port: UInt16, error: any Error) -> String {
  "failed to bind Float View backend to 127.0.0.1:\(port): \(error)"
}

public final class LoopbackServer: @unchecked Sendable {
  public let token: String
  public private(set) var port: UInt16 = 0
  public var origin: String { "http://127.0.0.1:\(port)" }

  private let html: Data
  private let requestedPort: UInt16
  private let applicationService: ApplicationService
  private let exportPresenter: (@Sendable (SessionExportRecord) -> Bool)?
  fileprivate let events: EventHub
  private let revisionLock = NSLock()
  private var revision = 0
  private var server: HTTPServer?
  private var serverTask: Task<Void, Never>?

  public init(
    html: Data,
    port: UInt16 = 0,
    applicationService: ApplicationService,
    exportPresenter: (@Sendable (SessionExportRecord) -> Bool)? = nil,
    events: EventHub
  ) {
    self.html = html
    requestedPort = port
    self.applicationService = applicationService
    self.exportPresenter = exportPresenter
    self.events = events
    token = Self.randomToken()
  }

  public func start() throws {
    guard server == nil else { return }
    let server = HTTPServer(address: try sockaddr_in.inet(ip4: "127.0.0.1", port: requestedPort))
    let result = StartResult()
    let ready = DispatchSemaphore(value: 0)
    serverTask = Task.detached { [weak self] in
      guard let self else { return }
      await server.appendRoute("GET /") { [weak self] _ in
        guard let self else { return HTTPResponse(statusCode: .internalServerError) }
        return HTTPResponse(
          statusCode: .ok,
          headers: [.contentType: "text/html", .cacheControl: "no-store"],
          body: self.html
        )
      }
      await server.appendRoute("GET /api/bootstrap") { [weak self] _ in
        guard let self else { return HTTPResponse(statusCode: .internalServerError) }
        let body = try JSONSerialization.data(withJSONObject: [
          "webSocketUrl": "\(self.origin.replacingOccurrences(of: "http", with: "ws"))/api/ws",
          "sessionToken": self.token,
        ])
        return HTTPResponse(
          statusCode: .ok,
          headers: [.contentType: "application/json", .cacheControl: "no-store"],
          body: body
        )
      }
      await server.appendRoute("GET /api/ws") { [weak self] request in
        guard let self,
              request.query["token"] == self.token,
              request.headers[HTTPHeader("Origin")] == self.origin
        else { return HTTPResponse(statusCode: .forbidden) }
        return try await WebSocketHTTPHandler(
          handler: MessageFrameWSHandler(
            handler: BackendMessageHandler(server: self),
            frameSize: 65_536
          )
        ).handleRequest(request)
      }
      let running = Task { try await server.run() }
      do {
        try await server.waitUntilListening()
        guard let address = await server.listeningAddress else {
          throw LoopbackServerError.failedToBind
        }
        switch address {
        case .ip4(_, let value), .ip6(_, let value): self.port = value
        case .unix: throw LoopbackServerError.failedToBind
        }
        result.set(.success(self.port))
        ready.signal()
        try await running.value
      } catch {
        result.set(.failure(error))
        ready.signal()
        running.cancel()
      }
    }
    self.server = server
    ready.wait()
    _ = try result.get().get()
  }

  public func stop() {
    let running = server
    serverTask?.cancel()
    serverTask = nil
    server = nil
    port = 0
    if let running {
      let stopped = DispatchSemaphore(value: 0)
      Task {
        await running.stop()
        stopped.signal()
      }
      stopped.wait()
    }
  }

  fileprivate func handle(_ request: [String: Any]) -> Data {
    let id = request["id"] as? Int
    guard let id, id >= 0,
          let method = request["method"] as? String,
          request["params"] is [String: Any]
    else {
      return failure(id ?? 0, .invalidRequest, "Request id, method, and params are required")
    }
    let params = request["params"] as! [String: Any]
    func success(_ result: Any) -> Data {
      Self.json(["id": id, "result": result])
    }

    do {
      switch method {
      case "state.get":
        return success(snapshot())
      case "bluetooth.requestPermission":
        return success(bluetoothPayload(applicationService.requestBluetoothPermission()))
      case "bluetooth.scan":
        let scanID = try applicationService.startDiscovery()
        let requestedTimeoutMs = (params["timeoutMs"] as? NSNumber)?.intValue ?? 5_000
        let timeoutMs = min(max(requestedTimeoutMs, 1), 60_000)
        Task.detached { [weak applicationService] in
          try? await Task.sleep(for: .milliseconds(timeoutMs))
          try? applicationService?.stopDiscovery(scanID: scanID)
        }
        return success(["accepted": true])
      case "device.connect":
        guard let deviceID = params["deviceId"] as? String, !deviceID.isEmpty else {
          return failure(id, .invalidRequest, "deviceId is required")
        }
        if applicationService.activeConnectionSnapshot() != nil {
          return failure(id, .invalidState, "A board is already active")
        }
        try applicationService.connect(DeviceID(deviceID))
        return success(["accepted": true])
      case "device.disconnect":
        try applicationService.disconnect()
        return success(["accepted": true])
      case "savedDevice.update":
        guard let savedID = params["savedDeviceId"] as? String,
              let expected = params["expectedRevision"] as? Int,
              params.keys.contains("customDisplayName"),
              params["customDisplayName"] is String || params["customDisplayName"] is NSNull
        else { return failure(id, .invalidRequest, "Valid saved-device update is required") }
        return success(savedDevicePayload(try applicationService.updateSavedDevice(
          savedDeviceID: savedID,
          expectedRevision: expected,
          customDisplayName: params["customDisplayName"] as? String
        )))
      case "settings.update":
        guard let expected = params["expectedRevision"] as? Int,
              let patch = params["patch"] as? [String: Any]
        else { return failure(id, .invalidRequest, "Revision and patch are required") }
        return success(settingsPayload(try applicationService.updateSettings(
          expectedRevision: expected,
          patchJSON: try Self.jsonString(patch)
        )))
      case "boardProfile.get":
        guard let savedID = params["savedDeviceId"] as? String else {
          return failure(id, .invalidRequest, "savedDeviceId is required")
        }
        return success(try boardProfilePayload(
          applicationService.boardProfile(savedDeviceID: savedID)
        ))
      case "boardProfile.update":
        guard let savedID = params["savedDeviceId"] as? String,
              let expected = params["expectedRevision"] as? Int,
              let patch = params["patch"] as? [String: Any]
        else { return failure(id, .invalidRequest, "Valid profile update is required") }
        return success(try boardProfilePayload(applicationService.updateBoardProfile(
          savedDeviceID: savedID,
          expectedRevision: expected,
          patchJSON: try Self.jsonString(patch)
        )))
      case "session.get":
        guard let sessionID = params["sessionId"] as? String else {
          return failure(id, .invalidRequest, "sessionId is required")
        }
        let summary = try applicationService.session(sessionID: sessionID)
        let maxPoints = min(max(params["maxPoints"] as? Int ?? 400, 1), 2_000)
        let points = try applicationService.querySessionSeries(
          sessionID: sessionID,
          startElapsedMs: 0,
          endElapsedMs: summary.totalElapsedMs,
          fields: ["packVoltageV", "dutyCycle", "speedKph"],
          maxPoints: maxPoints
        )
        return success([
          "summary": sessionPayload(summary),
          "series": points.map(seriesPayload),
        ])
      case "session.getDiagnostics":
        guard let sessionID = params["sessionId"] as? String,
              let center = (params["centerElapsedMs"] as? NSNumber)?.int64Value,
              let before = (params["beforeMs"] as? NSNumber)?.int64Value,
              let after = (params["afterMs"] as? NSNumber)?.int64Value
        else { return failure(id, .invalidRequest, "Valid diagnostic window is required") }
        return success(try applicationService.diagnosticWindow(
          sessionID: sessionID,
          centerElapsedMs: center,
          beforeMs: before,
          afterMs: after,
          fields: [
            "boardState", "packVoltageV", "dutyCycle", "controllerTempC", "motorTempC",
            "footpadState", "motorCurrentA", "batteryCurrentA", "erpm", "speedKph",
          ]
        ).map(seriesPayload))
      case "session.export":
        guard let sessionID = params["sessionId"] as? String,
              let format = params["format"] as? String
        else { return failure(id, .invalidRequest, "sessionId and format are required") }
        let value = try applicationService.exportSession(sessionID: sessionID, format: format)
        return success([
          "fileName": value.fileName,
          "delivery": exportPresenter?(value) == true ? "presented" : "staged",
        ])
      case "session.delete":
        guard let sessionID = params["sessionId"] as? String else {
          return failure(id, .invalidRequest, "sessionId is required")
        }
        try applicationService.deleteSession(sessionID: sessionID)
        return success(["accepted": true])
      case "sessionGroup.unlink":
        guard let groupID = params["sessionGroupId"] as? String else {
          return failure(id, .invalidRequest, "sessionGroupId is required")
        }
        try applicationService.unlinkSessionGroup(sessionGroupID: groupID)
        return success(["accepted": true])
      default:
        return failure(id, .invalidRequest, "Unknown method")
      }
    } catch let error as ApplicationError {
      return applicationFailure(id, error)
    } catch let error as ProductStoreError {
      return productFailure(id, error)
    } catch let error as SessionStoreError {
      return sessionFailure(id, error)
    } catch {
      return failure(id, .internal, "Backend operation failed")
    }
  }

  fileprivate func stateChanged() -> String {
    revisionLock.withLock { revision += 1 }
    return String(decoding: Self.json([
      "event": "state.changed",
      "data": snapshot(),
    ]), as: UTF8.self)
  }

  private func snapshot() -> [String: Any] {
    let discovery = applicationService.discoverySnapshot()
    let active = applicationService.activeConnectionSnapshot()
    let firmware = applicationService.firmwareSnapshot()
    let packageVersion = applicationService.refloatPackageVersionSnapshot()
    let sessions = try? applicationService.sessions(SessionListQuery(
      cursor: nil, limit: 50, deviceHistoryID: nil,
      startedAfter: nil, startedBefore: nil,
      minimumTotalElapsedMs: nil, maximumTotalElapsedMs: nil,
      minimumRidingElapsedMs: nil, maximumRidingElapsedMs: nil,
      minimumDistanceM: nil, maximumDistanceM: nil,
      state: nil, integrity: nil, sortField: "startedAt", sortDescending: true
    ))
    var connection: Any = NSNull()
    if let active {
      var value = connectionPayload(active)
      if let firmware, let packageVersion {
        value["firmware"] = [
          "vescVersion": "\(firmware.major).\(String(format: "%02d", firmware.minor))",
          "packageName": "refloat",
          "packageVersion": packageVersion,
          "hardwareName": firmware.hardwareName.map { $0 as Any } ?? NSNull(),
          "supported": (packageVersion == "1.2" || packageVersion.hasPrefix("1.2.")) &&
            (firmware.major > 6 || (firmware.major == 6 && firmware.minor >= 2)),
        ]
      } else {
        value["firmware"] = NSNull()
      }
      value["savedDeviceId"] = active.savedDevice?.savedDeviceID ?? NSNull()
      value["displayName"] = active.savedDevice?.displayName ?? active.name ??
        active.deviceID.value
      value["activeSession"] = active.activeSession.map(sessionPayload) ?? NSNull()
      value["latestSample"] = active.latestSample.map(liveSamplePayload) ?? NSNull()
      connection = value
    }
    return [
      "revision": revisionLock.withLock { revision },
      "bluetooth": bluetoothPayload(discovery.bluetoothState, scanning: discovery.activeScanID != nil),
      "discoveredDevices": Array(discovery.knownDevices.prefix(50)).map(devicePayload),
      "savedDevices": Array((try? applicationService.savedDevices())?.prefix(50) ?? []).map(savedDevicePayload),
      "activeConnection": connection,
      "recentSessions": sessions?.items.map(sessionPayload) ?? [],
      "sessionGroups": Array((try? applicationService.sessionGroups())?.prefix(50) ?? []).map(sessionGroupPayload),
      "settings": (try? applicationService.settings()).map(settingsPayload) ?? [:],
    ]
  }

  private func bluetoothPayload(_ value: BluetoothState, scanning: Bool = false) -> [String: Any] {
    [
      "radio": value.radio.rawValue,
      "scanPermission": value.scanPermission.rawValue,
      "connectPermission": value.connectPermission.rawValue,
      "scanning": scanning,
    ]
  }

  private func devicePayload(_ value: KnownDevice) -> [String: Any] {
    [
      "deviceId": value.deviceID.value,
      "name": value.name ?? "",
      "state": "discovered",
      "lastSeenAt": ISO8601DateFormatter().string(from: value.lastSeenAt),
    ]
  }

  private func connectionPayload(_ value: ActiveConnectionSnapshot) -> [String: Any] {
    [
      "deviceId": value.deviceID.value,
      "name": value.savedDevice?.displayName ?? value.name ?? value.deviceID.value,
      "state": deviceState(value.state),
    ]
  }

  private func savedDevicePayload(_ value: SavedDeviceRecord) -> [String: Any] {
    [
      "savedDeviceId": value.savedDeviceID,
      "deviceHistoryId": value.deviceHistoryID,
      "revision": value.revision,
      "displayName": value.displayName,
      "customDisplayName": value.customDisplayName ?? NSNull(),
      "advertisedName": value.advertisedName ?? NSNull(),
      "bluetoothIdentifier": value.bluetoothIdentifier,
      "lastSeenAt": value.lastSeenAt.map(ISO8601DateFormatter().string) ?? NSNull(),
      "lastConnectedAt": value.lastConnectedAt.map(ISO8601DateFormatter().string) ?? NSNull(),
      "profileComplete": value.profileComplete,
    ]
  }

  private func liveSamplePayload(_ value: TelemetrySample) -> [String: Any] {
    [
      "sequence": value.sequence,
      "observedAt": ISO8601DateFormatter().string(from: value.observedAt),
      "elapsedMs": value.elapsedMs,
      "rideState": value.rideState.rawValue,
      "boardState": value.boardState ?? NSNull(),
      "footpadState": value.footpadState ?? NSNull(),
      "packVoltageV": value.packVoltageV ?? NSNull(),
      "dutyCycle": value.dutyCycle ?? NSNull(),
      "controllerTempC": value.controllerTempC ?? NSNull(),
      "motorTempC": value.motorTempC ?? NSNull(),
      "motorCurrentA": value.extensionFields["motorCurrentA"] ?? NSNull(),
      "batteryCurrentA": value.extensionFields["batteryCurrentA"] ?? NSNull(),
      "erpm": value.extensionFields["erpm"] ?? NSNull(),
      "speedKph": value.extensionFields["speedKph"] ?? NSNull(),
    ]
  }

  private func boardProfilePayload(_ value: BoardProfileRecord) throws -> [String: Any] {
    guard let fields = try JSONSerialization.jsonObject(with: Data(value.valuesJSON.utf8))
      as? [String: Any] else { throw ProductStoreError.invalidProfilePatch }
    return [
      "savedDeviceId": value.savedDeviceID,
      "revision": value.revision,
      "values": fields,
      "missingFields": value.missingFields,
    ]
  }

  private func sessionPayload(_ value: SessionSummaryRecord) -> [String: Any] {
    [
      "sessionId": value.sessionID,
      "deviceHistoryId": value.deviceHistoryID,
      "displayName": value.displayName,
      "startedAt": ISO8601DateFormatter().string(from: value.startedAt),
      "endedAt": value.endedAt.map(ISO8601DateFormatter().string) ?? NSNull(),
      "state": value.state,
      "integrity": value.integrity,
      "sampleCount": value.sampleCount,
      "eventCount": value.eventCount,
      "ridingElapsedMs": value.ridingElapsedMs,
      "totalElapsedMs": value.totalElapsedMs,
      "distanceM": value.distanceM ?? NSNull(),
      "maxSpeedMps": value.maxSpeedMps ?? NSNull(),
      "maxDutyCycle": value.maxDutyCycle ?? NSNull(),
    ]
  }

  private func seriesPayload(_ value: SessionSeriesPoint) -> [String: Any] {
    var fields: [String: Any] = [:]
    value.values.forEach { fields[$0.key] = $0.value ?? NSNull() }
    return ["elapsedMs": value.elapsedMs, "values": fields]
  }

  private func sessionGroupPayload(_ value: SessionGroupRecord) -> [String: Any] {
    [
      "sessionGroupId": value.sessionGroupID,
      "deviceHistoryId": value.deviceHistoryID,
      "sessionIds": value.sessionIDs,
      "totalElapsedMs": value.totalElapsedMs,
      "distanceM": value.distanceM ?? NSNull(),
    ]
  }

  private func settingsPayload(_ value: SettingsRecord) -> [String: Any] {
    [
      "revision": value.revision,
      "idleProbeIntervalMs": value.idleProbeIntervalMs,
      "idlePersistIntervalMs": value.idlePersistIntervalMs,
      "ridingPersistIntervalMs": value.ridingPersistIntervalMs,
      "postRideTailMs": value.postRideTailMs,
      "reconnectDurationMs": value.reconnectDurationMs,
      "automaticSessionLinkingEnabled": value.automaticSessionLinkingEnabled,
      "sessionLinkWindowMs": value.sessionLinkWindowMs,
      "themePreference": value.themePreference.rawValue,
      "unitPreference": value.unitPreference.rawValue,
    ]
  }

  private func deviceState(_ state: ConnectionState) -> String {
    switch state {
    case .disconnected: "disconnected"
    case .connecting: "connecting"
    case .connected: "connected"
    case .discoveringServices, .enablingNotifications: "discovering_services"
    case .ready: "ready"
    case .reconnecting: "reconnecting"
    case .disconnecting: "disconnecting"
    case .failed: "failed"
    }
  }

  private enum ErrorCode: String {
    case invalidRequest = "invalid_request"
    case invalidState = "invalid_state"
    case permissionRequired = "permission_required"
    case notFound = "not_found"
    case conflict
    case unavailable
    case `internal`
  }

  private func failure(_ id: Int, _ code: ErrorCode, _ message: String) -> Data {
    Self.json(["id": id, "error": ["code": code.rawValue, "message": message]])
  }

  private func applicationFailure(_ id: Int, _ error: ApplicationError) -> Data {
    switch error {
    case .permissionRequired: failure(id, .permissionRequired, "Bluetooth permission is required")
    case .unknownDevice: failure(id, .notFound, "Device was not found")
    case .activeDeviceConflict, .invalidScan, .deviceNotReady:
      failure(id, .invalidState, "Backend state does not permit the operation")
    case .bluetoothUnavailable, .productStoreUnavailable:
      failure(id, .unavailable, "Backend service is unavailable")
    }
  }

  private func productFailure(_ id: Int, _ error: ProductStoreError) -> Data {
    switch error {
    case .savedDeviceNotFound: failure(id, .notFound, "Saved device was not found")
    case .savedDeviceRevisionConflict, .profileRevisionConflict, .settingsRevisionConflict:
      failure(id, .conflict, "Resource revision changed")
    case .invalidSavedDevicePatch, .invalidProfilePatch, .invalidSettingsPatch:
      failure(id, .invalidRequest, "Patch is invalid")
    }
  }

  private func sessionFailure(_ id: Int, _ error: SessionStoreError) -> Data {
    switch error {
    case .sessionNotFound: failure(id, .notFound, "Session was not found")
    case .sessionActive: failure(id, .invalidState, "Session is still active")
    case .invalidCursor, .invalidRange, .unsupportedField, .invalidExportFormat:
      failure(id, .invalidRequest, "Session request is invalid")
    }
  }

  private static func json(_ value: Any) -> Data {
    try! JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
  }

  private static func jsonString(_ value: Any) throws -> String {
    String(decoding: try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys]), as: UTF8.self)
  }

  private static func randomToken() -> String {
    var bytes = [UInt8](repeating: 0, count: 32)
    _ = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)
    return Data(bytes).base64EncodedString().replacingOccurrences(of: "/", with: "_")
      .replacingOccurrences(of: "+", with: "-").replacingOccurrences(of: "=", with: "")
  }
}

private struct BackendMessageHandler: WSMessageHandler {
  let server: LoopbackServer

  func makeMessages(for client: AsyncStream<WSMessage>) async throws -> AsyncStream<WSMessage> {
    AsyncStream(bufferingPolicy: .bufferingNewest(1)) { continuation in
      let subscription = server.events.subscribe { event in
        if invalidatesState(event) {
          continuation.yield(.text(server.stateChanged()))
        }
      }
      Task {
        defer { _ = subscription }
        for await message in client {
          guard case .text(let text) = message,
                let data = text.data(using: .utf8),
                data.count <= 65_536,
                let request = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
          else {
            continuation.yield(.close(.invalidFramePayload))
            break
          }
          continuation.yield(.text(String(decoding: server.handle(request), as: UTF8.self)))
        }
        continuation.finish()
      }
    }
  }
}

private func invalidatesState(_ event: any Sendable) -> Bool {
  if let message = event as? VescProtocolMessage {
    switch message {
    case .firmware, .refloatInfo:
      return true
    case .canDevices, .refloatTelemetryLayout, .refloatTelemetry:
      return false
    }
  }
  return event is BluetoothState || event is DiscoverySnapshot || event is ConnectionState ||
    event is KnownDevice ||
    event is SessionRecord || event is SavedDeviceRecord || event is BoardProfileRecord ||
    event is SettingsRecord || event is TelemetrySample
}

private enum LoopbackServerError: Error {
  case failedToBind
}

private final class StartResult: @unchecked Sendable {
  private let lock = NSLock()
  private var value: Result<UInt16, Error>?

  func set(_ value: Result<UInt16, Error>) {
    lock.withLock { if self.value == nil { self.value = value } }
  }

  func get() -> Result<UInt16, Error> {
    lock.withLock { value ?? .failure(LoopbackServerError.failedToBind) }
  }
}
