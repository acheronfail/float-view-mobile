import Foundation
import Testing
@testable import FloatViewCore

@Test
func readyCreatesDurableSession() throws {
  let store = MemoryStore()
  let service = ApplicationService(
    bluetooth: FakeBluetooth(),
    clock: FixedClock(),
    logStore: store,
    events: IgnoreEvents()
  )
  let id = DeviceID("fixture")

  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))

  #expect(service.connectionState == .ready)
  #expect(store.active() != nil)
}

private final class FakeBluetooth: BluetoothPort, @unchecked Sendable {
  func startDiscovery() {}
  func stopDiscovery() {}
  func connect(_: DeviceID) {}
  func write(_: Data, to _: DeviceID) {}
  func disconnect(_: DeviceID) {}
}

private struct FixedClock: ClockPort {
  func monotonicMilliseconds() -> Int64 { 0 }
  func now() -> Date { .init(timeIntervalSince1970: 0) }
}

private final class MemoryStore: LogStorePort, @unchecked Sendable {
  private var record: SessionRecord?
  func create(deviceID: DeviceID, at: Date) throws -> SessionRecord {
    let result = SessionRecord(
      sessionID: SessionID("session"),
      jobID: JobID("job"),
      deviceID: deviceID,
      startedAt: at,
      committedSequence: 0,
      active: true
    )
    record = result
    return result
  }
  func append(jobID _: JobID, samples _: [TelemetrySample]) throws {}
  func close(jobID _: JobID, at _: Date, reason _: String) throws -> SessionRecord {
    let result = try #require(record).withActive(false)
    record = nil
    return result
  }
  func active() -> SessionRecord? { record }
}

private struct IgnoreEvents: EventSink {
  func publish(_: any Sendable) {}
}

extension SessionRecord {
  fileprivate func withActive(_ active: Bool) -> SessionRecord {
    SessionRecord(
      sessionID: sessionID,
      jobID: jobID,
      deviceID: deviceID,
      startedAt: startedAt,
      committedSequence: committedSequence,
      active: active
    )
  }
}
