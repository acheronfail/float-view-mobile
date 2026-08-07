@testable import FloatViewBackend
import Foundation
import Testing

private let controlProfile = RefloatControlProfile(
  lighting: .stableCommand20, fatalClearAvailable: true
)

private func mutationContext(
  generation: UInt64 = 1,
  route: ControllerRoute = .direct,
  writable: Bool = true,
  rideState: RideState = .idle,
  packageState: PackageState = .ready,
  packageMode: PackageMode = .normal
) -> RefloatMutationContext {
  .init(
    generation: .init(rawValue: generation), route: route, profile: controlProfile,
    connectionReady: true, writable: writable, rideState: rideState,
    packageState: packageState, packageMode: packageMode
  )
}

private func readyCoordinator(
  packageMode: PackageMode = .normal,
  fatalLatched: Bool = false,
  fault: FirmwareFault? = nil
) -> DeviceMutationCoordinator {
  let owner = DeviceMutationCoordinator()
  _ = owner.configure(generation: .init(rawValue: 1), profile: controlProfile, writable: true)
  _ = owner.observeLighting(.init(ledsEnabled: false, headlightsEnabled: false))
  _ = owner.observeTelemetry(.init(
    rideState: .idle, packageState: .ready, packageMode: packageMode,
    firmwareFault: fault, fatalErrorLatched: fatalLatched
  ))
  return owner
}

@Test("control owner serializes direct and CAN writes and confirms from authoritative state")
func controlOwnerSerializesAndConfirms() throws {
  for route in [ControllerRoute.direct, .can(42)] {
    let owner = readyCoordinator()
    let authorization = try owner.authorizeSet(
      control: .package, enabled: false, context: mutationContext(route: route)
    )
    guard case let .mutation(mutation, pending) = authorization else {
      Issue.record("expected mutation"); return
    }
    #expect(mutation.oneWay)
    #expect(pending.pendingAction == .package)
    #expect(throws: RefloatMutationError.invalidState) {
      try owner.authorizeSet(control: .leds, enabled: true, context: mutationContext(route: route))
    }
    let confirmed = owner.observeTelemetry(.init(
      rideState: .idle, packageState: .disabled, packageMode: .normal,
      fatalErrorLatched: false
    ))
    #expect(confirmed?.pendingAction == nil)
    #expect(confirmed?.packageEnabled == false)
  }
}

@Test("flywheel mode rejects every mutation except eligible disable")
func flywheelModeHasSoleDisableException() throws {
  for control in RefloatBooleanControl.allCases {
    let owner = readyCoordinator(packageMode: .flywheel)
    if control == .flywheel {
      let result = try owner.authorizeSet(
        control: control, enabled: false,
        context: mutationContext(rideState: .riding, packageState: .running, packageMode: .flywheel)
      )
      guard case let .mutation(value, _) = result else { Issue.record("expected disable"); return }
      #expect(value.payload == RefloatControlCodec.flywheelRequest(enabled: false))
    } else {
      #expect(throws: RefloatMutationError.invalidState) {
        try owner.authorizeSet(
          control: control, enabled: true,
          context: mutationContext(rideState: .riding, packageState: .running, packageMode: .flywheel)
        )
      }
    }
  }
}

@Test("owner fails closed for motion, read-only, stale generations, and uncleared active faults")
func controlOwnerFailsClosed() throws {
  for context in [
    mutationContext(writable: false),
    mutationContext(rideState: .riding),
    mutationContext(generation: 2),
  ] {
    #expect(throws: RefloatMutationError.invalidState) {
      try readyCoordinator().authorizeSet(control: .package, enabled: false, context: context)
    }
  }
  let activeFault = readyCoordinator(fatalLatched: true, fault: .overVoltage)
  #expect(throws: RefloatMutationError.invalidState) {
    try activeFault.authorizeFatalClear(context: mutationContext())
  }
  let clearable = readyCoordinator(fatalLatched: true)
  guard case let .mutation(clear, _) = try clearable.authorizeFatalClear(context: mutationContext())
  else { Issue.record("expected clear"); return }
  #expect(clear.payload == [36, 101, 36, 1])
  #expect(clearable.failPending(generation: .init(rawValue: 2))?.pendingAction == .clearFatalError)
  #expect(clearable.failPending(generation: .init(rawValue: 1))?.pendingAction == nil)
}

