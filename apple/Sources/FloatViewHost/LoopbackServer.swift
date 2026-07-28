import CryptoKit
import Foundation
import Network
import Security
#if SWIFT_PACKAGE
import FloatViewCore
#endif

public final class LoopbackServer: @unchecked Sendable {
  public let token: String
  public private(set) var port: UInt16 = 0
  public var origin: String { "http://127.0.0.1:\(port)" }
  private let platform: String
  private let webRoot: URL
  private let applicationService: ApplicationService?
  private let exportPresenter: (@Sendable (SessionExportRecord) -> Bool)?
  private let queue = DispatchQueue(label: "fail.acheron.float-view.loopback")
  private var listener: NWListener?

  public init(
    webRoot: URL,
    platform: String,
    applicationService: ApplicationService? = nil,
    exportPresenter: (@Sendable (SessionExportRecord) -> Bool)? = nil
  ) {
    self.webRoot = webRoot
    self.platform = platform
    self.applicationService = applicationService
    self.exportPresenter = exportPresenter
    token = Self.randomToken()
  }

  public func start() throws {
    let parameters = NWParameters.tcp
    parameters.requiredLocalEndpoint = .hostPort(host: .ipv4(.loopback), port: .any)
    let listener = try NWListener(using: parameters)
    listener.newConnectionHandler = { [weak self] in self?.accept($0) }
    listener.stateUpdateHandler = { [weak self] state in
      if case .ready = state, let endpoint = listener.port {
        self?.port = endpoint.rawValue
      }
    }
    self.listener = listener
    listener.start(queue: queue)
    while port == 0 { RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.005)) }
  }

  public func stop() {
    listener?.cancel()
    listener = nil
  }

  private func accept(_ connection: NWConnection) {
    connection.start(queue: queue)
    receiveHTTP(connection, buffer: Data())
  }

  private func receiveHTTP(_ connection: NWConnection, buffer: Data) {
    connection.receive(minimumIncompleteLength: 1, maximumLength: 65_536) {
      [weak self] data, _, complete, error in
      guard let self, error == nil else { connection.cancel(); return }
      let joined = buffer + (data ?? Data())
      if joined.range(of: Data("\r\n\r\n".utf8)) == nil {
        if complete || joined.count > 65_536 { connection.cancel() }
        else { self.receiveHTTP(connection, buffer: joined) }
        return
      }
      self.handleHTTP(connection, request: joined)
    }
  }

  private func handleHTTP(_ connection: NWConnection, request: Data) {
    guard let text = String(data: request, encoding: .utf8) else {
      respond(connection, status: "400 Bad Request", body: Data()); return
    }
    let lines = text.components(separatedBy: "\r\n")
    let parts = (lines.first ?? "").split(separator: " ")
    guard parts.count >= 2 else {
      respond(connection, status: "400 Bad Request", body: Data()); return
    }
    let target = String(parts[1])
    let headers: [String: String] = Dictionary(uniqueKeysWithValues: lines.dropFirst().compactMap { line in
      guard let colon = line.firstIndex(of: ":") else { return nil }
      return (
        String(line[..<colon]).lowercased(),
        String(line[line.index(after: colon)...]).trimmingCharacters(in: .whitespaces)
      )
    })
    if target.hasPrefix("/api/v1/ws") {
      guard query(target, "token") == token,
            headers["origin"] == origin,
            headers["upgrade"]?.lowercased() == "websocket",
            let key = headers["sec-websocket-key"]
      else { respond(connection, status: "403 Forbidden", body: Data()); return }
      let accept = Data(Insecure.SHA1.hash(data: Data((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").utf8))).base64EncodedString()
      let response = "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: \(accept)\r\n\r\n"
      connection.send(content: Data(response.utf8), completion: .contentProcessed { [weak self] _ in
        self?.receiveWebSocket(connection, buffer: Data(), initialized: false)
      })
      return
    }
    if target == "/api/bootstrap" {
      let body = try! JSONSerialization.data(withJSONObject: [
        "apiVersion": ["major": 1, "minor": 0],
        "webSocketUrl": "\(origin.replacingOccurrences(of: "http", with: "ws"))/api/v1/ws",
        "sessionToken": token,
      ])
      respond(connection, status: "200 OK", contentType: "application/json", body: body)
      return
    }
    let requested = target == "/" ? "index.html" : String(target.dropFirst())
    let safe = requested.split(separator: "/").filter { $0 != ".." }.joined(separator: "/")
    let url = webRoot.appending(path: safe)
    guard url.path.hasPrefix(webRoot.path), let body = try? Data(contentsOf: url) else {
      respond(connection, status: "404 Not Found", body: Data()); return
    }
    respond(connection, status: "200 OK", contentType: mime(url.pathExtension), body: body)
  }

  private func receiveWebSocket(_ connection: NWConnection, buffer: Data, initialized: Bool) {
    connection.receive(minimumIncompleteLength: 1, maximumLength: 65_536) {
      [weak self] data, _, complete, error in
      guard let self, error == nil else { connection.cancel(); return }
      var joined = buffer + (data ?? Data())
      var hello = initialized
      while let frame = Self.takeFrame(&joined) {
        switch frame.opcode {
        case 1:
          guard frame.payload.count <= 65_536,
                let object = try? JSONSerialization.jsonObject(with: frame.payload),
                let request = object as? [String: Any]
          else { self.sendClose(connection, code: 1007); return }
          let result = self.handle(request, initialized: &hello)
          self.sendFrame(connection, opcode: 1, payload: result)
        case 8: connection.cancel(); return
        case 9: self.sendFrame(connection, opcode: 10, payload: frame.payload)
        default: break
        }
      }
      if complete {
        connection.cancel()
      } else {
        let remaining = Data(joined)
        let nextInitialized = hello
        self.queue.async {
          self.receiveWebSocket(connection, buffer: remaining, initialized: nextInitialized)
        }
      }
    }
  }

  private func handle(_ request: [String: Any], initialized: inout Bool) -> Data {
    let id = request["id"] ?? NSNull()
    let method = request["method"] as? String ?? ""
    func response(_ result: Any) -> Data {
      try! JSONSerialization.data(withJSONObject: ["jsonrpc": "2.0", "id": id, "result": result])
    }
    func failure(_ code: Int, _ kind: String, _ message: String) -> Data {
      try! JSONSerialization.data(withJSONObject: [
        "jsonrpc": "2.0", "id": id,
        "error": ["code": code, "message": message, "data": ["kind": kind, "recoverable": false]],
      ])
    }
    if method == "system.hello" {
      let params = request["params"] as? [String: Any]
      let api = params?["api"] as? [String: Any]
      guard api?["major"] as? Int == 1, api?["minMinor"] as? Int ?? 1 <= 0 else {
        return failure(-32001, "unsupported", "Unsupported API version")
      }
      initialized = true
      return response([
        "api": ["major": 1, "minor": 0],
        "backend": ["name": "float-view-apple", "version": "0.1.0", "platform": platform],
        "capabilities": capabilities(),
        "limits": ["minTelemetryIntervalMs": 50, "maxMessageBytes": 65_536, "maxReconnectDurationMs": 3_600_000],
        "snapshot": snapshot(),
      ])
    }
    guard initialized else { return failure(-32000, "invalid_state", "system.hello required") }
    if method == "system.getSnapshot" { return response(snapshot()) }
    if method == "bluetooth.getState" {
      return response(bluetoothPayload())
    }
    if method == "bluetooth.requestPermission" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Bluetooth is unavailable")
      }
      return response(bluetoothPayload(applicationService.requestBluetoothPermission()))
    }
    if method == "discovery.start" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Discovery is unavailable")
      }
      do {
        return response(["scanId": try applicationService.startDiscovery()])
      } catch let error as ApplicationError {
        return applicationFailure(error, failure: failure)
      } catch {
        return failure(-32005, "scan_failed", "Unable to start discovery")
      }
    }
    if method == "discovery.stop" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Discovery is unavailable")
      }
      guard let params = request["params"] as? [String: Any],
            let scanID = params["scanId"] as? String
      else {
        return failure(-32602, "protocol_error", "scanId is required")
      }
      do {
        try applicationService.stopDiscovery(scanID: scanID)
        return response(["accepted": true])
      } catch let error as ApplicationError {
        return applicationFailure(error, failure: failure)
      } catch {
        return failure(-32005, "scan_failed", "Unable to stop discovery")
      }
    }
    if method == "device.connect" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Connections are unavailable")
      }
      guard let params = request["params"] as? [String: Any],
            let deviceID = params["deviceId"] as? String, !deviceID.isEmpty
      else {
        return failure(-32602, "protocol_error", "deviceId is required")
      }
      do {
        try applicationService.connect(DeviceID(deviceID))
        guard let active = applicationService.activeConnectionSnapshot() else {
          return failure(-32004, "connection_failed", "Connection did not start")
        }
        return response(connectionPayload(active))
      } catch let error as ApplicationError {
        return applicationFailure(error, failure: failure)
      } catch {
        return failure(-32004, "connection_failed", "Unable to connect")
      }
    }
    if method == "device.disconnect" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Connections are unavailable")
      }
      do {
        try applicationService.disconnect()
        return response(["accepted": true])
      } catch {
        return failure(-32004, "connection_failed", "Unable to disconnect cleanly")
      }
    }
    if method == "device.getActive" {
      guard let active = applicationService?.activeConnectionSnapshot() else {
        return response(NSNull())
      }
      return response(connectionPayload(active))
    }
    if method == "vesc.getFirmwareInfo" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "VESC protocol is unavailable")
      }
      do {
        guard let firmware = try applicationService.requestFirmwareInfo() else {
          return failure(-32007, "device_not_ready", "Waiting for the VESC firmware response")
        }
        guard let packageVersion = applicationService.refloatPackageVersionSnapshot() else {
          return failure(-32007, "device_not_ready", "Waiting for the Refloat package response")
        }
        let hardwareName: Any = firmware.hardwareName.map { $0 as Any } ?? NSNull()
        return response([
          "vescVersion": "\(firmware.major).\(String(format: "%02d", firmware.minor))",
          "packageName": "refloat",
          "packageVersion": packageVersion,
          "hardwareName": hardwareName,
          "supported": (packageVersion == "1.2" || packageVersion.hasPrefix("1.2.")) &&
            (firmware.major > 6 || (firmware.major == 6 && firmware.minor >= 2)),
        ])
      } catch let error as ApplicationError {
        return applicationFailure(error, failure: failure)
      } catch {
        return failure(-32007, "device_not_ready", "Unable to read VESC firmware information")
      }
    }
    if method == "savedDevices.list" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Saved devices are unavailable")
      }
      do {
        return response(try applicationService.savedDevices().map(savedDevicePayload))
      } catch {
        return failure(-32005, "storage_unavailable", "Unable to read saved devices")
      }
    }
    if method == "boardProfiles.get" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Board Profiles are unavailable")
      }
      guard let params = request["params"] as? [String: Any],
            let savedDeviceID = params["savedDeviceId"] as? String,
            !savedDeviceID.isEmpty
      else {
        return failure(-32602, "protocol_error", "savedDeviceId is required")
      }
      do {
        return response(try boardProfilePayload(
          applicationService.boardProfile(savedDeviceID: savedDeviceID)
        ))
      } catch let error as ProductStoreError {
        return productStoreFailure(error, failure: failure)
      } catch {
        return failure(-32005, "storage_unavailable", "Unable to read Board Profile")
      }
    }
    if method == "boardProfiles.update" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Board Profiles are unavailable")
      }
      guard let params = request["params"] as? [String: Any],
            let savedDeviceID = params["savedDeviceId"] as? String,
            !savedDeviceID.isEmpty,
            let expectedRevision = params["expectedRevision"] as? Int,
            expectedRevision >= 0,
            let patch = params["patch"] as? [String: Any],
            JSONSerialization.isValidJSONObject(patch),
            let patchData = try? JSONSerialization.data(
              withJSONObject: patch,
              options: [.sortedKeys]
            ),
            let patchJSON = String(
              data: patchData,
              encoding: .utf8
            )
      else {
        return failure(
          -32602, "protocol_error",
          "savedDeviceId, expectedRevision, and an object patch are required"
        )
      }
      do {
        return response(try boardProfilePayload(
          applicationService.updateBoardProfile(
            savedDeviceID: savedDeviceID,
            expectedRevision: expectedRevision,
            patchJSON: patchJSON
          )
        ))
      } catch let error as ProductStoreError {
        return productStoreFailure(error, failure: failure)
      } catch {
        return failure(-32005, "storage_unavailable", "Unable to update Board Profile")
      }
    }
    if method == "settings.get" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Settings are unavailable")
      }
      do {
        return response(settingsPayload(try applicationService.settings()))
      } catch let error as ProductStoreError {
        return productStoreFailure(error, failure: failure)
      } catch {
        return failure(-32005, "storage_unavailable", "Unable to read settings")
      }
    }
    if method == "settings.update" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Settings are unavailable")
      }
      guard let params = request["params"] as? [String: Any],
            let expectedRevision = params["expectedRevision"] as? Int,
            expectedRevision >= 0,
            let patch = params["patch"] as? [String: Any],
            JSONSerialization.isValidJSONObject(patch),
            let data = try? JSONSerialization.data(withJSONObject: patch, options: [.sortedKeys]),
            let patchJSON = String(data: data, encoding: .utf8)
      else {
        return failure(-32602, "protocol_error", "expectedRevision and an object patch are required")
      }
      do {
        return response(settingsPayload(try applicationService.updateSettings(
          expectedRevision: expectedRevision,
          patchJSON: patchJSON
        )))
      } catch let error as ProductStoreError {
        return productStoreFailure(error, failure: failure)
      } catch {
        return failure(-32005, "storage_unavailable", "Unable to update settings")
      }
    }
    if method == "sessions.list" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Sessions are unavailable")
      }
      let params = request["params"] as? [String: Any] ?? [:]
      let filters = params["filters"] as? [String: Any] ?? [:]
      let sort = params["sort"] as? [String: Any] ?? [:]
      let limit = params["limit"] as? Int ?? 20
      guard (1 ... 100).contains(limit) else {
        return failure(-32602, "protocol_error", "limit must be between 1 and 100")
      }
      let formatter = ISO8601DateFormatter()
      do {
        let page = try applicationService.sessions(
          SessionListQuery(
            cursor: params["cursor"] as? String,
            limit: limit,
            deviceHistoryID: filters["deviceHistoryId"] as? String,
            startedAfter: (filters["startedAfter"] as? String).flatMap(formatter.date),
            startedBefore: (filters["startedBefore"] as? String).flatMap(formatter.date),
            minimumTotalElapsedMs: (filters["minimumTotalElapsedMs"] as? NSNumber)?.int64Value,
            maximumTotalElapsedMs: (filters["maximumTotalElapsedMs"] as? NSNumber)?.int64Value,
            minimumRidingElapsedMs: (filters["minimumRidingElapsedMs"] as? NSNumber)?.int64Value,
            maximumRidingElapsedMs: (filters["maximumRidingElapsedMs"] as? NSNumber)?.int64Value,
            minimumDistanceM: (filters["minimumDistanceM"] as? NSNumber)?.doubleValue,
            maximumDistanceM: (filters["maximumDistanceM"] as? NSNumber)?.doubleValue,
            state: filters["state"] as? String,
            integrity: filters["integrity"] as? String,
            sortField: sort["field"] as? String ?? "startedAt",
            sortDescending: (sort["direction"] as? String ?? "desc") == "desc"
          )
        )
        return response([
          "items": page.items.map(sessionPayload),
          "nextCursor": page.nextCursor.map { $0 as Any } ?? NSNull(),
        ])
      } catch let error as SessionStoreError {
        return sessionStoreFailure(error, failure: failure)
      } catch {
        return failure(-32005, "storage_unavailable", "Unable to list sessions")
      }
    }
    if method == "sessions.get" || method == "sessions.getSummary" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Sessions are unavailable")
      }
      guard let params = request["params"] as? [String: Any],
            let sessionID = params["sessionId"] as? String, !sessionID.isEmpty
      else { return failure(-32602, "protocol_error", "sessionId is required") }
      do {
        return response(sessionPayload(try applicationService.session(sessionID: sessionID)))
      } catch let error as SessionStoreError {
        return sessionStoreFailure(error, failure: failure)
      } catch {
        return failure(-32005, "storage_unavailable", "Unable to read session")
      }
    }
    if method == "sessions.querySeries" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Session queries are unavailable")
      }
      guard let params = request["params"] as? [String: Any],
            let sessionID = params["sessionId"] as? String,
            let start = (params["startElapsedMs"] as? NSNumber)?.int64Value,
            let end = (params["endElapsedMs"] as? NSNumber)?.int64Value,
            let fields = params["fields"] as? [String],
            let maxPoints = (params["maxPoints"] as? NSNumber)?.intValue
      else { return failure(-32602, "protocol_error", "Invalid series query") }
      do {
        return response(try applicationService.querySessionSeries(
          sessionID: sessionID, startElapsedMs: start, endElapsedMs: end,
          fields: fields, maxPoints: maxPoints
        ).map(seriesPayload))
      } catch let error as SessionStoreError {
        return sessionStoreFailure(error, failure: failure)
      } catch {
        return failure(-32005, "storage_unavailable", "Unable to query session samples")
      }
    }
    if method == "sessions.listEvents" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Session queries are unavailable")
      }
      guard let params = request["params"] as? [String: Any],
            let sessionID = params["sessionId"] as? String,
            let start = (params["startElapsedMs"] as? NSNumber)?.int64Value,
            let end = (params["endElapsedMs"] as? NSNumber)?.int64Value
      else { return failure(-32602, "protocol_error", "Invalid event query") }
      do {
        return response(try applicationService.sessionEvents(
          sessionID: sessionID, startElapsedMs: start, endElapsedMs: end
        ).map(sessionEventPayload))
      } catch let error as SessionStoreError {
        return sessionStoreFailure(error, failure: failure)
      } catch {
        return failure(-32005, "storage_unavailable", "Unable to query session events")
      }
    }
    if method == "sessions.getDiagnosticWindow" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Session queries are unavailable")
      }
      guard let params = request["params"] as? [String: Any],
            let sessionID = params["sessionId"] as? String,
            let center = (params["centerElapsedMs"] as? NSNumber)?.int64Value,
            let before = (params["beforeMs"] as? NSNumber)?.int64Value,
            let after = (params["afterMs"] as? NSNumber)?.int64Value,
            let fields = params["fields"] as? [String]
      else { return failure(-32602, "protocol_error", "Invalid diagnostic query") }
      do {
        return response(try applicationService.diagnosticWindow(
          sessionID: sessionID, centerElapsedMs: center,
          beforeMs: before, afterMs: after, fields: fields
        ).map(seriesPayload))
      } catch let error as SessionStoreError {
        return sessionStoreFailure(error, failure: failure)
      } catch {
        return failure(-32005, "storage_unavailable", "Unable to query diagnostic samples")
      }
    }
    if method == "sessions.export" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Session export is unavailable")
      }
      guard let params = request["params"] as? [String: Any],
            let sessionID = params["sessionId"] as? String, !sessionID.isEmpty,
            let format = params["format"] as? String
      else { return failure(-32602, "protocol_error", "sessionId and format are required") }
      do {
        let value = try applicationService.exportSession(sessionID: sessionID, format: format)
        let delivery = exportPresenter?(value) == true ? "presented" : "staged"
        return response([
          "exportId": value.exportID,
          "fileName": value.fileName,
          "delivery": delivery,
        ])
      } catch let error as SessionStoreError {
        return sessionStoreFailure(error, failure: failure)
      } catch {
        return failure(-32005, "storage_unavailable", "Unable to export session")
      }
    }
    if method == "sessions.delete" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Session deletion is unavailable")
      }
      guard let params = request["params"] as? [String: Any],
            let sessionID = params["sessionId"] as? String, !sessionID.isEmpty
      else { return failure(-32602, "protocol_error", "sessionId is required") }
      do {
        try applicationService.deleteSession(sessionID: sessionID)
        return response(["accepted": true])
      } catch let error as SessionStoreError {
        return sessionStoreFailure(error, failure: failure)
      } catch {
        return failure(-32005, "storage_unavailable", "Unable to delete session")
      }
    }
    if method == "sessionGroups.list" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Session groups are unavailable")
      }
      do {
        return response(try applicationService.sessionGroups().map(sessionGroupPayload))
      } catch {
        return failure(-32005, "storage_unavailable", "Unable to list session groups")
      }
    }
    if method == "sessionGroups.get" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Session groups are unavailable")
      }
      guard let params = request["params"] as? [String: Any],
            let groupID = params["sessionGroupId"] as? String, !groupID.isEmpty
      else { return failure(-32602, "protocol_error", "sessionGroupId is required") }
      do {
        return response(sessionGroupPayload(
          try applicationService.sessionGroup(sessionGroupID: groupID)
        ))
      } catch let error as SessionStoreError {
        return sessionStoreFailure(error, failure: failure)
      } catch {
        return failure(-32005, "storage_unavailable", "Unable to read session group")
      }
    }
    if method == "sessionGroups.unlink" {
      guard let applicationService else {
        return failure(-32601, "unsupported", "Session groups are unavailable")
      }
      guard let params = request["params"] as? [String: Any],
            let groupID = params["sessionGroupId"] as? String, !groupID.isEmpty
      else { return failure(-32602, "protocol_error", "sessionGroupId is required") }
      do {
        try applicationService.unlinkSessionGroup(sessionGroupID: groupID)
        return response(["accepted": true])
      } catch {
        return failure(-32005, "storage_unavailable", "Unable to unlink session group")
      }
    }
    return failure(-32601, "unsupported", "Unknown method: \(method)")
  }

  private func snapshot() -> [String: Any] {
    let discovery = applicationService?.discoverySnapshot()
    return [
      "revision": 0,
      "bluetooth": discovery.map { bluetoothPayload($0.bluetoothState) } ??
        ["radio": "unknown", "scanPermission": "unknown", "connectPermission": "unknown"],
      "knownDevices": discovery?.knownDevices.map(devicePayload) ?? [],
      "activeConnection": applicationService?.activeConnectionSnapshot().map(connectionPayload) ??
        NSNull(),
      "telemetrySubscriptions": [], "loggingJobs": [],
    ]
  }

  private func capabilities() -> [[String: Any]] {
    [
      ["name": "system.snapshot", "available": true],
      ["name": "bluetooth.discovery", "available": applicationService != nil],
      ["name": "device.connection", "available": applicationService != nil],
      ["name": "boardProfiles.local", "available": applicationService != nil],
      ["name": "sessions.review", "available": applicationService != nil],
      ["name": "sessions.manage", "available": applicationService != nil],
      ["name": "sessions.export", "available": applicationService != nil],
      ["name": "settings.local", "available": applicationService != nil],
    ]
  }

  private func bluetoothPayload() -> [String: Any] {
    guard let applicationService else {
      return ["radio": "unknown", "scanPermission": "unknown", "connectPermission": "unknown"]
    }
    return bluetoothPayload(applicationService.discoverySnapshot().bluetoothState)
  }

  private func bluetoothPayload(_ state: BluetoothState) -> [String: Any] {
    [
      "radio": state.radio.rawValue,
      "scanPermission": state.scanPermission.rawValue,
      "connectPermission": state.connectPermission.rawValue,
    ]
  }

  private func devicePayload(_ device: KnownDevice) -> [String: Any] {
    [
      "deviceId": device.deviceID.value,
      "name": device.name ?? "",
      "state": "discovered",
      "lastSeenAt": ISO8601DateFormatter().string(from: device.lastSeenAt),
    ]
  }

  private func connectionPayload(_ connection: ActiveConnectionSnapshot) -> [String: Any] {
    [
      "deviceId": connection.deviceID.value,
      "name": connection.name ?? "",
      "state": deviceState(connection.state),
    ]
  }

  private func savedDevicePayload(_ device: SavedDeviceRecord) -> [String: Any] {
    [
      "savedDeviceId": device.savedDeviceID,
      "deviceHistoryId": device.deviceHistoryID,
      "displayName": device.displayName,
      "advertisedName": device.advertisedName ?? NSNull(),
      "lastSeenAt": device.lastSeenAt.map { ISO8601DateFormatter().string(from: $0) } ?? NSNull(),
      "lastConnectedAt": device.lastConnectedAt.map {
        ISO8601DateFormatter().string(from: $0)
      } ?? NSNull(),
      "profileComplete": device.profileComplete,
    ]
  }

  private func boardProfilePayload(_ profile: BoardProfileRecord) throws -> [String: Any] {
    guard let values = try JSONSerialization.jsonObject(with: Data(profile.valuesJSON.utf8))
      as? [String: Any]
    else {
      throw ProductStoreError.invalidProfilePatch
    }
    return [
      "savedDeviceId": profile.savedDeviceID,
      "revision": profile.revision,
      "values": values,
      "missingFields": profile.missingFields,
    ]
  }

  private func sessionPayload(_ value: SessionSummaryRecord) -> [String: Any] {
    [
      "sessionId": value.sessionID,
      "deviceHistoryId": value.deviceHistoryID,
      "displayName": value.displayName,
      "startedAt": ISO8601DateFormatter().string(from: value.startedAt),
      "endedAt": value.endedAt.map { ISO8601DateFormatter().string(from: $0) } ?? NSNull(),
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

  private func sessionEventPayload(_ value: SessionEventRecord) -> [String: Any] {
    let details = (try? JSONSerialization.jsonObject(with: Data(value.detailsJSON.utf8)))
      as? [String: Any] ?? [:]
    return [
      "eventId": value.eventID,
      "elapsedMs": value.elapsedMs,
      "kind": value.kind,
      "details": details,
    ]
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
      "rawPacketCaptureEnabled": value.rawPacketCaptureEnabled,
    ]
  }

  private func sessionStoreFailure(
    _ error: SessionStoreError,
    failure: (Int, String, String) -> Data
  ) -> Data {
    switch error {
    case .sessionNotFound:
      failure(-32004, "device_not_found", "Session not found")
    case .sessionActive:
      failure(-32000, "invalid_state", "Active sessions cannot be exported or deleted")
    case .invalidCursor, .invalidRange:
      failure(-32602, "protocol_error", "Invalid session query")
    case .unsupportedField(let field):
      failure(-32602, "protocol_error", "Unsupported session field: \(field)")
    case .invalidExportFormat:
      failure(-32602, "protocol_error", "Invalid session export format")
    }
  }

  private func productStoreFailure(
    _ error: ProductStoreError,
    failure: (Int, String, String) -> Data
  ) -> Data {
    switch error {
    case .savedDeviceNotFound:
      failure(-32004, "device_not_found", "Saved device not found")
    case .profileRevisionConflict:
      failure(-32000, "invalid_state", "Profile revision conflict")
    case .invalidProfilePatch:
      failure(-32602, "protocol_error", "Invalid Board Profile patch")
    case .settingsRevisionConflict:
      failure(-32000, "invalid_state", "Settings revision conflict")
    case .invalidSettingsPatch:
      failure(-32602, "protocol_error", "Invalid Settings patch")
    }
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

  private func applicationFailure(
    _ error: ApplicationError,
    failure: (Int, String, String) -> Data
  ) -> Data {
    switch error {
    case .bluetoothUnavailable:
      failure(-32000, "invalid_state", "Bluetooth is not powered on")
    case .permissionRequired:
      failure(-32003, "permission_required", "Bluetooth scan permission is required")
    case .invalidScan:
      failure(-32000, "invalid_state", "Unknown active scan")
    case .activeDeviceConflict:
      failure(-32000, "invalid_state", "Another board is active")
    case .unknownDevice:
      failure(-32002, "not_found", "The discovered board is no longer available")
    case .deviceNotReady:
      failure(-32007, "device_not_ready", "The VESC connection is not ready")
    case .productStoreUnavailable:
      failure(-32601, "unsupported", "Product persistence is unavailable")
    }
  }

  private func respond(
    _ connection: NWConnection,
    status: String,
    contentType: String = "text/plain",
    body: Data
  ) {
    let headers = "HTTP/1.1 \(status)\r\nContent-Type: \(contentType)\r\nContent-Length: \(body.count)\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n"
    connection.send(content: Data(headers.utf8) + body, completion: .contentProcessed { _ in
      connection.cancel()
    })
  }

  private func sendFrame(_ connection: NWConnection, opcode: UInt8, payload: Data) {
    var frame = Data([0x80 | opcode])
    if payload.count < 126 { frame.append(UInt8(payload.count)) }
    else if payload.count <= 65_535 {
      frame.append(126)
      frame.append(UInt8(truncatingIfNeeded: payload.count >> 8))
      frame.append(UInt8(truncatingIfNeeded: payload.count))
    } else { sendClose(connection, code: 1009); return }
    frame.append(payload)
    connection.send(content: frame, completion: .contentProcessed { _ in })
  }

  private func sendClose(_ connection: NWConnection, code: UInt16) {
    sendFrame(
      connection,
      opcode: 8,
      payload: Data([UInt8(truncatingIfNeeded: code >> 8), UInt8(truncatingIfNeeded: code)])
    )
    connection.cancel()
  }

  private static func takeFrame(_ buffer: inout Data) -> (opcode: UInt8, payload: Data)? {
    // Data.removeFirst advances startIndex; normalize before integer indexing
    // because subsequent TCP receives may append to that same slice.
    if buffer.startIndex != 0 { buffer = Data(buffer) }
    guard buffer.count >= 2 else { return nil }
    let opcode = buffer[0] & 0x0f
    let masked = buffer[1] & 0x80 != 0
    var length = Int(buffer[1] & 0x7f)
    var offset = 2
    if length == 126 {
      guard buffer.count >= 4 else { return nil }
      length = Int(buffer[2]) << 8 | Int(buffer[3]); offset = 4
    } else if length == 127 {
      return nil
    }
    let maskCount = masked ? 4 : 0
    guard buffer.count >= offset + maskCount + length else { return nil }
    let mask = masked ? Array(buffer[offset..<(offset + 4)]) : []
    offset += maskCount
    var payload = Data(buffer[offset..<(offset + length)])
    if masked {
      for index in payload.indices { payload[index] ^= mask[index % 4] }
    }
    buffer.removeFirst(offset + length)
    return (opcode, payload)
  }

  private func query(_ target: String, _ name: String) -> String? {
    URLComponents(string: "http://localhost\(target)")?.queryItems?.first { $0.name == name }?.value
  }

  private func mime(_ ext: String) -> String {
    switch ext { case "html": "text/html"; case "js": "text/javascript"; case "css": "text/css"; case "svg": "image/svg+xml"; default: "application/octet-stream" }
  }

  private static func randomToken() -> String {
    var bytes = [UInt8](repeating: 0, count: 32)
    _ = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)
    return Data(bytes).base64EncodedString().replacingOccurrences(of: "/", with: "_")
      .replacingOccurrences(of: "+", with: "-").replacingOccurrences(of: "=", with: "")
  }
}
