import Foundation

public enum RefloatControlStatus: String, Sendable, Codable {
  case loading, ready, unavailable
}

public enum RefloatPendingAction: String, Sendable, Codable {
  case package, flywheel, leds, headlights
  case clearFatalError
}

public struct RefloatControls: Sendable, Equatable {
  public var status: RefloatControlStatus
  public var writable: Bool
  public var packageEnabled: Bool?
  public var flywheelEnabled: Bool?
  public var lightingAvailable: Bool
  public var ledsEnabled: Bool?
  public var headlightsEnabled: Bool?
  public var fatalErrorClearAvailable: Bool
  public var fatalErrorLatched: Bool?
  public var fatalErrorActive: Bool?
  public var fatalErrorClearable: Bool?
  public var pendingAction: RefloatPendingAction?

  public init(
    status: RefloatControlStatus = .loading,
    writable: Bool = false,
    packageEnabled: Bool? = nil,
    flywheelEnabled: Bool? = nil,
    lightingAvailable: Bool = false,
    ledsEnabled: Bool? = nil,
    headlightsEnabled: Bool? = nil,
    fatalErrorClearAvailable: Bool = false,
    fatalErrorLatched: Bool? = nil,
    fatalErrorActive: Bool? = nil,
    fatalErrorClearable: Bool? = nil,
    pendingAction: RefloatPendingAction? = nil
  ) {
    self.status = status
    self.writable = writable
    self.packageEnabled = packageEnabled
    self.flywheelEnabled = flywheelEnabled
    self.lightingAvailable = lightingAvailable
    self.ledsEnabled = ledsEnabled
    self.headlightsEnabled = headlightsEnabled
    self.fatalErrorClearAvailable = fatalErrorClearAvailable
    self.fatalErrorLatched = fatalErrorLatched
    self.fatalErrorActive = fatalErrorActive
    self.fatalErrorClearable = fatalErrorClearable
    self.pendingAction = pendingAction
  }
}

public struct RefloatMutationContext: Sendable {
  public let generation: VescRequestGeneration
  public let route: ControllerRoute
  public let profile: RefloatControlProfile
  public let connectionReady: Bool
  public let writable: Bool
  public let rideState: RideState
  public let packageState: PackageState?
  public let packageMode: PackageMode?
}

public struct AuthorizedRefloatMutation: Sendable, Equatable {
  public let action: RefloatPendingAction
  public let payload: [UInt8]
  public let oneWay: Bool
  public let expectedResponse: VescExpectedResponse
}

public enum RefloatMutationAuthorization: Sendable, Equatable {
  case idempotent(RefloatControls)
  case mutation(AuthorizedRefloatMutation, RefloatControls)
}

public enum RefloatMutationError: Error, Equatable {
  case invalidState
  case unavailable
}

public enum ControllerPendingAction: String, Sendable, Codable {
  case shutdown
}

public struct ControllerActions: Sendable, Equatable {
  public let shutdownAvailable: Bool
  public let pendingAction: ControllerPendingAction?

  public init(shutdownAvailable: Bool = false, pendingAction: ControllerPendingAction? = nil) {
    self.shutdownAvailable = shutdownAvailable
    self.pendingAction = pendingAction
  }
}

public struct ControllerShutdownContext: Sendable {
  public let generation: VescRequestGeneration
  public let connectionReady: Bool
  public let shutdownSupported: Bool
  public let rideState: RideState
  public let packageMode: PackageMode?

  public init(
    generation: VescRequestGeneration,
    connectionReady: Bool,
    shutdownSupported: Bool,
    rideState: RideState,
    packageMode: PackageMode?
  ) {
    self.generation = generation
    self.connectionReady = connectionReady
    self.shutdownSupported = shutdownSupported
    self.rideState = rideState
    self.packageMode = packageMode
  }
}

public struct RefloatConfigurationMutationContext: Sendable {
  public let generation: VescRequestGeneration
  public let connectionReady: Bool
  public let writable: Bool
  public let rideState: RideState
  public let footpadState: FootpadState?
  public let packageState: PackageState?
  public let packageMode: PackageMode?

  public init(
    generation: VescRequestGeneration, connectionReady: Bool, writable: Bool,
    rideState: RideState, footpadState: FootpadState?, packageState: PackageState?,
    packageMode: PackageMode?
  ) {
    self.generation = generation
    self.connectionReady = connectionReady
    self.writable = writable
    self.rideState = rideState
    self.footpadState = footpadState
    self.packageState = packageState
    self.packageMode = packageMode
  }
}

