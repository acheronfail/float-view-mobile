import Foundation

/// Native lifecycle and execution for the shared transport.
final class RustLoopbackHost: @unchecked Sendable {
  private let app: ApplicationService
  private let exchange: ServerExchange
  private let operations: ServerOperations
  private let platform: String
  private let lock = NSLock()
  private let admission = DispatchSemaphore(value: 64)
  private let control = DispatchQueue(label: "float-view.host-control")
  private let publications = DispatchQueue(label: "float-view.host-publication")
  private var subscription: EventSubscription?
  private var generation: UInt64 = 0
  private var sequence: UInt64 = 0
  private var running = false
  private var dirty = false
  private var catalogChanged = false
  private var worker: Thread?
  private let finished = DispatchSemaphore(value: 0)
  private(set) var port: UInt16 = 0
  private(set) var token = ""
  init(
    html: Data,
    applicationService: ApplicationService,
    events: EventHub,
    platform: String,
    presenter: (@Sendable (SessionExportRecord) -> Bool)? = nil
  ) throws {
    app = applicationService; self.platform = platform
    exchange = try ServerExchange(platform: platform, html: html)
    operations = ServerOperations(app: applicationService, exchange: exchange, presenter: presenter)
    subscription = events.subscribe { [weak self] event in self?.publish(catalog: event is SessionRecord) }
  }

  func start() throws {
    if lock.withLock({ running }) {
      return
    }
    let result = try exchange.command("start", ["platform": platform, "port": platform == "ios" ? 11011 : 11010])
    let p = HostParameters(value: result)
    try lock
      .withLock {
        generation = try p.number("generation"); port = try UInt16(p.int("port")); token = try p
          .string("token"); running = true
      }
    publish(catalog: false)
    publications.sync {}
    let thread = Thread { [self] in drain(); finished.signal() }
    worker = thread; thread.name = "float-view-host"; thread.start()
  }

  func publish(catalog: Bool) {
    let schedule = lock.withLock { () -> Bool in
      catalogChanged = catalogChanged || catalog
      if dirty {
        return false
      }; dirty = true; return true
    }
    guard schedule else { return }
    publications.async { [self] in
      let (epoch, changed) = lock
        .withLock { () -> (UInt64, Bool) in
          dirty = false; let changed = catalogChanged; catalogChanged = false; return (
            generation,
            changed
          )
        }
      do {
        let facts = try app.withSnapshot { try ServerFacts.encode([
          "discovery": ServerFacts.encode(app.discoverySnapshot()),
          "active": ServerFacts.encode(app.activeConnectionSnapshot() as Any),
          "firmware": ServerFacts.encode(app.firmwareSnapshot() as Any),
          "packageVersion": ServerFacts.encode(app.refloatPackageVersionSnapshot() as Any),
          "configuration": ServerFacts.encode(app.refloatConfigurationSummarySnapshot()),
          "savedBoards": ServerFacts.encode(Array(app.savedDevices().prefix(50))),
          "settings": ServerFacts.encode(app.settings()), "location": ServerFacts.encode(app.locationState()),
          "maintenance": ServerFacts.encode(app.maintenanceJob() as Any),
        ]) }
        guard sequence < 9_007_199_254_740_991 else { return }
        sequence += 1
        _ = try exchange.command(
          "publish",
          ["generation": epoch, "sequence": sequence, "catalogChanged": changed, "facts": facts]
        )
      } catch { /* Retry from the next domain update; no private facts are logged. */ }
    }
  }

  private struct Work: @unchecked Sendable { let id: [String: Any]; let operation: [String: Any] }
  private func drain() {
    let epoch = lock.withLock { generation }
    while lock.withLock({ running && generation == epoch }) {
      do {
        let batch = try exchange.command("drain", ["generation": epoch, "wait_ms": 250])
        for id in batch["cleanup"] as? [UInt64] ?? [] {
          control.async { [self] in operations.cleanup(id); _ = try? exchange.command(
            "transfer_cleaned",
            ["transfer_id": id]
          ) }
        }
        for item in batch["operations"] as? [[String: Any]] ?? [] {
          guard let id = item["id"] as? [String: Any],
                let operation = item["operation"] as? [String: Any] else { continue }
          let work = Work(id: id, operation: operation)
          guard admission.wait(timeout: .now()) == .success else {
            try? exchange.complete(id: id, outcome: ["status": "failed", "value": "unavailable"])
            continue
          }
          let execute: @Sendable () -> Void = { [self] in
            defer { admission.signal() }
            let outcome: [String: Any]
            do {
              let value = try operations.execute(work.operation, generation: epoch)
              outcome = ["status": "completed", "value": value]
            } catch { outcome = ["status": "failed", "value": Self.failure(error)] }
            let name = work.operation["operation"] as? String ?? ""
            if ![
              "configuration_schema",
              "list_tunes",
              "get_tune",
              "get_profile",
              "list_sessions",
              "get_session",
              "get_maintenance",
              "export_session",
              "begin_import",
              "write_transfer",
              "read_transfer",
              "cleanup_transfer",
            ].contains(name) {
              publish(catalog: [
                "delete_session",
                "delete_saved_board",
                "finish_import",
                "save_maintenance",
                "prune_session",
              ].contains(name))
              publications.sync {}
            }
            do { try exchange.complete(id: work.id, outcome: outcome) } catch { _ = try? exchange.command(
              "complete",
              ["id": work.id, "outcome": ["status": "failed", "value": "unavailable"]]
            ) }
          }
          control.async(execute: execute)
        }
      } catch { break }
    }
  }

  func stop() {
    let active = lock.withLock { () -> Bool in let active = running; running = false; return active }
    guard active else { return }
    let result = try? exchange.command("stop")
    _ = finished.wait(timeout: .now() + 2)
    for id in result?["cleanup"] as? [UInt64] ?? [] {
      control.async { [self] in operations.cleanup(id) }
    }
    worker = nil; port = 0
  }

  private static func failure(_ error: any Error) -> String {
    switch error {
    case ApplicationError.unknownDevice: "not_found"
    case ApplicationError.bluetoothUnavailable, ApplicationError.productStoreUnavailable: "unavailable"
    case ApplicationError.permissionRequired: "permission_required"
    case ApplicationError.invalidScan, ApplicationError.invalidControllerSelection: "invalid_request"
    case ApplicationError.activeDeviceConflict, ApplicationError.deviceNotReady: "invalid_state"
    case RefloatMutationError.invalidState: "invalid_state"
    case RefloatMutationError.unavailable, RefloatConfigurationServiceError.unavailable: "unavailable"
    case RefloatConfigurationServiceError.conflict: "conflict"
    case is RefloatConfigurationServiceError: "invalid_state"
    case ProductStoreError.savedDeviceRevisionConflict, ProductStoreError.profileRevisionConflict,
         ProductStoreError.settingsRevisionConflict, ProductStoreError.refloatConfigurationRevisionConflict: "conflict"
    case ProductStoreError.savedDeviceNotFound, ProductStoreError.refloatConfigurationNotFound,
         SessionStoreError.sessionNotFound: "not_found"
    case ProductStoreError.savedDeviceActive, SessionStoreError.sessionActive: "invalid_state"
    case SessionStoreError.maintenanceInProgress: "unavailable"
    case is ProductStoreError, is SessionStoreError: "invalid_request"
    default: "internal"
    }
  }
}
