import Foundation

/// Closed domain operations executed against the existing process application.
final class ServerOperations: @unchecked Sendable {
  let app: ApplicationService
  let exchange: ServerExchange
  let presenter: (@Sendable (SessionExportRecord) -> Bool)?
  private var scan: DispatchWorkItem?
  private var files: [UInt64: (URL, FileHandle)] = [:]
  init(app: ApplicationService, exchange: ServerExchange, presenter: (@Sendable (SessionExportRecord) -> Bool)?) {
    self.app = app; self.exchange = exchange; self.presenter = presenter
  }

  func cleanup(_ id: UInt64) {
    if let (url, file) = files
      .removeValue(forKey: id)
    {
      try? file.close(); try? FileManager.default.removeItem(at: url)
    }
  }

  func result(_ kind: String, _ facts: Any? = nil) throws -> [String: Any] {
    var result: [String: Any] = ["kind": kind]
    if kind != "unit", kind != "transfer_ready" {
      result["facts"] = try facts.map(ServerFacts.encode) ?? NSNull()
    }
    return result
  }

  func execute(_ op: [String: Any], generation: UInt64) throws -> [String: Any] {
    let p = HostParameters(value: op)
    switch try p.string("operation") {
    case "request_bluetooth_permission": return try result("bluetooth", app.requestBluetoothPermission())
    case "request_location_permission": return try result("location", app.requestLocationPermission())
    case "open_location_settings": return try result("location", app.openLocationSettings())
    case "scan":
      let id = try app.startDiscovery()
      scan?.cancel()
      let app = app
      let work = DispatchWorkItem { try? app.stopDiscovery(scanID: id) }
      scan = work
      try DispatchQueue.global().asyncAfter(deadline: .now() + .milliseconds(p.int("timeout_ms")), execute: work)
      return try result("unit")
    case "connect": try app.connect(DeviceID(p.string("device_id")), trackingMode: p.boolean("saved") ? .saved : .guest)
    case "select_controller": try app.selectController(
        deviceID: DeviceID(p.string("device_id")),
        canID: UInt8(p.int("can_id"))
      )
    case "disconnect": try app.disconnect()
    case "shutdown_controller": try app.shutdownController()
    case "set_control":
      guard let control = try RefloatBooleanControl(rawValue: p.string("control"))
      else { throw ServerFactError.invalid }
      return try result("controls", app.setRefloatControl(control, enabled: p.boolean("enabled")))
    case "clear_fatal_error": return try result("controls", app.clearRefloatFatalError())
    case "configuration_schema":
      guard let schema = app.refloatConfigurationSchemaSnapshot() else { throw ServerFactError.invalid }
      return try result("configuration_schema", schema)
    case "read_configuration": return try result(
        "configuration_values",
        app.readRefloatConfiguration(defaults: p.boolean("defaults"))
      )
    case "write_configuration": return try result(
        "configuration_values",
        app.writeRefloatConfiguration(
          packageVersion: p.string("package_version"),
          fingerprint: p.string("fingerprint"),
          wireSignature: UInt32(p.number("signature")),
          baseRevision: p.number("revision"),
          values: p.values()
        )
      )
    case "list_tunes": return try result(
        "tunes",
        app.refloatTunes(identity: p.identity(), limit: p.int("limit"), cursor: p.optional("cursor"))
      )
    case "get_tune": return try result("tune", app.refloatTune(configurationID: p.string("id")))
    case "create_tune": return try result(
        "tune",
        app.createRefloatTune(
          name: p.string("name"),
          description: p.optional("description"),
          identity: p.identity(),
          sourceRevision: p.number("revision"),
          values: p.values()
        )
      )
    case "update_tune": return try result(
        "tune",
        app.updateRefloatTune(
          configurationID: p.string("id"),
          expectedRevision: p.int("revision"),
          name: p.string("name"),
          description: p.optional("description"),
          values: p.values()
        )
      )
    case "delete_tune": try app.deleteRefloatTune(configurationID: p.string("id"), expectedRevision: p.int("revision"))
    case "apply_tune": return try result(
        "configuration_values",
        app.applyRefloatTune(
          configurationID: p.string("id"),
          expectedRevision: p.int("revision"),
          baseRevision: p.number("base_revision")
        )
      )
    case "update_saved_board": return try result(
        "saved_board",
        app.updateSavedDevice(
          savedDeviceID: p.string("id"),
          expectedRevision: p.int("revision"),
          customDisplayName: p.optional("name")
        )
      )
    case "delete_saved_board": try app.deleteSavedDevice(savedDeviceID: p.string("id"))
    case "update_settings": return try result(
        "settings",
        app.updateSettings(expectedRevision: p.int("revision"), patchJSON: p.string("patch_json"))
      )
    case "get_profile": return try result("profile", app.boardProfile(savedDeviceID: p.string("id")))
    case "update_profile": return try result(
        "profile",
        app.updateBoardProfile(
          savedDeviceID: p.string("id"),
          expectedRevision: p.int("revision"),
          patchJSON: p.string("patch_json")
        )
      )
    case "list_sessions": return try result(
        "sessions",
        app.sessions(SessionListQuery(
          cursor: p.optional("cursor"),
          limit: p.int("limit"),
          deviceHistoryID: p.optional("history_id"),
          state: p.boolean("completed_only") ? "completed" : nil
        ))
      )
    case "get_session":
      let id = try p.string("id")
      return try result(
        "session",
        [
          "summary": ServerFacts.encode(app.session(sessionID: id)),
          "boundary": ServerFacts.encode(app.sessionSnapshot(sessionID: id)),
          "path": app.sessionPath(id),
        ]
      )
    case "delete_session": try app.deleteSession(sessionID: p.string("id"))
    case "cancel_delete": return try result("cancel_delete", app.cancelPendingDelete(operationID: p.string("id")))
    case "get_maintenance": return try result("maintenance", app.maintenanceJob())
    case "cancel_maintenance": return try result("maintenance", app.cancelMaintenanceJob(operationID: p.string("id")))
    case "save_maintenance":
      let j = try HostParameters(value: p.object("job"))
      try app.saveMaintenanceJob(SessionMaintenanceJobRecord(
        operationID: j.string("operationId"),
        kind: j.string("kind"),
        scope: j.string("scope"),
        state: j.string("state"),
        sessionID: j.optional("sessionId"),
        targetResolutionSeconds: Int64(j.number("targetResolutionSeconds")),
        upperSessionID: j.optional("upperSessionId"),
        lastSessionID: j.optional("lastSessionId"),
        completedSessions: Int64(j.number("completedSessions")),
        skippedSessions: Int64(j.number("skippedSessions")),
        retainedSamples: Int64(j.number("retainedSamples")),
        reclaimableBytes: Int64(j.number("reclaimableBytes")),
        error: j.optional("error")
      ))
    case "prune_session": return try result(
        "prune_preview",
        p.boolean("preview") ? app.prunePreview(
          sessionID: p.string("id"),
          targetResolutionSeconds: Int64(p.number("resolution"))
        ) : app.pruneSession(sessionID: p.string("id"), targetResolutionSeconds: Int64(p.number("resolution")))
      )
    case "begin_import":
      guard files.count < 2 else { throw ServerFactError.invalid }
      let url = FileManager.default.temporaryDirectory.appendingPathComponent("float-view-import-\(UUID().uuidString)")
      guard FileManager.default.createFile(atPath: url.path, contents: nil) else { throw ServerFactError.invalid }
      try files[p.number("transfer_id")] = try (url, FileHandle(forUpdating: url))
      return try result("transfer_ready")
    case "write_transfer":
      let id = try p.number("transfer_id")
      guard let (_, file) = files[id], try file.offset() == p.number("offset") else { throw ServerFactError.invalid }
      try file.write(contentsOf: exchange.read(generation: generation, transfer: id))
      return try result("transfer_ready")
    case "finish_import":
      guard let (url, _) = try files[p.number("transfer_id")] else { throw ServerFactError.invalid }
      return try result("imported", app.importSessionArchive(data: Data(contentsOf: url)))
    case "export_session":
      let record = try app.exportSession(sessionID: p.string("id"), format: p.string("format"))
      let presented = try p.boolean("present") && presenter?(record) == true
      let id = try p.number("transfer_id")
      if id != 0 {
        guard files.count < 2 else { throw ServerFactError.invalid }; files[id] = try (
          record.fileURL,
          FileHandle(forReadingFrom: record.fileURL)
        )
      }
      let length = try FileManager.default.attributesOfItem(atPath: record.fileURL.path)[.size] as? NSNumber ?? 0
      return try result(
        "export",
        ["file_name": record.fileName, "presented": presented, "transfer_id": id, "length": length]
      )
    case "read_transfer":
      let id = try p.number("transfer_id"), offset = try p.number("offset")
      guard let (_, file) = files[id] else { throw ServerFactError.invalid }
      try file.seek(toOffset: offset)
      guard let data = try file.read(upToCount: p.int("length")),
            try data.count == (p.int("length")) else { throw ServerFactError.invalid }
      try exchange.write(generation: generation, transfer: id, offset: offset, data: data)
      return try result("transfer_ready")
    case "cleanup_transfer": try cleanup(p.number("transfer_id"))
    default: throw ServerFactError.invalid
    }
    return try result("unit")
  }
}