/** The sole connection-scoped authorization and serialization boundary for device mutations. */
public final class DeviceMutationCoordinator: @unchecked Sendable {
  private let lock = NSLock()
  private var generation = VescRequestGeneration(rawValue: 0)
  private var controls: RefloatControls?
  private var pendingExpected: Bool?
  private var controllerPendingAction: ControllerPendingAction?
  private var configurationPending = false

  public init() {}

  public func configure(
    generation: VescRequestGeneration,
    profile: RefloatControlProfile,
    writable: Bool
  ) -> RefloatControls {
    lock.withLock {
      self.generation = generation
      pendingExpected = nil
      controllerPendingAction = nil
      configurationPending = false
      controls = RefloatControls(
        status: .loading,
        writable: writable,
        lightingAvailable: true,
        fatalErrorClearAvailable: profile.fatalClearAvailable,
        fatalErrorLatched: profile.fatalClearAvailable ? nil : false,
        fatalErrorActive: profile.fatalClearAvailable ? nil : false,
        fatalErrorClearable: profile.fatalClearAvailable ? nil : false
      )
      return controls!
    }
  }

  public func snapshot() -> RefloatControls? {
    lock.withLock { controls }
  }

  public func controllerActions(shutdownAvailable: Bool) -> ControllerActions {
    lock.withLock {
      ControllerActions(
        shutdownAvailable: shutdownAvailable && controllerPendingAction == nil && !configurationPending,
        pendingAction: controllerPendingAction
      )
    }
  }

  public func authorizeShutdown(context: ControllerShutdownContext) throws -> ControllerActions {
    try lock.withLock {
      guard context.generation == generation,
            context.connectionReady,
            context.shutdownSupported,
            context.rideState == .idle,
            context.packageMode != .flywheel,
            controls?.pendingAction == nil,
            controllerPendingAction == nil,
            !configurationPending
      else { throw RefloatMutationError.invalidState }
      controllerPendingAction = .shutdown
      return ControllerActions(shutdownAvailable: false, pendingAction: .shutdown)
    }
  }

  public func clearShutdown(generation: VescRequestGeneration) -> ControllerActions? {
    lock.withLock {
      guard generation == self.generation, controllerPendingAction == .shutdown else { return nil }
      controllerPendingAction = nil
      return ControllerActions()
    }
  }

  public func authorizeConfiguration(
    context: RefloatConfigurationMutationContext
  ) throws {
    try lock.withLock {
      guard context.generation == generation,
            context.connectionReady,
            context.writable,
            context.rideState == .idle,
            context.footpadState == FootpadState.none,
            context.packageState == .ready,
            context.packageMode == .normal,
            controls?.pendingAction == nil,
            controllerPendingAction == nil,
            !configurationPending
      else { throw RefloatMutationError.invalidState }
      configurationPending = true
    }
  }

  public func finishConfiguration(generation: VescRequestGeneration) {
    lock.withLock {
      guard generation == self.generation else { return }
      configurationPending = false
    }
  }

  public func observeTelemetry(_ value: RefloatTelemetryRecord) -> RefloatControls? {
    lock.withLock {
      guard var current = controls else { return nil }
      if let packageState = value.packageState {
        current.packageEnabled = packageState != .disabled
      }
      if let packageMode = value.packageMode {
        current.flywheelEnabled = packageMode == .flywheel
      }
      if let latched = value.fatalErrorLatched {
        current.fatalErrorLatched = latched
        current.fatalErrorActive = value.firmwareFault != nil
        current.fatalErrorClearable = latched && value.firmwareFault == nil
      }
      current.status = readiness(current) ? .ready : .loading
      controls = current
      confirmIfSatisfied()
      return controls
    }
  }

  public func observeLighting(_ value: RefloatLightingState) -> RefloatControls? {
    lock.withLock {
      guard var current = controls else { return nil }
      current.ledsEnabled = value.ledsEnabled
      current.headlightsEnabled = value.headlightsEnabled
      current.status = readiness(current) ? .ready : .loading
      controls = current
      confirmIfSatisfied()
      return controls
    }
  }

