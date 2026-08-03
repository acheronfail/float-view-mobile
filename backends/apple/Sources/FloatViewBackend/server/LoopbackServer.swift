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
          let result = try await withTaskCancellationHandler(operation: {
            try self.sessionDataPayload(payload, timing: timing)
          }, onCancel: {
            self.applicationService.cancelHistoricalReads(sessionID: sessionID)
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
          return HTTPResponse(statusCode: .badRequest, headers: [.cacheControl: "no-store"])
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
        if let active = applicationService.activeConnectionSnapshot(),
           active.deviceID.value != deviceID || active.state != .failed
        {
          return failure(id, .invalidState, "A different board is already active")
        }
        try applicationService.connect(DeviceID(deviceID))
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
        guard let limit = (params["limit"] as? NSNumber)?.intValue, (1 ... 100).contains(limit) else {
          return failure(id, .invalidRequest, "Session page size must be 1 through 100")
        }
        let cursor = params["cursor"] as? String
        let context: [String: Any]?
        if let cursor {
          guard cursor.utf8.count <= 1_024, let decoded = Self.decodeSessionCatalogCursor(cursor) else {
            return failure(id, .invalidRequest, "Session cursor is invalid")
          }
          guard let revision = decoded["catalogRevision"] as? Int,
                revision == revisionLock.withLock({ sessionCatalogRevision })
          else {
            return failure(id, .conflict, "Session cursor is stale")
          }
          context = decoded
        } else {
          context = nil
        }
        let innerCursor = context?["innerCursor"] as? String
        let page = try applicationService.sessions(SessionListQuery(
          cursor: innerCursor, limit: limit,
          sortField: "startedAt", sortDescending: true
        ))
        let nextCursor = page.nextCursor.map { inner in
          Self.encodeSessionCatalogCursor([
            "version": 1,
            "query": "startedAt.desc.sessionId.desc",
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
        value["firmware"] = [
          "vescVersion": "\(firmware.major).\(String(format: "%02d", firmware.minor))",
          "packageName": "refloat",
          "packageVersion": packageVersion,
          "hardwareName": firmware.hardwareName.map { $0 as Any } ?? NSNull(),
          "accessMode": VescCompatibility.accessMode(firmware)?.rawValue ?? "readOnly",
          "supported": RefloatCompatibility.entry(wireVersion: packageVersion) != nil &&
            VescCompatibility.supports(firmware),
        ]
      } else {
        value["firmware"] = NSNull()
      }
      value["savedDeviceId"] = active.savedDevice?.savedDeviceID ?? NSNull()
      value["displayName"] = active.savedDevice?.displayName ?? active.name ??
        active.deviceID.value
      value["activeSession"] = active.activeSession.map(sessionPayload) ?? NSNull()
      value["latestSample"] = active.latestSample.map(liveSamplePayload) ?? NSNull()
      value["currentLimits"] = active.currentLimits.map(currentLimitsPayload) ?? NSNull()
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
      "controllerConnection": value.controllerRoute.map(controllerRoutePayload) ?? NSNull(),
      "controllerCandidates": value.controllerCandidates.map(controllerCandidatePayload),
      "connectionIssue": value.connectionIssue.map(connectionIssuePayload) ?? NSNull(),
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
          kind == "events" || (!fields.isEmpty && Set(fields).count == fields.count) else { return nil }
    let metadataStarted = DispatchTime.now().uptimeNanoseconds
    let published = try applicationService.sessionSnapshot(sessionID: sessionID)
    guard let snapshotRevision = snapshot["historyRevision"] as? NSNumber,
          let sampleSequence = snapshot["committedSampleSequence"] as? NSNumber,
          let eventSequence = snapshot["committedEventSequence"] as? NSNumber,
          let committedElapsed = snapshot["committedElapsedMs"] as? NSNumber
    else { return nil }
    guard snapshotRevision.int64Value == published.historyRevision,
          sampleSequence.int64Value == published.committedSampleSequence,
          eventSequence.int64Value == published.committedEventSequence,
          committedElapsed.int64Value == published.committedElapsedMs
    else { throw SessionStoreError.invalidRange }
    let echoedSnapshot: [String: Any] = [
      "historyRevision": snapshotRevision.int64Value,
      "committedSampleSequence": sampleSequence.int64Value,
      "committedEventSequence": eventSequence.int64Value,
      "committedElapsedMs": committedElapsed.int64Value,
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
      let requestedLimit = (params["mode"] as? String) == "nearest"
        ? 1
        : min(max((params["limit"] as? NSNumber)?.intValue ?? 2_000, 1), 2_000)
      let seed = SessionHistoryCursor(
        sessionID: sessionID, snapshot: published,
        range: SessionElapsedRange(startElapsedMs: start, endElapsedMs: end), elapsedMs: start, sequence: 0
      )
      let decodedCursor: SessionHistoryCursor?
      if let raw = params["cursor"] as? String {
        let cursor = try SessionHistoryReducer.decodeCursor(raw)
        guard cursor.sessionID == sessionID, cursor.snapshot == published,
              cursor.range.startElapsedMs == start, cursor.range.endElapsedMs == end
        else { throw SessionStoreError.invalidCursor }
        decodedCursor = cursor
      } else {
        decodedCursor = nil
      }
      let mode = params["mode"] as? String ?? "window"
      let points: [SessionSeriesPoint]
      let nextCursor: String?
      if mode == "nearest" {
        guard let target = (params["targetElapsedMs"] as? NSNumber)?.int64Value,
              target >= start, target <= end else { throw SessionStoreError.invalidRange }
        points = try applicationService.queryNearestSessionSample(
          sessionID: sessionID, targetElapsedMs: target, fields: fields,
          publishedSampleSequence: sampleSequence.int64Value
        ).map { [$0] } ?? []
        nextCursor = nil
      } else {
        let source = try applicationService.querySessionSeriesSource(
          sessionID: sessionID, startElapsedMs: start, endElapsedMs: end, fields: fields,
          maxRows: 200_000, publishedSampleSequence: sampleSequence.int64Value
        )
        let page = try SessionHistoryReducer.page(
          rows: source, limit: requestedLimit, cursor: decodedCursor, cursorContext: seed
        )
        points = page.rows
        nextCursor = try page.next.map(SessionHistoryReducer.encodeCursor)
      }
      let values = Dictionary(uniqueKeysWithValues: fields.map { field in
        (field, points.map { point -> Any in point.values[field].flatMap { $0 } as Any? ?? NSNull() })
      })
      return ["kind": kind, "sessionId": sessionID, "snapshot": echoedSnapshot, "range": range,
              "mode": mode, "fields": descriptors, "elapsedMs": points.map(\.elapsedMs),
              "sequence": points.map(\.sequence), "values": values, "nextCursor": nextCursor as Any? ?? NSNull()]
    }
    guard kind == "events" else { return nil }
    let requestedLimit = min(max((params["limit"] as? NSNumber)?.intValue ?? 500, 1), 500)
    let seed = SessionHistoryCursor(
      sessionID: sessionID, snapshot: published,
      range: SessionElapsedRange(startElapsedMs: start, endElapsedMs: end), elapsedMs: start, sequence: 0
    )
    let decodedCursor: SessionHistoryCursor?
    if let raw = params["cursor"] as? String {
      let cursor = try SessionHistoryReducer.decodeCursor(raw)
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
      next = try SessionHistoryReducer.encodeCursor(cursor)
    } else {
      next = nil
    }
    return ["kind": kind, "sessionId": sessionID, "snapshot": echoedSnapshot, "range": range,
            "page": ["elapsedMs": events.map(\.elapsedMs), "sequence": events.map { Int64($0.eventID) ?? 0 },
                     "sourceSampleSequence": events.map { $0.sourceSampleSequence ?? 0 },
                     "kinds": events.map(\.kind), "values": values],
            "nextCursor": next as Any? ?? NSNull()]
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
  if let message = event as? VescProtocolMessage {
    switch message {
    case .firmware, .currentLimits, .refloatInfo:
      return true
    case .canDevices, .refloatTelemetryLayout, .refloatTelemetry:
      return false
    }
  }
  return event is BluetoothState || event is DiscoverySnapshot || event is ConnectionState ||
    event is KnownDevice ||
    event is SessionRecord || event is SavedDeviceRecord || event is BoardProfileRecord ||
    event is SettingsRecord || event is TelemetrySample || event is SessionCatalogChanged
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
