import FlyingFox
import Foundation
import Security
import zlib

public enum PlatformLoopbackPort {
  public static let macOS: UInt16 = 11_010
  public static let iOS: UInt16 = 11_011
}

public enum LoopbackProtocolLimits {
  public static let maximumWebSocketMessageBytes = 65_536
  public static let maximumSessionImportBytes = 128 * 1024 * 1024
}

public func loopbackStartupFailure(port: UInt16, error: any Error) -> String {
  "failed to bind Float View backend to 127.0.0.1:\(port): \(error)"
}

private final class SessionDataTiming: @unchecked Sendable {
  let startedAt = DispatchTime.now().uptimeNanoseconds
  var kind = "unknown"
  var sourceRows = 0
  var metadataMs = 0.0
  var configurationMs = 0.0
  var sourceMs = 0.0
  var reducerMs = 0.0
  var jsonMs = 0.0
  var gzipMs = 0.0
  var responseBytes = 0
  var compressedBytes = 0

  var totalMs: Double {
    Double(DispatchTime.now().uptimeNanoseconds - startedAt) / 1_000_000
  }
}

public final class LoopbackServer: @unchecked Sendable {
  public let token: String
  public private(set) var port: UInt16 = 0
  public var origin: String {
    "http://127.0.0.1:\(port)"
  }

  private let html: Data
  private let platform: String
  private let requestedPort: UInt16
  private let applicationService: ApplicationService
  private let exportPresenter: (@Sendable (SessionExportRecord) -> Bool)?
  fileprivate let events: EventHub
  private let revisionLock = NSLock()
  private var revision = 0
  private var sessionCatalogRevision = 0
  private var server: HTTPServer?
  private var serverTask: Task<Void, Never>?

  public init(
    html: Data,
    port: UInt16 = 0,
    applicationService: ApplicationService,
    exportPresenter: (@Sendable (SessionExportRecord) -> Bool)? = nil,
    events: EventHub,
    platform: String = "macos"
  ) {
    self.html = html
    self.platform = platform
    requestedPort = port
    self.applicationService = applicationService
    self.exportPresenter = exportPresenter
    self.events = events
    token = Self.randomToken()
  }