@Test("shutdown shares idle, flywheel, generation, and one-pending-mutation gates")
func shutdownUsesDeviceMutationCoordinator() throws {
  let owner = readyCoordinator()
  let context = ControllerShutdownContext(
    generation: .init(rawValue: 1), connectionReady: true, shutdownSupported: true,
    rideState: .idle, packageMode: .normal
  )
  #expect(try owner.authorizeShutdown(context: context).pendingAction == .shutdown)
  #expect(throws: RefloatMutationError.invalidState) {
    try owner.authorizeSet(control: .leds, enabled: true, context: mutationContext())
  }
  #expect(owner.clearShutdown(generation: .init(rawValue: 2)) == nil)
  #expect(owner.controllerActions(shutdownAvailable: false).pendingAction == .shutdown)
  #expect(owner.clearShutdown(generation: .init(rawValue: 1))?.pendingAction == nil)

  for denied in [
    ControllerShutdownContext(
      generation: .init(rawValue: 2), connectionReady: true, shutdownSupported: true,
      rideState: .idle, packageMode: .normal
    ),
    ControllerShutdownContext(
      generation: .init(rawValue: 1), connectionReady: true, shutdownSupported: true,
      rideState: .riding, packageMode: .normal
    ),
    ControllerShutdownContext(
      generation: .init(rawValue: 1), connectionReady: true, shutdownSupported: true,
      rideState: .idle, packageMode: .flywheel
    ),
    ControllerShutdownContext(
      generation: .init(rawValue: 1), connectionReady: true, shutdownSupported: false,
      rideState: .idle, packageMode: .normal
    ),
  ] {
    #expect(throws: RefloatMutationError.invalidState) {
      try readyCoordinator().authorizeShutdown(context: denied)
    }
  }

  let competing = readyCoordinator()
  _ = try competing.authorizeSet(control: .leds, enabled: true, context: mutationContext())
  #expect(throws: RefloatMutationError.invalidState) {
    try competing.authorizeShutdown(context: context)
  }
}

@Test("configuration writes share the sole mutation gate and enforce idle normal footpad safety")
func configurationUsesDeviceMutationCoordinator() throws {
  let valid = RefloatConfigurationMutationContext(
    generation: .init(rawValue: 1), connectionReady: true, writable: true,
    rideState: .idle, footpadState: FootpadState.none, packageState: .ready, packageMode: .normal
  )
  let owner = readyCoordinator()
  try owner.authorizeConfiguration(context: valid)
  #expect(!owner.controllerActions(shutdownAvailable: true).shutdownAvailable)
  #expect(throws: RefloatMutationError.invalidState) {
    try owner.authorizeSet(control: .leds, enabled: true, context: mutationContext())
  }
  #expect(throws: RefloatMutationError.invalidState) {
    try owner.authorizeConfiguration(context: valid)
  }
  owner.finishConfiguration(generation: .init(rawValue: 2))
  #expect(throws: RefloatMutationError.invalidState) {
    try owner.authorizeConfiguration(context: valid)
  }
  owner.finishConfiguration(generation: .init(rawValue: 1))
  try owner.authorizeConfiguration(context: valid)

  for denied in [
    RefloatConfigurationMutationContext(
      generation: .init(rawValue: 2), connectionReady: true, writable: true,
      rideState: .idle, footpadState: FootpadState.none, packageState: .ready, packageMode: .normal
    ),
    RefloatConfigurationMutationContext(
      generation: .init(rawValue: 1), connectionReady: true, writable: true,
      rideState: .riding, footpadState: FootpadState.none, packageState: .running, packageMode: .normal
    ),
    RefloatConfigurationMutationContext(
      generation: .init(rawValue: 1), connectionReady: true, writable: true,
      rideState: .idle, footpadState: .left, packageState: .ready, packageMode: .normal
    ),
    RefloatConfigurationMutationContext(
      generation: .init(rawValue: 1), connectionReady: true, writable: true,
      rideState: .idle, footpadState: FootpadState.none, packageState: .ready, packageMode: .flywheel
    ),
  ] {
    #expect(throws: RefloatMutationError.invalidState) {
      try readyCoordinator().authorizeConfiguration(context: denied)
    }
  }
}
