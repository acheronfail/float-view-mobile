import Foundation
import Testing
@testable import FloatViewCore
@testable import FloatViewLogging

private func repositoryRoot() -> URL {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0..<4 { root.deleteLastPathComponent() }
  return root
}

@Test func sqliteStoreCommitsAndClosesDurably() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-store-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let store = try SQLiteLogStore(
    root: root,
    schemaDirectory: repositoryRoot().appending(path: "logging/sql"),
    availableBytes: { 2_000_000_000 }
  )
  let record = try store.create(deviceID: DeviceID("fixture"), at: .distantPast)
  try store.append(
    jobID: record.jobID,
    samples: [
      TelemetrySample(
        sequence: 1, observedAt: .distantPast, elapsedMs: 100,
        rideState: .riding, packVoltageV: 51.6
      ),
    ]
  )
  let closed = try store.close(jobID: record.jobID, at: .distantFuture, reason: "test")
  #expect(closed.committedSequence == 1)
  #expect(!closed.active)
  #expect(store.active() == nil)
}

@Test func sqliteStoreRecoversAnInterruptedSessionAtStartupBoundary() throws {
  let root = FileManager.default.temporaryDirectory
    .appending(path: "float-view-recovery-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let schema = repositoryRoot().appending(path: "logging/sql")
  let interrupted = try SQLiteLogStore(
    root: root,
    schemaDirectory: schema,
    availableBytes: { 2_000_000_000 }
  )
  _ = try interrupted.create(deviceID: DeviceID("fixture"), at: .distantPast)

  let restarted = try SQLiteLogStore(
    root: root,
    schemaDirectory: schema,
    availableBytes: { 2_000_000_000 }
  )
  #expect(try restarted.recoverInterrupted() == 1)
  #expect(try restarted.recoverInterrupted() == 0)
}

@Test func storageThresholdsMatchContract() {
  #expect(StoragePolicy.level(availableBytes: StoragePolicy.warningBytes) == .normal)
  #expect(StoragePolicy.level(availableBytes: StoragePolicy.criticalBytes) == .warning)
  #expect(StoragePolicy.level(availableBytes: StoragePolicy.stopBytes) == .critical)
  #expect(StoragePolicy.level(availableBytes: StoragePolicy.stopBytes - 1) == .stop)
}