struct HostParameters {
  let value: [String: Any]
  func string(_ key: String) throws -> String {
    guard let v = value[key] as? String
    else { throw ServerFactError.invalid }; return v
  }

  func optional(_ key: String) -> String? {
    value[key] as? String
  }

  func number(_ key: String) throws -> UInt64 {
    guard let v = value[key] as? NSNumber,
          v.doubleValue >= 0
    else { throw ServerFactError.invalid }; return v.uint64Value
  }

  func int(_ key: String) throws -> Int {
    guard let v = try Int(exactly: number(key))
    else { throw ServerFactError.invalid }; return v
  }

  func boolean(_ key: String) throws -> Bool {
    guard let v = value[key] as? Bool
    else { throw ServerFactError.invalid }; return v
  }

  func object(_ key: String) throws -> [String: Any] {
    guard let v = value[key] as? [String: Any]
    else { throw ServerFactError.invalid }; return v
  }

  func identity() throws -> RefloatConfigurationIdentityRecord {
    let p = try HostParameters(value: object("identity"))
    return try RefloatConfigurationIdentityRecord(
      packageName: p.string("packageName"),
      packageVersion: p.string("packageVersion"),
      schemaFingerprint: p.string("schemaFingerprint"),
      wireSignature: UInt32(p.number("wireSignature"))
    )
  }

  func values() throws -> [String: RefloatConfigurationValue] {
    try object("values").mapValues { raw in
      guard let value = raw as? [String: Any] else { throw ServerFactError.invalid }
      let p = HostParameters(value: value)
      switch try p.string("type") {
      case "double": guard let n = value["value"] as? Double else { throw ServerFactError.invalid }; return .double(n)
      case "integer": guard let n = value["value"] as? Int64 else { throw ServerFactError.invalid }; return .integer(n)
      case "boolean": return try .boolean(p.boolean("value"))
      case "string": return try .string(p.string("value"))
      default: throw ServerFactError.invalid
      }
    }
  }
}
