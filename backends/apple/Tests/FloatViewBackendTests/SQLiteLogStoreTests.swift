import CSQLite
@testable import FloatViewBackend
import Foundation
import Testing

private func repositoryRoot() -> URL {
  var root = URL(fileURLWithPath: #filePath)
  for _ in 0 ..< 4 {
    root.deleteLastPathComponent()
  }
  return root
}

private struct TestClock: ClockPort {
  let date = Date(timeIntervalSince1970: 1_700_000_000)
  func monotonicMilliseconds() -> Int64 {
    30_000
  }

  func now() -> Date {
    date
  }
}

private final class TestBluetooth: BluetoothPort, @unchecked Sendable {
  func currentState() -> BluetoothState {
    .init(radio: .poweredOn, scanPermission: .granted, connectPermission: .granted)
  }

  func requestPermission() {}
  func startDiscovery() {}
  func stopDiscovery() {}
  func connect(_: DeviceID) {}
  func write(_: Data, to _: DeviceID) {}
  func disconnect(_: DeviceID) {}
}

private func makeStore(_ root: URL) throws -> SQLiteLogStore {
  try SQLiteLogStore(
    root: root,
    schemaDirectory: repositoryRoot().appending(path: "sql"),
    availableBytes: { 2_000_000_000 }
  )
}

@Test func sqliteCatalogueOwnsLifecycleAndExports() throws {
  let root = FileManager.default.temporaryDirectory.appending(path: "float-view-catalog-\(UUID())")
  defer { try? FileManager.default.removeItem(at: root) }
  let store = try makeStore(root)
  let started = Date(timeIntervalSince1970: 1_700_000_000)
  let record = try store.create(deviceID: DeviceID("fixture"), advertisedName: "Fixture", at: started)
  try store.append(jobID: record.jobID, samples: [
    TelemetrySample(
      sequence: 1, observedAt: started, elapsedMs: 0, rideState: .riding,
      packVoltageV: 52, speedKph: 20, distanceM: 0
    ),
    TelemetrySample(
      sequence: 2, observedAt: started.addingTimeInterval(30), elapsedMs: 30_000,
      rideState: .riding, packVoltageV: 50, speedKph: 20, distanceM: 200
    ),
  ])
  let closed = try store.close(jobID: record.jobID, at: started.addingTimeInterval(31), reason: "test")
  #expect(!closed.active)
  #expect(try store.sessions(SessionListQuery()).items.map(\.sessionID) == [record.sessionID.value])
  #expect(try FileManager.default.fileExists(atPath: store.sessionURL(sessionID: record.sessionID.value).path))
}

@Test func unifiedRuntimeReadsReducesLocationsAndPrunesSQLiteSessions() throws {
  let root = FileManager.default.temporaryDirectory.appending(path: "float-view-runtime-session-\(UUID())")
  defer { try? FileManager.default.removeItem(at: root) }
  let store = try makeStore(root)
  let started = Date(timeIntervalSince1970: 1_700_000_000)
  let record = try store.create(deviceID: DeviceID("fixture"), advertisedName: nil, at: started)
  var samples: [TelemetrySample] = []
  for index in 0 ..< 31 {
    samples.append(TelemetrySample(
      sequence: Int64(index + 1), observedAt: started.addingTimeInterval(Double(index)),
      elapsedMs: Int64(index * 1_000), rideState: .riding,
      packVoltageV: 52 - Double(index) / 10, speedKph: 20,
      pitchDeg: Double(index % 5), distanceM: Double(index * 10),
      extensionFields: ["future.energy": Double(index)]
    ))
  }
  try store.append(jobID: record.jobID, samples: samples)
  try store.appendLocations(jobID: record.jobID, observations: [
    LocationObservation(observedAt: started, elapsedMs: 1_000, latitude: 1, longitude: 2, accuracyM: 3),
    LocationObservation(
      observedAt: started.addingTimeInterval(20),
      elapsedMs: 20_000,
      latitude: 4,
      longitude: 5,
      accuracyM: nil
    ),
  ])
  _ = try store.close(jobID: record.jobID, at: started.addingTimeInterval(31), reason: "test")

  let service = ApplicationService(
    bluetooth: TestBluetooth(), clock: TestClock(), logStore: store,
    productStore: store, sessionStore: store, events: EventHub()
  )
  let snapshot = try service.sessionSnapshot(sessionID: record.sessionID.value)
  let rows = try service.querySessionSeriesSource(
    sessionID: record.sessionID.value, startElapsedMs: 0, endElapsedMs: 30_000,
    fields: ["packVoltageV", "future.energy"], maxRows: 2_000,
    publishedSampleSequence: snapshot.committedSampleSequence
  )
  #expect(rows.count == 31)
  #expect(rows.last?.values["future.energy"] == 30)
  let reduced = try service.reduceSessionSeries(
    sessionID: record.sessionID.value, startElapsedMs: 0, endElapsedMs: 30_000,
    fields: ["packVoltageV"], buckets: 8, gapPolicy: .init(thresholdSeconds: nil),
    expectedPersistenceMs: 1_000, expectedRidingPersistenceMs: 1_000,
    expectedIdlePersistenceMs: 1_000, retainedResolutionSeconds: 0,
    publishedSampleSequence: snapshot.committedSampleSequence
  )
  #expect(reduced.sourceRows == 31)
  #expect(!(reduced.columns["packVoltageV"]?.values.isEmpty ?? true))
  #expect(try service.sessionExtensionFields(sessionID: record.sessionID.value) == ["future.energy"])
  let locations = try service.querySessionLocations(
    sessionID: record.sessionID.value, startElapsedMs: 0, endElapsedMs: 30_000,
    limit: 10, cursor: nil, publishedLocationSequence: snapshot.committedLocationSequence
  )
  #expect(locations.points.count == 2)
  #expect(locations.gaps.count == 1)
  let preview = try service.prunePreview(sessionID: record.sessionID.value, targetResolutionSeconds: 5)
  #expect(preview.recordedSampleCount == 31)
  _ = try service.pruneSession(sessionID: record.sessionID.value, targetResolutionSeconds: 5)
  #expect(try service.sessionSnapshot(sessionID: record.sessionID.value).storedSampleCount <= 31)
}

@Test func rustCommitMarkerAdvancesTheRetainedNativeCatalogue() throws {
  let root = FileManager.default.temporaryDirectory.appending(path: "float-view-rust-marker-\(UUID())")
  defer { try? FileManager.default.removeItem(at: root) }
  let store = try makeStore(root)
  let started = Date(timeIntervalSince1970: 1_700_000_000)
  let record = try store.create(deviceID: DeviceID("fixture"), advertisedName: nil, at: started)
  let snapshot = SessionPublishedSnapshotRecord(
    historyRevision: 4, committedSampleSequence: 7, committedEventSequence: 2,
    committedElapsedMs: 6_000, recordedSampleCount: 7, storedSampleCount: 7,
    committedLocationSequence: 3, committedLocationElapsedMs: 5_000,
    recordedLocationCount: 3, storedLocationCount: 3, locationStatus: "recorded"
  )
  try store.applySessionCommitMarker(
    sessionID: record.sessionID.value,
    marker: SessionCommitMarkerRecord(
      snapshot: snapshot, committedEventCount: 2, committedAt: started.addingTimeInterval(6),
      ridingElapsedMs: 5_000, distanceM: 25, maximumSpeedKph: 18,
      maximumSpeedSourceSequence: 6, maximumDutyCycle: 0.7,
      maximumDutySourceSequence: 5, lastSpeedKph: 9, lastRideState: "riding"
    )
  )

  #expect(store.active()?.committedSequence == 7)
  let summary = try store.session(sessionID: record.sessionID.value)
  #expect(summary.sampleCount == 7)
  #expect(summary.ridingElapsedMs == 5_000)
  #expect(summary.distanceM == 25)
  #expect(summary.maxSpeedMps == 5)
  #expect(try store.sessionSnapshot(sessionID: record.sessionID.value) == snapshot)
}