  public func authorizeSet(
    control: RefloatBooleanControl,
    enabled: Bool,
    context: RefloatMutationContext
  ) throws -> RefloatMutationAuthorization {
    try lock.withLock {
      var current = try validate(context)
      let action = RefloatPendingAction(rawValue: control.rawValue)!
      if current.pendingAction != nil || controllerPendingAction != nil || configurationPending {
        throw RefloatMutationError.invalidState
      }
      if current.flywheelEnabled == true && !(control == .flywheel && !enabled) {
        throw RefloatMutationError.invalidState
      }
      let activeFlywheelDisable = control == .flywheel && !enabled && context.packageMode == .flywheel
      if control == .package || control == .flywheel && !activeFlywheelDisable,
         context.rideState != .idle
      {
        throw RefloatMutationError.invalidState
      }
      if control == .flywheel, enabled,
         context.packageState != .ready || context.packageMode != .normal
      { throw RefloatMutationError.invalidState }
      if control == .flywheel, !enabled, context.packageMode != .flywheel {
        throw RefloatMutationError.invalidState
      }
      if control == .leds || control == .headlights, !current.lightingAvailable {
        throw RefloatMutationError.invalidState
      }
      let confirmed: Bool? = switch control {
      case .package: current.packageEnabled
      case .flywheel: current.flywheelEnabled
      case .leds: current.ledsEnabled
      case .headlights: current.headlightsEnabled
      }
      guard confirmed != enabled else { return .idempotent(current) }
      let payload: [UInt8]
      let oneWay: Bool
      switch control {
      case .package:
        payload = RefloatControlCodec.packageRequest(enabled: enabled)
        oneWay = true
      case .flywheel:
        payload = RefloatControlCodec.flywheelRequest(enabled: enabled)
        oneWay = true
      case .leds, .headlights:
        guard let leds = current.ledsEnabled, let headlights = current.headlightsEnabled else {
          throw RefloatMutationError.invalidState
        }
        payload = try RefloatControlCodec.lightingRequest(
          profile: context.profile, control: control, enabled: enabled,
          current: .init(ledsEnabled: leds, headlightsEnabled: headlights)
        )
        oneWay = false
      }
      current.pendingAction = action
      pendingExpected = enabled
      controls = current
      return .mutation(
        .init(
          action: action,
          payload: payload,
          oneWay: oneWay,
          expectedResponse: .init(
            command: VescCommand.customAppData.rawValue,
            refloatCommand: payload[2]
          )
        ),
        current
      )
    }
  }

  public func authorizeFatalClear(
    context: RefloatMutationContext
  ) throws -> RefloatMutationAuthorization {
    try lock.withLock {
      var current = try validate(context)
      guard current.pendingAction == nil,
            controllerPendingAction == nil,
            !configurationPending,
            current.flywheelEnabled != true,
            context.rideState == .idle,
            current.fatalErrorClearAvailable,
            current.fatalErrorLatched == true,
            current.fatalErrorActive == false,
            current.fatalErrorClearable == true
      else { throw RefloatMutationError.invalidState }
      let payload = try RefloatControlCodec.fatalClearRequest(profile: context.profile)
      current.pendingAction = .clearFatalError
      pendingExpected = false
      controls = current
      return .mutation(
        .init(
          action: .clearFatalError, payload: payload, oneWay: true,
          expectedResponse: .init(
            command: VescCommand.customAppData.rawValue,
            refloatCommand: payload[2]
          )
        ),
        current
      )
    }
  }

  public func failPending(generation: VescRequestGeneration) -> RefloatControls? {
    lock.withLock {
      guard generation == self.generation else { return controls }
      guard var current = controls, current.pendingAction != nil else { return nil }
      current.pendingAction = nil
      pendingExpected = nil
      controls = current
      return current
    }
  }

  public func cancel() -> RefloatControls? {
    lock.withLock {
      if var current = controls {
        current.pendingAction = nil; controls = current
      }
      pendingExpected = nil
      controllerPendingAction = nil
      configurationPending = false
      return controls
    }
  }

  private func validate(_ context: RefloatMutationContext) throws -> RefloatControls {
    guard context.generation == generation,
          context.connectionReady,
          context.writable,
          let current = controls,
          current.status == .ready,
          current.writable
    else { throw RefloatMutationError.invalidState }
    return current
  }

  private func readiness(_ value: RefloatControls) -> Bool {
    value.packageEnabled != nil && value.flywheelEnabled != nil
      && (!value.lightingAvailable || value.ledsEnabled != nil && value.headlightsEnabled != nil)
      && (!value.fatalErrorClearAvailable || value.fatalErrorLatched != nil && value.fatalErrorActive != nil)
  }

  private func confirmIfSatisfied() {
    guard var current = controls, let action = current.pendingAction else { return }
    let satisfied = switch action {
    case .package: current.packageEnabled == pendingExpected
    case .flywheel: current.flywheelEnabled == pendingExpected
    case .leds: current.ledsEnabled == pendingExpected
    case .headlights: current.headlightsEnabled == pendingExpected
    case .clearFatalError:
      pendingExpected == false && current.fatalErrorLatched == false && current.fatalErrorActive == false
    }
    if satisfied { current.pendingAction = nil; pendingExpected = nil; controls = current }
  }
}
