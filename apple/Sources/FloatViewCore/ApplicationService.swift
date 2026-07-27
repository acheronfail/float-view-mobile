import Foundation

public final class ApplicationService: @unchecked Sendable {
  private let bluetooth: BluetoothPort
  private let clock: ClockPort
  private let logStore: LogStorePort
  private let events: EventSink
  private var sampling: AdaptiveSamplingPolicy
  private var explicitDisconnect = false

  public private(set) var activeDeviceID: DeviceID?
  public private(set) var connectionState: ConnectionState = .disconnected

  public init(
    bluetooth: BluetoothPort,
    clock: ClockPort,
    logStore: LogStorePort,
    events: EventSink,
    samplingSettings: SamplingSettings = .init()
  ) {
    self.bluetooth = bluetooth
    self.clock = clock
    self.logStore = logStore
    self.events = events
    self.sampling = AdaptiveSamplingPolicy(settings: samplingSettings)
  }

  public func connect(_ deviceID: DeviceID) throws {
    guard activeDeviceID == nil || activeDeviceID == deviceID else {
      throw ApplicationError.activeDeviceConflict
    }
    activeDeviceID = deviceID
    explicitDisconnect = false
    transition(.connecting)
    bluetooth.connect(deviceID)
  }

  public func receive(_ event: BluetoothEvent) throws {
    switch event {
    case .connectionChanged(let deviceID, let state):
      guard deviceID == activeDeviceID else { return }
      if state == .ready, logStore.active() == nil {
        events.publish(try logStore.create(deviceID: deviceID, at: clock.now()))
      }
      if state == .disconnected, !explicitDisconnect, logStore.active() != nil {
        transition(.reconnecting)
        bluetooth.connect(deviceID)
      } else {
        transition(state)
        if state == .disconnected { activeDeviceID = nil }
      }
    default:
      events.publish(event)
    }
  }

  public func disconnect() throws {
    guard let deviceID = activeDeviceID else { return }
    transition(.disconnecting)
    explicitDisconnect = true
    if let active = logStore.active() {
      events.publish(
        try logStore.close(jobID: active.jobID, at: clock.now(), reason: "explicit_disconnect")
      )
    }
    bluetooth.disconnect(deviceID)
  }

  public func ingest(_ sample: TelemetrySample) throws {
    guard connectionState == .ready, let active = logStore.active() else { return }
    if sampling.shouldPersist(rideState: sample.rideState, elapsedMs: sample.elapsedMs) {
      try logStore.append(jobID: active.jobID, samples: [sample])
    }
    events.publish(sample)
  }

  private func transition(_ state: ConnectionState) {
    connectionState = state
    events.publish(state)
  }
}

public enum ApplicationError: Error, Equatable {
  case activeDeviceConflict
}