  public func start() throws {
    guard server == nil else { return }
    let server = try HTTPServer(address: sockaddr_in.inet(ip4: "127.0.0.1", port: requestedPort))
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
          "sessionDataUrl": "\(self.origin)/api/session-data",
          "sessionExportUrl": "\(self.origin)/api/session-export",
          "sessionImportUrl": "\(self.origin)/api/session-import",
          "sessionToken": self.token,
          "platform": self.platform,
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
            frameSize: LoopbackProtocolLimits.maximumWebSocketMessageBytes
          )
        ).handleRequest(request)
      }
      await server.appendRoute("POST /api/session-data") { [weak self] request in
        guard let self,
              request.headers[HTTPHeader("Origin")] == self.origin,
              request.headers[HTTPHeader("X-Float-View-Token")] == self.token
        else { return HTTPResponse(statusCode: .forbidden, headers: [.cacheControl: "no-store"]) }
        if let contentLength = request.headers[HTTPHeader("Content-Length")].flatMap(Int.init), contentLength > 16_384 {
          return HTTPResponse(statusCode: .payloadTooLarge, headers: [.cacheControl: "no-store"])
        }
        let timing = SessionDataTiming()
        let body = try await request.bodyData
        guard body.count <= 16_384 else { return HTTPResponse(
          statusCode: .payloadTooLarge,
          headers: [.cacheControl: "no-store"]
        ) }
        do {
          let decodeStarted = DispatchTime.now().uptimeNanoseconds
          guard let payload = try JSONSerialization.jsonObject(with: body) as? [String: Any] else {
            return HTTPResponse(statusCode: .badRequest, headers: [.cacheControl: "no-store"])
          }
          timing.jsonMs = Double(DispatchTime.now().uptimeNanoseconds - decodeStarted) / 1_000_000
          guard let sessionID = payload["sessionId"] as? String, !sessionID.isEmpty else {
            return HTTPResponse(statusCode: .badRequest, headers: [.cacheControl: "no-store"])
          }
          let readCancellation = SessionReadCancellation()
          let result = try await withTaskCancellationHandler(operation: {
            try self.applicationService.withSessionReadCancellation(readCancellation) {
              try self.sessionDataPayload(payload, timing: timing)
            }
          }, onCancel: {
            readCancellation.cancel()
          })
          guard let result else {
            return HTTPResponse(statusCode: .badRequest, headers: [.cacheControl: "no-store"])
          }
          let encodeStarted = DispatchTime.now().uptimeNanoseconds
          let responseData = try JSONSerialization.data(withJSONObject: result)
          timing.responseBytes = responseData.count
          timing.jsonMs += Double(DispatchTime.now().uptimeNanoseconds - encodeStarted) / 1_000_000
          guard responseData.count <= 4 * 1024 * 1024 else {
            return HTTPResponse(statusCode: .payloadTooLarge, headers: [.cacheControl: "no-store"])
          }
          if request.headers[HTTPHeader("Accept-Encoding")]?.localizedCaseInsensitiveContains("gzip") == true,
             let compressed = Self.gzip(responseData, timing: timing)
          {
            timing.compressedBytes = compressed.count
            Self.logSessionDataTiming(timing)
            return HTTPResponse(
              statusCode: .ok,
              headers: [
                .contentType: "application/json",
                .cacheControl: "no-store",
                HTTPHeader("Content-Encoding"): "gzip",
              ],
              body: compressed
            )
          }
          Self.logSessionDataTiming(timing)
          return HTTPResponse(
            statusCode: .ok,
            headers: [.contentType: "application/json", .cacheControl: "no-store"],
            body: responseData
          )
        } catch {
          if error is CancellationError || Task.isCancelled { throw CancellationError() }
          if error is SessionStoreError {
            return HTTPResponse(statusCode: .badRequest, headers: [.cacheControl: "no-store"])
          }
          print("[FloatView] session-data failed (\(type(of: error)))")
          return HTTPResponse(statusCode: .internalServerError, headers: [.cacheControl: "no-store"])
        }
      }
      await server.appendRoute("POST /api/session-import") { [weak self] request in
        guard let self,
              request.headers[HTTPHeader("Origin")] == self.origin,
              request.headers[HTTPHeader("X-Float-View-Token")] == self.token
        else { return HTTPResponse(statusCode: .forbidden, headers: [.cacheControl: "no-store"]) }
        if let contentLength = request.headers[HTTPHeader("Content-Length")].flatMap(Int.init),
           contentLength > LoopbackProtocolLimits.maximumSessionImportBytes
        {
          return HTTPResponse(statusCode: .payloadTooLarge, headers: [.cacheControl: "no-store"])
        }
        do {
          let body = try await request.bodyData
          guard body.count <= LoopbackProtocolLimits.maximumSessionImportBytes else {
            return HTTPResponse(statusCode: .payloadTooLarge, headers: [.cacheControl: "no-store"])
          }
          let sessionID = try self.applicationService.importSessionArchive(data: body)
          self.events.publish(SessionCatalogChanged())
          let response = try JSONSerialization.data(withJSONObject: ["sessionId": sessionID])
          return HTTPResponse(
            statusCode: .ok,
            headers: [.contentType: "application/json", .cacheControl: "no-store"],
            body: response
          )
        } catch {
          return HTTPResponse(statusCode: .badRequest, headers: [.cacheControl: "no-store"])
        }
      }
      await server.appendRoute("POST /api/session-export") { [weak self] request in
        guard let self,
              request.headers[HTTPHeader("Origin")] == self.origin,
              request.headers[HTTPHeader("X-Float-View-Token")] == self.token
        else { return HTTPResponse(statusCode: .forbidden, headers: [.cacheControl: "no-store"]) }
        if let contentLength = request.headers[HTTPHeader("Content-Length")].flatMap(Int.init), contentLength > 16_384 {
          return HTTPResponse(statusCode: .payloadTooLarge, headers: [.cacheControl: "no-store"])
        }
        do {
          let body = try await request.bodyData
          guard body.count <= 16_384,
                let payload = try JSONSerialization.jsonObject(with: body) as? [String: Any],
                let sessionID = payload["sessionId"] as? String,
                !sessionID.isEmpty,
                let format = payload["format"] as? String,
                format == "csv" || format == "float-view-archive"
          else { return HTTPResponse(statusCode: .badRequest, headers: [.cacheControl: "no-store"]) }
          let value = try self.applicationService.exportSession(sessionID: sessionID, format: format)
          let file = try HTTPBodySequence(file: value.fileURL)
          return HTTPResponse(
            statusCode: .ok,
            headers: [
              .contentType: format == "csv" ? "text/csv" : "application/octet-stream",
              .cacheControl: "no-store",
              HTTPHeader("Content-Disposition"): "attachment; filename=\"\(value.fileName)\"",
            ],
            body: file
          )
        } catch {
          return HTTPResponse(statusCode: .badRequest, headers: [.cacheControl: "no-store"])
        }
      }
      let running = Task { try await server.run() }
      do {
        try await server.waitUntilListening()
        guard let address = await server.listeningAddress else {
          throw LoopbackServerError.failedToBind
        }
        switch address {
        case let .ip4(_, value), let .ip6(_, value): self.port = value
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
      case "location.requestPermission":
        return success(locationPayload(applicationService.requestLocationPermission()))
      case "location.openSettings":
        return success(locationPayload(applicationService.openLocationSettings()))
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
        guard let trackingRaw = params["trackingMode"] as? String,
              let trackingMode = TrackingMode(rawValue: trackingRaw)
        else { return failure(id, .invalidRequest, "trackingMode must be saved or guest") }
        if let active = applicationService.activeConnectionSnapshot(),
           active.deviceID.value != deviceID || active.state != .failed
        {
          return failure(id, .invalidState, "A different board is already active")
        }
        try applicationService.connect(DeviceID(deviceID), trackingMode: trackingMode)
        return success(["accepted": true])
      case "device.selectController":
        guard let deviceID = params["deviceId"] as? String, !deviceID.isEmpty,
              let number = params["canId"] as? NSNumber,
              CFGetTypeID(number) != CFBooleanGetTypeID(),
              number.doubleValue.rounded() == number.doubleValue,
              (0 ... 253).contains(number.intValue)
        else {
          return failure(id, .invalidRequest, "deviceId and CAN ID 0...253 are required")
        }
        try applicationService.selectController(
          deviceID: DeviceID(deviceID),
          canID: UInt8(number.intValue)
        )
        return success(["accepted": true])
      case "device.disconnect":
        try applicationService.disconnect()
        return success(["accepted": true])
      case "controller.shutdown":
        guard params.isEmpty else {
          return failure(id, .invalidRequest, "Controller shutdown does not accept parameters")
        }
        try applicationService.shutdownController()
        return success(["accepted": true])
      case "refloat.control.set":
        guard params.count == 2,
              let rawControl = params["control"] as? String,
              let control = RefloatBooleanControl(rawValue: rawControl),
              let enabled = params["enabled"] as? Bool
        else { return failure(id, .invalidRequest, "A valid Refloat control and boolean are required") }
        return try success(refloatControlsPayload(
          applicationService.setRefloatControl(control, enabled: enabled)
        ))
      case "refloat.fatalError.clear":
        guard params.isEmpty else {
          return failure(id, .invalidRequest, "Fatal-error clear does not accept parameters")
        }
        return try success(refloatControlsPayload(applicationService.clearRefloatFatalError()))
      case "refloat.configuration.open":
        guard params.isEmpty else {
          return failure(id, .invalidRequest, "Configuration open does not accept parameters")
        }
        return success(refloatConfigurationSummaryPayload(
          applicationService.refloatConfigurationSummarySnapshot()
        ))
      case "refloat.configuration.schema":
        guard params.keys.allSatisfy({ $0 == "cursor" }),
              let page = refloatConfigurationSchemaPage(cursor: params["cursor"])
        else { return failure(id, .invalidRequest, "Configuration schema cursor is invalid") }
        return success(page)
      case "refloat.configuration.read":
        guard params.count == 1, let source = params["source"] as? String,
              source == "current" || source == "default"
        else { return failure(id, .invalidRequest, "Configuration source must be current or default") }
        return try success(refloatConfigurationValuesPayload(
          applicationService.readRefloatConfiguration(defaults: source == "default")
        ))
      case "refloat.configuration.write":
        guard params.count == 5,
              let packageVersion = params["packageVersion"] as? String,
              !packageVersion.isEmpty, packageVersion.utf8.count <= 128,
              let fingerprint = params["schemaFingerprint"] as? String,
              fingerprint.range(of: "^[0-9a-f]{64}$", options: .regularExpression) != nil,
              let signatureNumber = params["wireSignature"] as? NSNumber,
              CFGetTypeID(signatureNumber) != CFBooleanGetTypeID(),
              signatureNumber.doubleValue.rounded() == signatureNumber.doubleValue,
              (0 ... Double(UInt32.max)).contains(signatureNumber.doubleValue),
              let baseNumber = params["baseRevision"] as? NSNumber,
              CFGetTypeID(baseNumber) != CFBooleanGetTypeID(),
              baseNumber.doubleValue.rounded() == baseNumber.doubleValue,
              baseNumber.doubleValue >= 0,
              let rawValues = params["values"] as? [String: Any],
              let values = refloatConfigurationValues(rawValues)
        else { return failure(id, .invalidRequest, "Complete typed configuration values are required") }
        return try success(refloatConfigurationValuesPayload(
          applicationService.writeRefloatConfiguration(
            packageVersion: packageVersion, fingerprint: fingerprint,
            wireSignature: signatureNumber.uint32Value, baseRevision: baseNumber.uint64Value,
            values: values
          )
        ))
      case "refloat.tune.list":
        guard params.keys.allSatisfy({ ["identity", "limit", "cursor"].contains($0) }),
              let identity = refloatConfigurationIdentity(params["identity"]),
              let limit = params["limit"] as? Int, (1 ... 50).contains(limit),
              params["cursor"] == nil || params["cursor"] is NSNull ||
              ((params["cursor"] as? String)?.utf8.count ?? 513) <= 512
        else { return failure(id, .invalidRequest, "Tune list parameters are invalid") }
        let page = try applicationService.refloatTunes(
          identity: identity, limit: limit, cursor: params["cursor"] as? String
        )
        return success([
          "items": page.items.map { refloatTunePayload($0, includeValues: false) },
          "nextCursor": page.nextCursor.map { $0 as Any } ?? NSNull(),
        ])
      case "refloat.tune.get":
        guard params.count == 1, let tuneID = params["tuneId"] as? String, !tuneID.isEmpty,
              tuneID.utf8.count <= 128
        else { return failure(id, .invalidRequest, "Tune ID is invalid") }
        return try success(refloatTunePayload(
          applicationService.refloatTune(configurationID: tuneID), includeValues: true
        ))
      case "refloat.tune.create":
        guard params.count == 5,
              let name = boundedRefloatText(params["name"], maximumBytes: 128),
              params.keys.contains("description"),
              params["description"] is NSNull || boundedRefloatText(params["description"], maximumBytes: 1_024) != nil,
              let identity = refloatConfigurationIdentity(params["identity"]),
              let source = params["sourceRevision"] as? NSNumber,
              source.doubleValue >= 0, source.doubleValue.rounded() == source.doubleValue,
              let rawValues = params["values"] as? [String: Any],
              let values = refloatConfigurationValues(rawValues)
        else { return failure(id, .invalidRequest, "Tune create parameters are invalid") }
        let description = boundedRefloatText(params["description"], maximumBytes: 1_024)
        return try success(refloatTunePayload(applicationService.createRefloatTune(
          name: name, description: description, identity: identity,
          sourceRevision: source.uint64Value, values: values
        ), includeValues: true))
      case "refloat.tune.update":
        guard params.count == 5,
              let tuneID = params["tuneId"] as? String, !tuneID.isEmpty, tuneID.utf8.count <= 128,
              let revision = params["expectedRevision"] as? Int, revision >= 1,
              let name = boundedRefloatText(params["name"], maximumBytes: 128),
              params.keys.contains("description"),
              params["description"] is NSNull || boundedRefloatText(params["description"], maximumBytes: 1_024) != nil,
              let rawValues = params["values"] as? [String: Any],
              let values = refloatConfigurationValues(rawValues)
        else { return failure(id, .invalidRequest, "Tune update parameters are invalid") }
        let description = boundedRefloatText(params["description"], maximumBytes: 1_024)
        return try success(refloatTunePayload(applicationService.updateRefloatTune(
          configurationID: tuneID, expectedRevision: revision, name: name,
          description: description, values: values
        ), includeValues: true))
      case "refloat.tune.delete":
        guard params.count == 2, let tuneID = params["tuneId"] as? String, !tuneID.isEmpty,
              let revision = params["expectedRevision"] as? Int, revision >= 1
        else { return failure(id, .invalidRequest, "Tune delete parameters are invalid") }
        try applicationService.deleteRefloatTune(configurationID: tuneID, expectedRevision: revision)
        return success(["accepted": true])
      case "refloat.tune.apply":
        guard params.count == 3, let tuneID = params["tuneId"] as? String, !tuneID.isEmpty,
              let revision = params["expectedRevision"] as? Int, revision >= 1,
              let base = params["baseRevision"] as? NSNumber,
              base.doubleValue >= 0, base.doubleValue.rounded() == base.doubleValue
        else { return failure(id, .invalidRequest, "Tune apply parameters are invalid") }
        return try success(refloatConfigurationValuesPayload(applicationService.applyRefloatTune(
          configurationID: tuneID, expectedRevision: revision, baseRevision: base.uint64Value
        )))
      case "savedDevice.update":
        guard let savedID = params["savedDeviceId"] as? String,
              let expected = params["expectedRevision"] as? Int,
              params.keys.contains("customDisplayName"),
              params["customDisplayName"] is String || params["customDisplayName"] is NSNull
        else { return failure(id, .invalidRequest, "Valid saved-device update is required") }
        return try success(savedDevicePayload(applicationService.updateSavedDevice(
          savedDeviceID: savedID,
          expectedRevision: expected,
          customDisplayName: params["customDisplayName"] as? String
        )))
      case "savedDevice.delete":
        guard let savedID = params["savedDeviceId"] as? String, !savedID.isEmpty else {
          return failure(id, .invalidRequest, "savedDeviceId is required")
        }
        try applicationService.deleteSavedDevice(savedDeviceID: savedID)
        events.publish(SessionCatalogChanged())
        return success(["accepted": true])
      case "settings.update":
        guard let expected = params["expectedRevision"] as? Int,
              let patch = params["patch"] as? [String: Any]
        else { return failure(id, .invalidRequest, "Revision and patch are required") }
        return try success(settingsPayload(applicationService.updateSettings(
          expectedRevision: expected,
          patchJSON: Self.jsonString(patch)
        )))
      case "boardProfile.get":
        guard let savedID = params["savedDeviceId"] as? String else {
          return failure(id, .invalidRequest, "savedDeviceId is required")
        }
        return try success(boardProfilePayload(
          applicationService.boardProfile(savedDeviceID: savedID)
        ))
      case "boardProfile.update":
        guard let savedID = params["savedDeviceId"] as? String,
              let expected = params["expectedRevision"] as? Int,
              let patch = params["patch"] as? [String: Any]
        else { return failure(id, .invalidRequest, "Valid profile update is required") }
        return try success(boardProfilePayload(applicationService.updateBoardProfile(
          savedDeviceID: savedID,
          expectedRevision: expected,
          patchJSON: Self.jsonString(patch)
        )))
      case "session.list":
        guard let savedID = params["savedDeviceId"] as? String, !savedID.isEmpty,
              let limit = (params["limit"] as? NSNumber)?.intValue, (1 ... 100).contains(limit)
        else {
          return failure(id, .invalidRequest, "savedDeviceId and a valid session page size are required")
        }
        guard let board = try applicationService.savedDevices().first(where: { $0.savedDeviceID == savedID }) else {
          return failure(id, .notFound, "Saved board was not found")
        }
        let deviceHistoryID = board.deviceHistoryID
        let cursorValue = params["cursor"] as? String
        let context: [String: Any]?
        if let cursorValue {
          guard cursorValue.utf8.count <= 1_024,
                let decoded = Self.decodeSessionCatalogCursor(cursorValue),
                decoded["deviceHistoryId"] as? String == deviceHistoryID
          else { return failure(id, .invalidRequest, "Session cursor is invalid") }
          guard let revision = decoded["catalogRevision"] as? Int,
                revision == revisionLock.withLock({ sessionCatalogRevision })
          else { return failure(id, .conflict, "Session cursor is stale") }
          context = decoded
        } else {
          if params.keys.contains("cursor"), !(params["cursor"] is NSNull) {
            return failure(id, .invalidRequest, "Session cursor is invalid")
          }
          context = nil
        }
        let innerCursor = context?["innerCursor"] as? String
        let page = try applicationService.sessions(SessionListQuery(
          cursor: innerCursor, limit: limit, deviceHistoryID: deviceHistoryID,
          sortField: "startedAt", sortDescending: true
        ))
        let nextCursor = page.nextCursor.map { inner in
          Self.encodeSessionCatalogCursor([
            "version": 1,
            "query": "startedAt.desc.sessionId.desc",
            "deviceHistoryId": deviceHistoryID,
            "catalogRevision": revisionLock.withLock { sessionCatalogRevision },
            "innerCursor": inner,
            "lastStartedAt": page.items.last.map { ISO8601DateFormatter().string(from: $0.startedAt) } ?? NSNull(),
            "lastSessionId": page.items.last?.sessionID ?? NSNull(),
          ])
        }
        return success([
          "items": page.items.map(sessionPayload),
          "catalogRevision": revisionLock.withLock { sessionCatalogRevision },
          "nextCursor": nextCursor.map { $0 as Any } ?? NSNull(),
        ])
      case "session.get":
        guard let sessionID = params["sessionId"] as? String else {
          return failure(id, .invalidRequest, "sessionId is required")
        }
        let summary = try applicationService.session(sessionID: sessionID)
        let extensionFields = try applicationService.sessionExtensionFields(sessionID: sessionID)
        let published = try applicationService.sessionSnapshot(sessionID: sessionID)
        let prune = try applicationService.sessionPruneMetadata(sessionID: sessionID)
        return success(sessionMetadataPayload(
          summary,
          extensionFields: extensionFields,
          snapshot: published,
          prune: prune
        ))
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
        events.publish(SessionCatalogChanged())
        return success(["accepted": true])
      case "session.delete.cancel":
        let operationID = params["operationId"] as? String ?? ""
        let cancelled = try applicationService.cancelPendingDelete(operationID: operationID)
        return success(operationPayload(
          kind: "delete",
          status: cancelled ? "cancelled" : "completed",
          sessionID: cancelled ? operationID : nil,
          target: nil,
          operationID: operationID
        ))
      case "session.prune.preview":
        guard let target = (params["targetResolutionSeconds"] as? NSNumber)?.intValue,
              (1 ... 300).contains(target)
        else {
          return failure(id, .invalidRequest, "Prune resolution must be 1 through 300 seconds")
        }
        let scope = params["scope"] as? String ?? (params["sessionId"] == nil ? "all" : "one")
        guard scope == "one" || scope == "all" else {
          return failure(id, .invalidRequest, "scope must be one or all")
        }
        let requestedSessionID = params["sessionId"] as? String
        guard scope == "all" || (requestedSessionID?.isEmpty == false) else {
          return failure(id, .invalidRequest, "sessionId is required for one-session prune preview")
        }
        let sessionIDs = try pruneSessionIDs(scope: scope, sessionID: requestedSessionID)
        let previous = try applicationService.maintenanceJob().flatMap { value in
          value.kind == "prune_preview" && value.scope == scope && value.sessionID == requestedSessionID &&
            value.targetResolutionSeconds == Int64(target) && value.state == "pending" ? value : nil
        }
        let workIDs: [String] = if let last = previous?.lastSessionID,
                                   let index = sessionIDs.firstIndex(of: last)
        {
          Array(sessionIDs.dropFirst(index + 1))
        } else {
          sessionIDs
        }
        let operationID = "prunePreview-\(UUID().uuidString)"
        var job: SessionMaintenanceJobRecord
        if let previous {
          job = SessionMaintenanceJobRecord(
            operationID: previous.operationID, kind: previous.kind, scope: previous.scope, state: "running",
            sessionID: previous.sessionID, targetResolutionSeconds: previous.targetResolutionSeconds,
            upperSessionID: sessionIDs.last, lastSessionID: previous.lastSessionID,
            completedSessions: previous.completedSessions, skippedSessions: previous.skippedSessions,
            retainedSamples: previous.retainedSamples, reclaimableBytes: previous.reclaimableBytes,
            error: previous.error
          )
        } else {
          job = SessionMaintenanceJobRecord(
            operationID: operationID, kind: "prune_preview", scope: scope, state: "running",
            sessionID: requestedSessionID, targetResolutionSeconds: Int64(target),
            upperSessionID: sessionIDs.last, lastSessionID: nil, completedSessions: 0,
            skippedSessions: 0, retainedSamples: 0, reclaimableBytes: 0, error: nil
          )
        }
        try applicationService.saveMaintenanceJob(job)
        var recorded: Int64 = 0
        var stored: Int64 = 0
        var reclaimable: Int64 = 0
        var skipped = true
        var skipReason: String?
        for sessionID in workIDs {
          if try applicationService.maintenanceJob()?.state == "cancel_requested" { break }
          let value = try applicationService.prunePreview(sessionID: sessionID, targetResolutionSeconds: Int64(target))
          job = SessionMaintenanceJobRecord(
            operationID: job.operationID, kind: job.kind, scope: job.scope, state: job.state,
            sessionID: sessionID, targetResolutionSeconds: job.targetResolutionSeconds,
            upperSessionID: job.upperSessionID, lastSessionID: sessionID,
            completedSessions: job.completedSessions + 1,
            skippedSessions: job.skippedSessions + (value.skipped ? 1 : 0),
            retainedSamples: job.retainedSamples + value.estimatedStoredSampleCount,
            reclaimableBytes: job.reclaimableBytes + (value.estimatedReclaimableBytes ?? 0), error: nil
          )
          try applicationService.saveMaintenanceJob(job)
          recorded += value.recordedSampleCount
          stored += value.estimatedStoredSampleCount
          reclaimable += value.estimatedReclaimableBytes ?? 0
          skipped = skipped && value.skipped
          if value.skipReason != nil { skipReason = value.skipReason }
        }
        let terminal = try (applicationService.maintenanceJob()?.state == "cancel_requested") ? "cancelled" : "completed"
        job = SessionMaintenanceJobRecord(
          operationID: job.operationID, kind: job.kind, scope: job.scope, state: terminal,
          sessionID: job.sessionID, targetResolutionSeconds: job.targetResolutionSeconds,
          upperSessionID: job.upperSessionID, lastSessionID: job.lastSessionID,
          completedSessions: job.completedSessions, skippedSessions: job.skippedSessions,
          retainedSamples: job.retainedSamples, reclaimableBytes: job.reclaimableBytes, error: nil
        )
        try applicationService.saveMaintenanceJob(job)
        let operation = maintenanceJobPayload(job)
        return success(["operation": operation, "recordedSampleCount": recorded,
                        "estimatedStoredSampleCount": stored,
                        "estimatedReclaimableBytes": reclaimable,
                        "skipped": skipped, "skipReason": skipReason.map { $0 as Any } ?? NSNull()])
      case "session.prune.start":
        guard let target = (params["targetResolutionSeconds"] as? NSNumber)?.intValue,
              (1 ... 300).contains(target)
        else {
          return failure(id, .invalidRequest, "Prune resolution must be 1 through 300 seconds")
        }
        let scope = params["scope"] as? String ?? (params["sessionId"] == nil ? "all" : "one")
        guard scope == "one" || scope == "all" else {
          return failure(id, .invalidRequest, "scope must be one or all")
        }
        let requestedSessionID = params["sessionId"] as? String
        guard scope == "all" || (requestedSessionID?.isEmpty == false) else {
          return failure(id, .invalidRequest, "sessionId is required for one-session prune")
        }
        let sessionIDs = try pruneSessionIDs(scope: scope, sessionID: requestedSessionID)
        let previous = try applicationService.maintenanceJob().flatMap { value in
          value.kind == "prune" && value.scope == scope && value.sessionID == requestedSessionID &&
            value.targetResolutionSeconds == Int64(target) && value.state == "pending" ? value : nil
        }
        let workIDs: [String] = if let last = previous?.lastSessionID,
                                   let index = sessionIDs.firstIndex(of: last)
        {
          Array(sessionIDs.dropFirst(index + 1))
        } else {
          sessionIDs
        }
        let operationID = "prune-\(UUID().uuidString)"
        var job: SessionMaintenanceJobRecord
        if let previous {
          job = SessionMaintenanceJobRecord(
            operationID: previous.operationID, kind: previous.kind, scope: previous.scope, state: "running",
            sessionID: previous.sessionID, targetResolutionSeconds: previous.targetResolutionSeconds,
            upperSessionID: sessionIDs.last, lastSessionID: previous.lastSessionID,
            completedSessions: previous.completedSessions, skippedSessions: previous.skippedSessions,
            retainedSamples: previous.retainedSamples, reclaimableBytes: previous.reclaimableBytes,
            error: previous.error
          )
        } else {
          job = SessionMaintenanceJobRecord(
            operationID: operationID, kind: "prune", scope: scope, state: "running",
            sessionID: requestedSessionID, targetResolutionSeconds: Int64(target),
            upperSessionID: sessionIDs.last, lastSessionID: nil, completedSessions: 0,
            skippedSessions: 0, retainedSamples: 0, reclaimableBytes: 0, error: nil
          )
        }
        try applicationService.saveMaintenanceJob(job)
        for sessionID in workIDs {
          if try applicationService.maintenanceJob()?.state == "cancel_requested" { break }
          let value = try applicationService.pruneSession(sessionID: sessionID, targetResolutionSeconds: Int64(target))
          job = SessionMaintenanceJobRecord(
            operationID: job.operationID, kind: job.kind, scope: job.scope, state: job.state,
            sessionID: sessionID, targetResolutionSeconds: job.targetResolutionSeconds,
            upperSessionID: job.upperSessionID, lastSessionID: sessionID,
            completedSessions: job.completedSessions + 1,
            skippedSessions: job.skippedSessions + (value.skipped ? 1 : 0),
            retainedSamples: job.retainedSamples + value.estimatedStoredSampleCount,
            reclaimableBytes: job.reclaimableBytes + (value.estimatedReclaimableBytes ?? 0), error: nil
          )
          try applicationService.saveMaintenanceJob(job)
        }
        let terminal = try (applicationService.maintenanceJob()?.state == "cancel_requested") ? "cancelled" : "completed"
        job = SessionMaintenanceJobRecord(
          operationID: job.operationID, kind: job.kind, scope: job.scope, state: terminal,
          sessionID: job.sessionID, targetResolutionSeconds: job.targetResolutionSeconds,
          upperSessionID: job.upperSessionID, lastSessionID: job.lastSessionID,
          completedSessions: job.completedSessions, skippedSessions: job.skippedSessions,
          retainedSamples: job.retainedSamples, reclaimableBytes: job.reclaimableBytes, error: nil
        )
        try applicationService.saveMaintenanceJob(job)
        events.publish(SessionCatalogChanged())
        return success(maintenanceJobPayload(job))
      case "session.prune.cancel":
        let operationID = params["operationId"] as? String ?? ""
        if let job = try applicationService.cancelMaintenanceJob(operationID: operationID) {
          return success(maintenanceJobPayload(job))
        }
        return success(operationPayload(
          kind: "prune",
          status: "cancelled",
          sessionID: nil,
          target: nil,
          operationID: operationID
        ))
      default:
        return failure(id, .invalidRequest, "Unknown method")
      }
    } catch let error as ApplicationError {
      return applicationFailure(id, error)
    } catch let error as ProductStoreError {
      return productFailure(id, error)
    } catch let error as SessionStoreError {
      return sessionFailure(id, error)
    } catch let error as RefloatMutationError {
      return error == .invalidState
        ? failure(id, .invalidState, "Refloat control is not permitted in the current state")
        : failure(id, .unavailable, "Refloat control confirmation was unavailable")
    } catch let error as RefloatConfigurationServiceError {
      switch error {
      case .conflict:
        return failure(id, .conflict, "Configuration changed or verification did not match")
      case .unavailable:
        return failure(id, .unavailable, "Configuration request did not complete")
      case .invalidState, .staleResponse, .unauditedSchema, .malformedResponse:
        return failure(id, .invalidState, "Configuration is not available in the current state")
      }
    } catch {
      return failure(id, .internal, "Backend operation failed")
    }
  }

  fileprivate func stateChanged(catalogChanged: Bool = false) -> String {
    revisionLock.withLock {
      revision += 1
      if catalogChanged { sessionCatalogRevision += 1 }
    }
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
    var connection: Any = NSNull()
    if let active {
      var value = connectionPayload(active)
      if let firmware, let packageVersion {
        let configuration = applicationService.refloatConfigurationSummarySnapshot()
        value["firmware"] = [
          "vescVersion": "\(firmware.major).\(String(format: "%02d", firmware.minor))",
          "packageName": "refloat",
          "packageVersion": packageVersion,
          "hardwareName": firmware.hardwareName.map { $0 as Any } ?? NSNull(),
          "accessMode": configuration.writable ? "readWrite" : "readOnly",
          "supported": true,
        ]
      } else {
        value["firmware"] = NSNull()
      }
      value["savedDeviceId"] = active.savedDevice?.savedDeviceID ?? NSNull()
      value["displayName"] = active.savedDevice?.displayName ?? active.name ??
        active.deviceID.value
      value["activeSession"] = active.activeSession.map(sessionPayload) ?? NSNull()
      value["latestSample"] = active.latestSample.map(liveSamplePayload) ?? NSNull()
      value["latestLocation"] = active.latestLocation.map(liveLocationPayload) ?? NSNull()
      value["currentLimits"] = active.currentLimits.map(currentLimitsPayload) ?? NSNull()
      value["refloatConfiguration"] = packageVersion == nil
        ? NSNull()
        : refloatConfigurationSummaryPayload(applicationService.refloatConfigurationSummarySnapshot())
      connection = value
    }
    return [
      "revision": revisionLock.withLock { revision },
      "bluetooth": bluetoothPayload(discovery.bluetoothState, scanning: discovery.activeScanID != nil),
      "discoveredDevices": Array(discovery.knownDevices.prefix(50)).map(devicePayload),
      "savedDevices": Array((try? applicationService.savedDevices())?.prefix(50) ?? []).map(savedDevicePayload),
      "activeConnection": connection,
      "sessionCatalogRevision": revisionLock.withLock { sessionCatalogRevision },
      "settings": (try? applicationService.settings()).map(settingsPayload) ?? [:],
      "location": locationPayload(applicationService.locationState()),
      "sessionOperation": (try? applicationService.maintenanceJob())
        .flatMap { $0.map(maintenanceJobPayload) } ?? NSNull(),
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

  private func refloatConfigurationSummaryPayload(_ value: RefloatConfigurationSummary) -> [String: Any] {
    [
      "availability": value.availability.rawValue,
      "writable": value.writable,
      "packageVersion": value.packageVersion.map { $0 as Any } ?? NSNull(),
      "schemaFingerprint": value.schemaFingerprint.map { $0 as Any } ?? NSNull(),
      "wireSignature": value.wireSignature.map { Int64($0) as Any } ?? NSNull(),
      "currentLoaded": value.currentLoaded,
      "defaultLoaded": value.defaultLoaded,
      "pendingOperation": value.pendingOperation.map { $0.rawValue as Any } ?? NSNull(),
    ]
  }

  private func refloatConfigurationIdentityPayload() -> [String: Any]? {
    let summary = applicationService.refloatConfigurationSummarySnapshot()
    guard let version = summary.packageVersion,
          let fingerprint = summary.schemaFingerprint,
          let signature = summary.wireSignature
    else { return nil }
    return [
      "packageName": "refloat", "packageVersion": version,
      "schemaFingerprint": fingerprint, "wireSignature": Int64(signature),
    ]
  }

  private func refloatConfigurationIdentity(_ raw: Any?) -> RefloatConfigurationIdentityRecord? {
    guard let value = raw as? [String: Any], value.count == 4,
          value["packageName"] as? String == "refloat",
          let version = value["packageVersion"] as? String,
          !version.isEmpty, version.utf8.count <= 128,
          let fingerprint = value["schemaFingerprint"] as? String,
          fingerprint.range(of: "^[0-9a-f]{64}$", options: .regularExpression) != nil,
          let signature = value["wireSignature"] as? NSNumber,
          CFGetTypeID(signature) != CFBooleanGetTypeID(),
          signature.doubleValue.rounded() == signature.doubleValue,
          (0 ... Double(UInt32.max)).contains(signature.doubleValue)
    else { return nil }
    return .init(
      packageName: "refloat", packageVersion: version,
      schemaFingerprint: fingerprint, wireSignature: signature.uint32Value
    )
  }

  private func boundedRefloatText(_ raw: Any?, maximumBytes: Int) -> String? {
    guard let value = raw as? String else { return nil }
    let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty, trimmed.utf8.count <= maximumBytes else { return nil }
    return trimmed
  }

  private func refloatTunePayload(
    _ record: RefloatConfigurationRecord, includeValues: Bool
  ) -> [String: Any] {
    var result: [String: Any] = [
      "tuneId": record.configurationID, "revision": record.revision,
      "name": record.name, "description": record.description.map { $0 as Any } ?? NSNull(),
      "identity": [
        "packageName": record.identity.packageName,
        "packageVersion": record.identity.packageVersion,
        "schemaFingerprint": record.identity.schemaFingerprint,
        "wireSignature": Int64(record.identity.wireSignature),
      ],
      "createdAt": ISO8601DateFormatter().string(from: record.createdAt),
      "updatedAt": ISO8601DateFormatter().string(from: record.updatedAt),
    ]
    if includeValues,
       let raw = try? JSONSerialization.jsonObject(with: Data(record.valuesJSON.utf8)) as? [String: Any],
       let values = refloatConfigurationValues(raw)
    {
      result["values"] = values.mapValues(refloatConfigurationValuePayload)
    }
    return result
  }

  private func refloatConfigurationSchemaPage(cursor: Any?) -> [String: Any]? {
    guard let schema = applicationService.refloatConfigurationSchemaSnapshot(),
          let identity = refloatConfigurationIdentityPayload()
    else { return nil }
    let pageIndex: Int
    if cursor == nil || cursor is NSNull {
      pageIndex = 0
    } else {
      guard let raw = cursor as? String, raw.utf8.count <= 512,
            let data = Data(base64Encoded: raw), data.count <= 384,
            let decoded = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
            decoded["fingerprint"] as? String == schema.fingerprint,
            let page = decoded["page"] as? Int, page >= 0
      else { return nil }
      pageIndex = page
    }
    let byName = Dictionary(uniqueKeysWithValues: schema.fields.map { ($0.name, $0) })
    var pages: [(String, String, [RefloatConfigurationField])] = []
    for group in schema.groups {
      for subgroup in group.subgroups {
        let fields = subgroup.fields.compactMap { byName[$0] }
        for offset in stride(from: 0, to: fields.count, by: 32) {
          pages.append((group.name, subgroup.name, Array(fields[offset ..< min(offset + 32, fields.count)])))
        }
      }
    }
    guard pageIndex < pages.count else { return nil }
    let page = pages[pageIndex]
    let next = pageIndex + 1 < pages.count
      ? Self.encodeSessionCatalogCursor(["fingerprint": schema.fingerprint, "page": pageIndex + 1])
      : nil
    return [
      "identity": identity,
      "group": page.0,
      "subgroup": page.1,
      "fields": page.2.map(refloatConfigurationFieldPayload),
      "nextCursor": next.map { $0 as Any } ?? NSNull(),
    ]
  }

  private func refloatConfigurationFieldPayload(_ field: RefloatConfigurationField) -> [String: Any] {
    let type = switch field.type {
    case .double: "double"
    case .integer: "integer"
    case .string: "string"
    case .enumeration: "enum"
    case .boolean: "boolean"
    case .bitfield: "bitfield"
    }
    return [
      "name": field.name, "label": field.longName, "type": type,
      "defaultValue": refloatConfigurationValuePayload(field.defaultValue),
      "minimum": field.minimum.map { $0 as Any } ?? NSNull(),
      "maximum": field.maximum.map { $0 as Any } ?? NSNull(),
      "step": field.step.map { $0 as Any } ?? NSNull(),
      "precision": field.precision.map { $0 as Any } ?? NSNull(),
      "suffix": field.suffix, "choices": field.choices, "description": field.description,
    ]
  }

  private func refloatConfigurationValuePayload(_ value: RefloatConfigurationValue) -> [String: Any] {
    switch value {
    case let .double(value): ["type": "double", "value": value]
    case let .integer(value): ["type": "integer", "value": value]
    case let .boolean(value): ["type": "boolean", "value": value]
    case let .string(value): ["type": "string", "value": value]
    }
  }

  private func refloatConfigurationValuesPayload(_ value: RefloatConfigurationValues) -> [String: Any] {
    [
      "identity": refloatConfigurationIdentityPayload() ?? [:],
      "revision": value.revision,
      "source": value.source,
      "values": value.values.mapValues(refloatConfigurationValuePayload),
    ]
  }

  private func refloatConfigurationValues(
    _ raw: [String: Any]
  ) -> [String: RefloatConfigurationValue]? {
    guard raw.count <= RefloatConfigurationLimits.fields,
          let data = try? JSONSerialization.data(withJSONObject: raw), data.count <= 48 * 1024
    else { return nil }
    var result: [String: RefloatConfigurationValue] = [:]
    for (name, rawValue) in raw {
      guard !name.isEmpty, name.utf8.count <= RefloatConfigurationLimits.nameBytes,
            let typed = rawValue as? [String: Any], typed.count == 2,
            let type = typed["type"] as? String, typed.keys.contains("value")
      else { return nil }
      switch type {
      case "double":
        guard let number = typed["value"] as? NSNumber,
              CFGetTypeID(number) != CFBooleanGetTypeID(), number.doubleValue.isFinite
        else { return nil }
        result[name] = .double(number.doubleValue)
      case "integer":
        guard let number = typed["value"] as? NSNumber,
              CFGetTypeID(number) != CFBooleanGetTypeID(),
              number.doubleValue.rounded() == number.doubleValue,
              abs(number.doubleValue) <= Double(9_007_199_254_740_991 as Int64)
        else { return nil }
        result[name] = .integer(number.int64Value)
      case "boolean":
        guard let value = typed["value"] as? Bool else { return nil }
        result[name] = .boolean(value)
      case "string":
        guard let value = typed["value"] as? String,
              value.utf8.count <= RefloatConfigurationLimits.stringValueBytes
        else { return nil }
        result[name] = .string(value)
      default:
        return nil
      }
    }
    return result
  }

  private func devicePayload(_ value: KnownDevice) -> [String: Any] {
    [
      "deviceId": value.deviceID.value,
      "name": value.name ?? "",
      "state": "discovered",
      "rssi": value.rssi.map { $0 as Any } ?? NSNull(),
      "lastSeenAt": ISO8601DateFormatter().string(from: value.lastSeenAt),
      "savedDeviceId": value.savedDeviceID ?? NSNull(),
    ]
  }

  private func connectionPayload(_ value: ActiveConnectionSnapshot) -> [String: Any] {
    [
      "deviceId": value.deviceID.value,
      "deviceKind": value.deviceKind?.rawValue ?? NSNull(),
      "name": value.savedDevice?.displayName ?? value.name ?? value.deviceID.value,
      "state": deviceState(value.state),
      "trackingMode": value.trackingMode.rawValue,
      "controllerConnection": value.controllerRoute.map(controllerRoutePayload) ?? NSNull(),
      "controllerCandidates": value.controllerCandidates.map(controllerCandidatePayload),
      "connectionIssue": value.connectionIssue.map(connectionIssuePayload) ?? NSNull(),
      "bms": bmsStatePayload(value.bms),
      "refloatControls": value.refloatControls.map(refloatControlsPayload) ?? NSNull(),
      "controllerActions": controllerActionsPayload(value.controllerActions),
    ]
  }

  private func refloatControlsPayload(_ value: RefloatControls) -> [String: Any] {
    [
      "status": value.status.rawValue,
      "writable": value.writable,
      "packageEnabled": value.packageEnabled ?? NSNull(),
      "flywheelEnabled": value.flywheelEnabled ?? NSNull(),
      "lightingAvailable": value.lightingAvailable,
      "ledsEnabled": value.ledsEnabled ?? NSNull(),
      "headlightsEnabled": value.headlightsEnabled ?? NSNull(),
      "fatalErrorClearAvailable": value.fatalErrorClearAvailable,
      "fatalErrorLatched": value.fatalErrorLatched ?? NSNull(),
      "fatalErrorActive": value.fatalErrorActive ?? NSNull(),
      "fatalErrorClearable": value.fatalErrorClearable ?? NSNull(),
      "pendingAction": value.pendingAction?.rawValue ?? NSNull(),
    ]
  }

  private func controllerActionsPayload(_ value: ControllerActions) -> [String: Any] {
    [
      "shutdownAvailable": value.shutdownAvailable,
      "pendingAction": value.pendingAction?.rawValue ?? NSNull(),
    ]
  }

  private func bmsConnectionPayload(_ value: BmsConnection) -> [String: Any] {
    switch value {
    case .direct: ["kind": "direct"]
    case let .can(canID): ["kind": "can", "canId": Int(canID)]
    }
  }

  private func bmsStatePayload(_ value: BmsState) -> [String: Any] {
    [
      "status": value.status.rawValue,
      "connection": value.connection.map(bmsConnectionPayload) ?? NSNull(),
      "candidates": value.candidates.prefix(16).map { candidate in
        [
          "connection": bmsConnectionPayload(candidate.connection),
          "hardwareName": candidate.hardwareName.map { $0 as Any } ?? NSNull(),
          "vescVersion": candidate.vescVersion,
        ]
      },
      "latestSnapshot": value.latestSnapshot.map(bmsSnapshotPayload) ?? NSNull(),
    ]
  }

  private func bmsSnapshotPayload(_ value: BmsSnapshot) -> [String: Any] {
    [
      "observedAt": ISO8601DateFormatter().string(from: value.observedAt),
      "totalVoltageV": value.totalVoltageV,
      "chargeVoltageV": value.chargeVoltageV,
      "inputCurrentA": value.inputCurrentA,
      "inputIcCurrentA": value.inputICCurrentA,
      "sessionAmpHoursAh": value.sessionAmpHoursAh,
      "sessionWattHoursWh": value.sessionWattHoursWh,
      "cellVoltagesV": Array(value.cellVoltagesV.prefix(50)),
      "balancing": Array(value.balancing.prefix(50)),
      "temperaturesC": Array(value.temperaturesC.prefix(50)),
      "icTemperatureC": value.icTemperatureC,
      "humiditySensorTemperatureC": value.humiditySensorTemperatureC,
      "humidityPercent": value.humidityPercent,
      "highestCellTemperatureC": value.highestCellTemperatureC,
      "stateOfCharge": value.stateOfCharge,
      "stateOfHealth": value.stateOfHealth,
      "reportedCanId": Int(value.reportedCanID),
      "lifetimeChargeAmpHoursAh": value.lifetimeChargeAmpHoursAh,
      "lifetimeChargeWattHoursWh": value.lifetimeChargeWattHoursWh,
      "lifetimeDischargeAmpHoursAh": value.lifetimeDischargeAmpHoursAh,
      "lifetimeDischargeWattHoursWh": value.lifetimeDischargeWattHoursWh,
      "pressurePa": value.pressurePa,
      "dataVersion": value.dataVersion.map { Int($0) as Any } ?? NSNull(),
      "status": value.status.flatMap { $0.utf8.count <= 40 ? $0 : nil } ?? NSNull(),
    ]
  }

  private func currentLimitsPayload(_ value: ControllerCurrentLimits) -> [String: Any] {
    [
      "motorCurrentMaxA": value.motorCurrentMaxA,
      "motorCurrentMinA": value.motorCurrentMinA,
      "batteryCurrentMaxA": value.batteryCurrentMaxA,
      "batteryCurrentMinA": value.batteryCurrentMinA,
      "batterySeriesCells": value.batterySeriesCells ?? NSNull(),
    ]
  }

  private func liveLocationPayload(_ value: LiveLocation) -> [String: Any] {
    [
      "latitude": value.latitude,
      "longitude": value.longitude,
      "accuracyM": value.accuracyM.map { $0 as Any } ?? NSNull(),
      "observedAt": ISO8601DateFormatter().string(from: value.observedAt),
    ]
  }

  private func controllerRoutePayload(_ value: ControllerRoute) -> [String: Any] {
    switch value {
    case .direct: ["kind": "direct"]
    case let .can(canID): ["kind": "can", "canId": Int(canID)]
    }
  }

  private func controllerCandidatePayload(_ value: ControllerCandidate) -> [String: Any] {
    [
      "canId": Int(value.canID),
      "hardwareName": value.hardwareName ?? NSNull(),
      "firmwareName": value.firmwareName ?? NSNull(),
      "vescVersion": value.vescVersion,
      "vescSupported": value.vescSupported,
    ]
  }

  private func connectionIssuePayload(_ value: ConnectionIssue) -> [String: Any] {
    ["code": value.code.rawValue, "message": value.message]
  }

  private func savedDevicePayload(_ value: SavedDeviceRecord) -> [String: Any] {
    [
      "savedDeviceId": value.savedDeviceID,
      "deviceHistoryId": value.deviceHistoryID,
      "revision": value.revision,
      "displayName": value.displayName,
      "customDisplayName": value.customDisplayName ?? NSNull(),
      "advertisedName": value.advertisedName ?? NSNull(),
      "bluetoothIdentifier": value.bluetoothIdentifier.map { $0 as Any } ?? NSNull(),
      "lastSeenAt": value.lastSeenAt.map(ISO8601DateFormatter().string) ?? NSNull(),
      "lastConnectedAt": value.lastConnectedAt.map(ISO8601DateFormatter().string) ?? NSNull(),
      "profileComplete": value.profileComplete,
      "summary": [
        "sessionCount": value.summary.sessionCount,
        "totalRidingElapsedMs": value.summary.totalRidingElapsedMs,
        "totalDistanceM": value.summary.totalDistanceM.map { $0 as Any } ?? NSNull(),
        "mostRecentSessionAt": value.summary.mostRecentSessionAt
          .map(ISO8601DateFormatter().string) ?? NSNull(),
      ],
    ]
  }

  private func liveSamplePayload(_ value: TelemetrySample) -> [String: Any] {
    [
      "sequence": value.sequence,
      "observedAt": ISO8601DateFormatter().string(from: value.observedAt),
      "slowObservedAt": value.slowObservedAt.map(ISO8601DateFormatter().string) ?? NSNull(),
      "elapsedMs": value.elapsedMs,
      "rideState": value.rideState.rawValue,
      "packageState": value.packageState?.rawValue ?? NSNull(),
      "packageMode": value.packageMode?.rawValue ?? NSNull(),
      "footpadState": value.footpadState?.rawValue ?? NSNull(),
      "stopCondition": value.stopCondition?.rawValue ?? NSNull(),
      "setpointAdjustment": value.setpointAdjustment?.rawValue ?? NSNull(),
      "wheelslip": value.wheelslip ?? NSNull(),
      "darkride": value.darkride ?? NSNull(),
      "charging": value.charging ?? NSNull(),
      "alertReason": value.alertReason?.rawValue ?? NSNull(),
      "firmwareFault": value.firmwareFault?.rawValue ?? NSNull(),
      "packVoltageV": value.packVoltageV ?? NSNull(),
      "dutyCycle": value.dutyCycle ?? NSNull(),
      "controllerTempC": value.controllerTempC ?? NSNull(),
      "motorTempC": value.motorTempC ?? NSNull(),
      "motorCurrentA": value.motorCurrentA ?? NSNull(),
      "batteryCurrentA": value.batteryCurrentA ?? NSNull(),
      "erpm": value.erpm ?? NSNull(),
      "speedKph": value.speedKph ?? NSNull(),
      "pitchDeg": value.pitchDeg ?? NSNull(),
      "balancePitchDeg": value.balancePitchDeg ?? NSNull(),
      "rollDeg": value.rollDeg ?? NSNull(),
      "directionalMotorCurrentA": value.directionalMotorCurrentA ?? NSNull(),
      "filteredMotorCurrentA": value.filteredMotorCurrentA ?? NSNull(),
      "requestedBalanceCurrentA": value.requestedBalanceCurrentA ?? NSNull(),
      "boosterCurrentA": value.boosterCurrentA ?? NSNull(),
      "fieldWeakeningCurrentA": value.fieldWeakeningCurrentA ?? NSNull(),
      "footpadAdc1V": value.footpadAdc1V ?? NSNull(),
      "footpadAdc2V": value.footpadAdc2V ?? NSNull(),
      "remoteInput": value.remoteInput ?? NSNull(),
      "setpointDeg": value.setpointDeg ?? NSNull(),
      "atrSetpointDeg": value.atrSetpointDeg ?? NSNull(),
      "brakeTiltSetpointDeg": value.brakeTiltSetpointDeg ?? NSNull(),
      "torqueTiltSetpointDeg": value.torqueTiltSetpointDeg ?? NSNull(),
      "turnTiltSetpointDeg": value.turnTiltSetpointDeg ?? NSNull(),
      "remoteSetpointDeg": value.remoteSetpointDeg ?? NSNull(),
      "atrAccelerationDifferenceErpmPerSecond":
        value.atrAccelerationDifferenceErpmPerSecond ?? NSNull(),
      "atrSpeedBoost": value.atrSpeedBoost ?? NSNull(),
      "distanceM": value.distanceM ?? NSNull(),
      "odometerM": value.odometerM ?? NSNull(),
      "ampHoursDrawnAh": value.ampHoursDrawnAh ?? NSNull(),
      "ampHoursChargedAh": value.ampHoursChargedAh ?? NSNull(),
      "wattHoursDrawnWh": value.wattHoursDrawnWh ?? NSNull(),
      "wattHoursChargedWh": value.wattHoursChargedWh ?? NSNull(),
      "batteryLevel": value.batteryLevel ?? NSNull(),
      "chargingCurrentA": value.chargingCurrentA ?? NSNull(),
      "chargingVoltageV": value.chargingVoltageV ?? NSNull(),
      "extensions": value.extensionFields,
    ]
  }

  private func sessionEventPayload(_ value: SessionEventRecord) throws -> [String: Any] {
    guard let details = try JSONSerialization.jsonObject(with: Data(value.detailsJSON.utf8))
      as? [String: Any],
      let eventValue = details["value"] as? String
    else { throw SessionStoreError.invalidRange }
    return ["elapsedMs": value.elapsedMs, "kind": value.kind, "value": eventValue]
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

  private func sessionMetadataPayload(
    _ value: SessionSummaryRecord,
    extensionFields: [String],
    snapshot: SessionPublishedSnapshotRecord,
    prune: SessionPruneMetadataRecord
  ) -> [String: Any] {
    let fields: [[String: Any]] = Self.numericSeriesFields.map { ["name": $0, "source": "stable"] }
      + extensionFields.map { ["name": $0, "source": "extension"] }
    return [
      "summary": sessionPayload(value),
      "fields": fields,
      "eventCount": value.eventCount,
      "recordedSampleCount": snapshot.recordedSampleCount,
      "storedSampleCount": snapshot.storedSampleCount,
      "prune": [
        "targetResolutionSeconds": prune.targetResolutionSeconds.map { $0 as Any } ?? NSNull(),
        "recordedSampleCount": prune.recordedSampleCount,
        "storedSampleCount": prune.storedSampleCount,
        "policy": prune.policy.map { $0 as Any } ?? NSNull(),
      ],
      "snapshot": [
        "historyRevision": snapshot.historyRevision,
        "committedSampleSequence": snapshot.committedSampleSequence,
        "committedEventSequence": snapshot.committedEventSequence,
        "committedElapsedMs": snapshot.committedElapsedMs,
        "committedLocationSequence": snapshot.committedLocationSequence,
        "committedLocationElapsedMs": snapshot.committedLocationElapsedMs,
      ],
      "location": [
        "available": snapshot.recordedLocationCount > 0,
        "count": snapshot.recordedLocationCount,
        "lastSequence": snapshot.committedLocationSequence,
        "lastElapsedMs": snapshot.committedLocationElapsedMs,
        "status": snapshot.locationStatus,
      ],
    ]
  }

  private func sessionDataPayload(_ params: [String: Any], timing: SessionDataTiming) throws -> [String: Any]? {
    guard let kind = params["kind"] as? String,
          let sessionID = params["sessionId"] as? String,
          let range = params["range"] as? [String: Any],
          let start = (range["startElapsedMs"] as? NSNumber)?.int64Value,
          let end = (range["endElapsedMs"] as? NSNumber)?.int64Value,
          let snapshot = params["snapshot"] as? [String: Any]
    else { return nil }
    timing.kind = kind
    let fields = (params["fields"] as? [String] ?? []).filter { !$0.isEmpty }
    guard start >= 0, end >= start, fields.count <= 16,
          kind == "events" || kind == "locations" || (!fields.isEmpty && Set(fields).count == fields.count)
    else { return nil }
    let metadataStarted = DispatchTime.now().uptimeNanoseconds
    let published = try applicationService.sessionSnapshot(sessionID: sessionID)
    guard let snapshotRevision = snapshot["historyRevision"] as? NSNumber,
          let sampleSequence = snapshot["committedSampleSequence"] as? NSNumber,
          let eventSequence = snapshot["committedEventSequence"] as? NSNumber,
          let committedElapsed = snapshot["committedElapsedMs"] as? NSNumber,
          let locationSequence = snapshot["committedLocationSequence"] as? NSNumber,
          let locationElapsed = snapshot["committedLocationElapsedMs"] as? NSNumber
    else { return nil }
    guard snapshotRevision.int64Value == published.historyRevision,
          sampleSequence.int64Value == published.committedSampleSequence,
          eventSequence.int64Value == published.committedEventSequence,
          committedElapsed.int64Value == published.committedElapsedMs,
          locationSequence.int64Value == published.committedLocationSequence,
          locationElapsed.int64Value == published.committedLocationElapsedMs
    else { throw SessionStoreError.invalidRange }
    let echoedSnapshot: [String: Any] = [
      "historyRevision": snapshotRevision.int64Value,
      "committedSampleSequence": sampleSequence.int64Value,
      "committedEventSequence": eventSequence.int64Value,
      "committedElapsedMs": committedElapsed.int64Value,
      "committedLocationSequence": locationSequence.int64Value,
      "committedLocationElapsedMs": locationElapsed.int64Value,
    ]
    let extensionFields = try Set(applicationService.sessionExtensionFields(sessionID: sessionID))
    timing.metadataMs = Double(DispatchTime.now().uptimeNanoseconds - metadataStarted) / 1_000_000
    let descriptors = fields.map { field in
      [
        "name": field,
        "source": extensionFields.contains(field) ? "extension" : "stable",
      ]
    }
    if kind == "series" {
      let buckets = min(max((params["buckets"] as? NSNumber)?.intValue ?? 1, 1), 1_024)
      let gap = params["gapPolicy"] as? [String: Any]
      let threshold = (gap?["thresholdSeconds"] as? NSNumber)?.doubleValue
      if let threshold, !threshold.isFinite || threshold < 0.1 || threshold > 3_600 {
        throw SessionStoreError.invalidRange
      }
      guard gap != nil, gap?["thresholdSeconds"] != nil else { throw SessionStoreError.invalidRange }
      let configurationStarted = DispatchTime.now().uptimeNanoseconds
      let historyConfiguration = try applicationService.sessionHistoryConfiguration(sessionID: sessionID)
      timing.configurationMs = Double(DispatchTime.now().uptimeNanoseconds - configurationStarted) / 1_000_000
      let streamed = try applicationService.reduceSessionSeries(
        sessionID: sessionID, startElapsedMs: start, endElapsedMs: end, fields: fields,
        buckets: buckets,
        gapPolicy: SessionGapPolicyRecord(thresholdSeconds: threshold),
        expectedPersistenceMs: 1_000,
        expectedRidingPersistenceMs: historyConfiguration.expectedRidingPersistenceMs,
        expectedIdlePersistenceMs: historyConfiguration.expectedIdlePersistenceMs,
        retainedResolutionSeconds: historyConfiguration.retainedResolutionSeconds,
        publishedSampleSequence: sampleSequence.int64Value
      )
      // The bounded reducer now runs during the SQLite cursor walk. The
      // native reader reports the cursor and accumulator portions separately;
      // neither stage retains a second full-row array.
      timing.sourceMs = streamed.sourceMs
      timing.reducerMs = streamed.reducerMs
      timing.sourceRows = streamed.sourceRows
      let columns = Dictionary(uniqueKeysWithValues: fields.map { field in
        let column = streamed.columns[field]!
        let values: [Any] = column.values.map { $0 as Any? ?? NSNull() }
        return (field, [
          "elapsedMs": column.elapsedMs,
          "sequence": column.sequence,
          "values": values,
          "generations": column.generations,
        ])
      })
      let echoedGap: [String: Any] = ["thresholdSeconds": threshold as Any? ?? NSNull()]
      return ["kind": kind, "sessionId": sessionID, "snapshot": echoedSnapshot, "range": range,
              "gapPolicy": echoedGap, "fields": descriptors, "columns": columns]
    }
    if kind == "samples" {
      let requestedLimit = min(max((params["limit"] as? NSNumber)?.intValue ?? 2_000, 1), 2_000)
      let seed = SessionHistoryCursor(
        sessionID: sessionID, snapshot: published,
        range: SessionElapsedRange(startElapsedMs: start, endElapsedMs: end), elapsedMs: start, sequence: 0
      )
      let decodedCursor: SessionHistoryCursor?
      if let raw = params["cursor"] as? String {
        let cursor = try Self.decodeSessionCursor(raw)
        guard cursor.sessionID == sessionID, cursor.snapshot == published,
              cursor.range.startElapsedMs == start, cursor.range.endElapsedMs == end
        else { throw SessionStoreError.invalidCursor }
        decodedCursor = cursor
      } else {
        decodedCursor = nil
      }
      let source = try applicationService.querySessionSeriesSource(
        sessionID: sessionID, startElapsedMs: start, endElapsedMs: end, fields: fields,
        maxRows: 200_000, publishedSampleSequence: sampleSequence.int64Value
      )
      let available = source.filter { point in
        guard let decodedCursor else { return true }
        return point.elapsedMs > decodedCursor.elapsedMs ||
          (point.elapsedMs == decodedCursor.elapsedMs && point.sequence > decodedCursor.sequence)
      }
      let points = Array(available.prefix(requestedLimit))
      let nextCursor = points.last.flatMap { last in
        points.count < available.count ? try? Self.encodeSessionCursor(SessionHistoryCursor(
          sessionID: seed.sessionID, snapshot: seed.snapshot, range: seed.range,
          elapsedMs: last.elapsedMs, sequence: last.sequence
        )) : nil
      }
      let values = Dictionary(uniqueKeysWithValues: fields.map { field in
        (field, points.map { point -> Any in point.values[field].flatMap { $0 } as Any? ?? NSNull() })
      })
      return ["kind": kind, "sessionId": sessionID, "snapshot": echoedSnapshot, "range": range,
              "mode": "window", "fields": descriptors, "elapsedMs": points.map(\.elapsedMs),
              "sequence": points.map(\.sequence), "values": values, "nextCursor": nextCursor as Any? ?? NSNull()]
    }
    if kind == "locations" {
      let requestedLimit = min(max((params["limit"] as? NSNumber)?.intValue ?? 1, 1), 2_000)
      let page = try applicationService.querySessionLocations(
        sessionID: sessionID,
        startElapsedMs: start,
        endElapsedMs: end,
        limit: requestedLimit,
        cursor: params["cursor"] as? String,
        publishedLocationSequence: published.committedLocationSequence
      )
      return [
        "kind": kind, "sessionId": sessionID, "snapshot": echoedSnapshot, "range": range,
        "points": page.points.map { point in
          ["locationSequence": point.locationSequence, "elapsedMs": point.elapsedMs,
           "latitude": point.latitude, "longitude": point.longitude,
           "accuracyM": point.accuracyM ?? NSNull(),
           "observedAt": ISO8601DateFormatter().string(from: point.observedAt)]
        },
        "gaps": page.gaps.map { [
          "startElapsedMs": $0.startElapsedMs,
          "endElapsedMs": $0.endElapsedMs,
          "reason": $0.reason,
        ] },
        "nextCursor": page.nextCursor as Any? ?? NSNull(),
        "locationBoundary": ["lastSequence": page.snapshot.committedLocationSequence,
                             "lastElapsedMs": page.snapshot.committedLocationElapsedMs],
      ]
    }
    guard kind == "events" else { return nil }
    let requestedLimit = min(max((params["limit"] as? NSNumber)?.intValue ?? 500, 1), 500)
    let seed = SessionHistoryCursor(
      sessionID: sessionID, snapshot: published,
      range: SessionElapsedRange(startElapsedMs: start, endElapsedMs: end), elapsedMs: start, sequence: 0
    )
    let decodedCursor: SessionHistoryCursor?
    if let raw = params["cursor"] as? String {
      let cursor = try Self.decodeSessionCursor(raw)
      guard cursor.sessionID == sessionID, cursor.snapshot == published,
            cursor.range.startElapsedMs == start, cursor.range.endElapsedMs == end
      else { throw SessionStoreError.invalidCursor }
      decodedCursor = cursor
    } else {
      decodedCursor = nil
    }
    let allEvents = try applicationService.sessionEvents(
      sessionID: sessionID, startElapsedMs: start, endElapsedMs: end,
      publishedEventSequence: eventSequence.int64Value
    ).sorted { left, right in
      let leftSequence = Int64(left.eventID) ?? 0
      let rightSequence = Int64(right.eventID) ?? 0
      return left.elapsedMs < right.elapsedMs ||
        (left.elapsedMs == right.elapsedMs && leftSequence < rightSequence)
    }
    let pageEvents = allEvents.filter { event in
      guard let decodedCursor else { return true }
      let sequence = Int64(event.eventID) ?? 0
      return event.elapsedMs > decodedCursor.elapsedMs ||
        (event.elapsedMs == decodedCursor.elapsedMs && sequence > decodedCursor.sequence)
    }
    let events = Array(pageEvents.prefix(requestedLimit))
    let values = events
      .map {
        (try? JSONSerialization.jsonObject(with: Data(
          $0.detailsJSON.utf8
        )) as? [String: Any])?["value"] as? String ?? ""
      }
    let next: String?
    if let last = events.last, events.count < pageEvents.count {
      let cursor = SessionHistoryCursor(
        sessionID: seed.sessionID, snapshot: seed.snapshot, range: seed.range,
        elapsedMs: last.elapsedMs, sequence: Int64(last.eventID) ?? 0
      )
      next = try Self.encodeSessionCursor(cursor)
    } else {
      next = nil
    }
    return ["kind": kind, "sessionId": sessionID, "snapshot": echoedSnapshot, "range": range,
            "page": ["elapsedMs": events.map(\.elapsedMs), "sequence": events.map { Int64($0.eventID) ?? 0 },
                     "sourceSampleSequence": events.map { $0.sourceSampleSequence ?? 0 },
                     "kinds": events.map(\.kind), "values": values],
            "nextCursor": next as Any? ?? NSNull()]
  }

  private static func decodeSessionCursor(_ raw: String) throws -> SessionHistoryCursor {
    guard let data = Data(base64Encoded: raw), data.count <= 4_096,
          let cursor = try? JSONDecoder().decode(SessionHistoryCursor.self, from: data),
          cursor.version == 1
    else { throw SessionStoreError.invalidCursor }
    return cursor
  }

  private static func encodeSessionCursor(_ cursor: SessionHistoryCursor) throws -> String {
    let data = try JSONEncoder().encode(cursor)
    guard data.count <= 4_096 else { throw SessionStoreError.invalidCursor }
    return data.base64EncodedString()
  }

  private func operationPayload(
    kind: String,
    status: String,
    sessionID: String?,
    target: Int?,
    operationID: String? = nil,
    completedSessions: Int? = nil,
    eligibleSessions: Int? = nil
  ) -> [String: Any] {
    [
      "operationId": operationID ?? "\(kind)-\(revision)",
      "kind": kind,
      "status": status,
      "sessionId": sessionID ?? NSNull(),
      "scope": sessionID == nil ? "all" : "one",
      "targetResolutionSeconds": target ?? NSNull(),
      "completedSessions": completedSessions ?? (status == "completed" ? 1 : 0),
      "eligibleSessions": eligibleSessions ?? 1,
      "error": NSNull(),
    ]
  }

  private func maintenanceJobPayload(_ job: SessionMaintenanceJobRecord) -> [String: Any] {
    [
      "operationId": job.operationID,
      "kind": job.kind == "prune_preview" ? "prunePreview" : "prune",
      "status": job.state == "cancel_requested" ? "cancelling" : job.state,
      "sessionId": job.sessionID ?? NSNull(),
      "scope": job.scope,
      "targetResolutionSeconds": job.targetResolutionSeconds,
      "completedSessions": job.completedSessions,
      "eligibleSessions": job.upperSessionID == nil ? NSNull() : job.completedSessions,
      "error": job.error ?? NSNull(),
    ]
  }

  private func pruneSessionIDs(scope: String, sessionID: String?) throws -> [String] {
    if scope == "one" {
      guard let sessionID, !sessionID.isEmpty else { throw SessionStoreError.sessionNotFound }
      return [sessionID]
    }
    var cursor: String?
    var ids: [String] = []
    repeat {
      let page = try applicationService.sessions(SessionListQuery(
        cursor: cursor, limit: 100, state: "completed", integrity: "complete",
        sortField: "startedAt", sortDescending: true
      ))
      ids.append(contentsOf: page.items.map(\.sessionID))
      cursor = page.nextCursor
    } while cursor != nil
    return ids
  }

  private static func gzip(_ data: Data, timing: SessionDataTiming? = nil) -> Data? {
    let started = DispatchTime.now().uptimeNanoseconds
    var stream = z_stream()
    guard deflateInit2_(
      &stream,
      Z_DEFAULT_COMPRESSION,
      Z_DEFLATED,
      15 + 16,
      8,
      Z_DEFAULT_STRATEGY,
      ZLIB_VERSION,
      Int32(MemoryLayout<z_stream>.size)
    ) == Z_OK else { return nil }
    defer {
      deflateEnd(&stream)
      if let timing {
        timing.gzipMs = Double(DispatchTime.now().uptimeNanoseconds - started) / 1_000_000
      }
    }
    var output = Data(count: max(256, data.count + data.count / 100 + 256))
    let result = data.withUnsafeBytes { inputBuffer in
      output.withUnsafeMutableBytes { outputBuffer in
        stream.next_in = UnsafeMutablePointer(mutating: inputBuffer.bindMemory(to: Bytef.self).baseAddress)
        stream.avail_in = uInt(inputBuffer.count)
        stream.next_out = outputBuffer.bindMemory(to: Bytef.self).baseAddress
        stream.avail_out = uInt(outputBuffer.count)
        return deflate(&stream, Z_FINISH)
      }
    }
    guard result == Z_STREAM_END else { return nil }
    output.count = Int(stream.total_out)
    return output
  }

  private static func logSessionDataTiming(_ timing: SessionDataTiming) {
    #if DEBUG
      print(
        "[FloatView performance] session-data kind=\(timing.kind) rows=\(timing.sourceRows) " +
          "metadata_ms=\(timing.metadataMs) config_ms=\(timing.configurationMs) " +
          "source_ms=\(timing.sourceMs) reducer_ms=\(timing.reducerMs) " +
          "json_ms=\(timing.jsonMs) gzip_ms=\(timing.gzipMs) " +
          "response_bytes=\(timing.responseBytes) compressed_bytes=\(timing.compressedBytes) " +
          "total_ms=\(timing.totalMs)"
      )
    #endif
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

  private static let numericSeriesFields = [
    "packVoltageV", "dutyCycle", "controllerTempC", "motorTempC",
    "motorCurrentA", "batteryCurrentA", "erpm", "speedKph", "pitchDeg",
    "balancePitchDeg", "rollDeg", "directionalMotorCurrentA",
    "filteredMotorCurrentA", "requestedBalanceCurrentA", "boosterCurrentA",
    "fieldWeakeningCurrentA", "footpadAdc1V", "footpadAdc2V", "remoteInput",
    "setpointDeg", "atrSetpointDeg", "brakeTiltSetpointDeg",
    "torqueTiltSetpointDeg", "turnTiltSetpointDeg", "remoteSetpointDeg",
    "atrAccelerationDifferenceErpmPerSecond", "atrSpeedBoost", "distanceM",
    "odometerM", "ampHoursDrawnAh", "ampHoursChargedAh",
    "wattHoursDrawnWh", "wattHoursChargedWh", "batteryLevel",
    "chargingCurrentA", "chargingVoltageV",
  ]

  private func settingsPayload(_ value: SettingsRecord) -> [String: Any] {
    [
      "revision": value.revision,
      "idleProbeIntervalMs": value.idleProbeIntervalMs,
      "idlePersistIntervalMs": value.idlePersistIntervalMs,
      "ridingPersistIntervalMs": value.ridingPersistIntervalMs,
      "postRideTailMs": value.postRideTailMs,
      "reconnectDurationMs": value.reconnectDurationMs,
      "minimumSessionDistance": value.minimumSessionDistance,
      "themePreference": value.themePreference.rawValue,
      "unitPreference": value.unitPreference.rawValue,
      "historyGapThresholdSeconds": value.historyGapThresholdSeconds ?? NSNull(),
      "locationRecordingEnabled": value.locationRecordingEnabled,
      "includeGpsLocationWhenExporting": value.includeGpsLocationWhenExporting,
    ]
  }

  private func locationPayload(_ value: LocationProviderState) -> [String: Any] {
    let enabled = (try? applicationService.settings().locationRecordingEnabled) ?? false
    guard enabled else {
      return [
        "status": "disabled",
        "permissionAction": "requestPermission",
        "backgroundCapture": "unavailable",
        "reason": NSNull(),
      ]
    }
    let status: String
    switch value.authorization {
    case .notDetermined: status = "notDetermined"
    case .foregroundOnly: status = "foregroundOnly"
    case .authorized: status = "authorized"
    case .denied: status = "denied"
    case .restricted: status = "restricted"
    case .unavailable: status = "unavailable"
    }
    let permissionAction: Any = switch value.authorization {
    case .notDetermined: "requestPermission"
    case .denied, .restricted, .foregroundOnly: "openSettings"
    case .authorized, .unavailable: NSNull()
    }
    return [
      "status": status,
      "permissionAction": permissionAction,
      "backgroundCapture": value.backgroundCapture.rawValue,
      "reason": value.reason ?? NSNull(),
    ]
  }

  private func deviceState(_ state: ConnectionState) -> String {
    switch state {
    case .disconnected: "disconnected"
    case .connecting: "connecting"
    case .connected: "connected"
    case .discoveringServices, .enablingNotifications: "discovering_services"
    case .identifying: "identifying"
    case .selectingController: "selecting_controller"
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
    case .activeDeviceConflict, .invalidScan, .deviceNotReady, .invalidControllerSelection:
      failure(id, .invalidState, "Backend state does not permit the operation")
    case .bluetoothUnavailable, .productStoreUnavailable:
      failure(id, .unavailable, "Backend service is unavailable")
    }
  }

  private func productFailure(_ id: Int, _ error: ProductStoreError) -> Data {
    switch error {
    case .savedDeviceNotFound: failure(id, .notFound, "Saved device was not found")
    case .refloatConfigurationNotFound: failure(id, .notFound, "Tune was not found")
    case .savedDeviceRevisionConflict, .profileRevisionConflict, .settingsRevisionConflict,
         .refloatConfigurationRevisionConflict:
      failure(id, .conflict, "Resource revision changed")
    case .savedDeviceActive:
      failure(id, .invalidState, "Disconnect the board before forgetting it")
    case .invalidSavedDevicePatch, .invalidProfilePatch, .invalidSettingsPatch,
         .invalidRefloatConfiguration:
      failure(id, .invalidRequest, "Patch is invalid")
    }
  }

  private func sessionFailure(_ id: Int, _ error: SessionStoreError) -> Data {
    switch error {
    case .sessionNotFound: failure(id, .notFound, "Session was not found")
    case .sessionActive: failure(id, .invalidState, "Session is still active")
    case .maintenanceInProgress: failure(id, .unavailable, "Session maintenance is in progress")
    case .invalidCursor, .invalidRange, .unsupportedField, .invalidExportFormat, .invalidArchive:
      failure(id, .invalidRequest, "Session request is invalid")
    }
  }

  private static func json(_ value: Any) -> Data {
    try! JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
  }

  private static func jsonString(_ value: Any) throws -> String {
    try String(decoding: JSONSerialization.data(withJSONObject: value, options: [.sortedKeys]), as: UTF8.self)
  }

  private static func randomToken() -> String {
    var bytes = [UInt8](repeating: 0, count: 32)
    _ = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)
    return Data(bytes).base64EncodedString().replacingOccurrences(of: "/", with: "_")
      .replacingOccurrences(of: "+", with: "-").replacingOccurrences(of: "=", with: "")
  }

  private static func encodeSessionCatalogCursor(_ value: [String: Any]) -> String {
    json(value).base64EncodedString()
  }

  private static func decodeSessionCatalogCursor(_ value: String) -> [String: Any]? {
    guard let data = Data(base64Encoded: value),
          let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
          object["version"] as? Int == 1,
          object["query"] as? String == "startedAt.desc.sessionId.desc",
          object["innerCursor"] as? String != nil
    else { return nil }
    return object
  }
}

private struct BackendMessageHandler: WSMessageHandler {
  let server: LoopbackServer

  func makeMessages(for client: AsyncStream<WSMessage>) async throws -> AsyncStream<WSMessage> {
    AsyncStream(bufferingPolicy: .bufferingNewest(1)) { continuation in
      let subscription = server.events.subscribe { event in
        if invalidatesState(event) {
          continuation
            .yield(.text(server.stateChanged(catalogChanged: event is SessionRecord || event is SessionCatalogChanged)))
        }
      }
      Task {
        defer { _ = subscription }
        for await message in client {
          guard case let .text(text) = message,
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
  return event is BluetoothState || event is DiscoverySnapshot || event is ConnectionState ||
    event is KnownDevice ||
    event is SessionRecord || event is SavedDeviceRecord || event is BoardProfileRecord ||
    event is SettingsRecord || event is LocationProviderState || event is TelemetrySample ||
    event is BmsState || event is BmsSnapshot || event is RefloatControls ||
    event is RefloatConfigurationSummary || event is LocationUpdated || event is SessionCatalogChanged
}

private struct SessionCatalogChanged {}

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
