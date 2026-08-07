@testable import FloatViewCoreBridge
import Foundation
import XCTest

final class FloatViewCoreRuntimeTests: XCTestCase {
  func testPumpRunsOffMainAndUsesReplacementPresentation() throws {
    for cycle in 0 ..< 2 {
      let runtime = try FloatViewCoreRuntime()
      runtime.replacePresentation(generation: UInt64(cycle + 1))
      let received = expectation(description: "command (cycle)")
      try runtime.start { command in
        XCTAssertFalse(Thread.isMainThread)
        XCTAssertEqual(command.kind, 42)
        XCTAssertEqual(command.flags, 0)
        XCTAssertGreaterThan(command.runtimeGeneration, 0)
        XCTAssertEqual(command.payload, Data([1, 2, 3]))
        XCTAssertEqual(command.presentationGeneration, UInt64(cycle + 1))
        try? runtime.complete(command)
        received.fulfill()
      }
      try runtime.submitTestCommand(kind: 42, payload: Data([1, 2, 3]))
      wait(for: [received], timeout: 2)
      try runtime.stop()
    }
  }

  func testCallerBuffersResizeCopyAndRejectDuplicateCompletion() throws {
    let runtime = try FloatViewCoreRuntime()
    let received = expectation(description: "large copied command")
    let payload = Data(repeating: 0xA5, count: 2_048)
    try runtime.start { command in
      XCTAssertEqual(command.payload, payload)
      do {
        try runtime.complete(command, resultKind: 7, payload: Data([9, 8, 7]))
      } catch {
        XCTFail("Completion failed: \(error)")
      }
      do {
        try runtime.complete(command)
        XCTFail("Duplicate completion unexpectedly succeeded")
      } catch {
        XCTAssertEqual(error as? FloatViewCoreError, .native(.duplicateOperation))
      }
      received.fulfill()
    }
    try runtime.submitTestCommand(kind: 99, payload: payload)
    wait(for: [received], timeout: 2)
    XCTAssertNil(runtime.commandPumpFailure)
    try runtime.stop()
  }

  func testPayloadBoundAndStopWakeAreStable() throws {
    let runtime = try FloatViewCoreRuntime()
    try runtime.start { _ in
      XCTFail("No command expected")
    }
    XCTAssertThrowsError(
      try runtime.submitTestCommand(
        kind: 1,
        payload: Data(repeating: 0, count: FloatViewCoreRuntime.maximumPayloadBytes + 1)
      )
    ) { error in
      XCTAssertEqual(error as? FloatViewCoreError, .payloadTooLarge)
    }
    let started = ContinuousClock.now
    try runtime.stop()
    XCTAssertLessThan(started.duration(to: .now), .seconds(1))
  }

  func testProcessRuntimeReplacementRejectsPriorGenerationCommands() throws {
    let first = try FloatViewCoreRuntime()
    let received = expectation(description: "first runtime command")
    let capture = CommandCapture()
    try first.start { command in
      capture.store(command)
      received.fulfill()
    }
    try first.submitTestCommand(kind: 55, payload: Data([1]))
    wait(for: [received], timeout: 2)
    let firstGeneration = first.generation
    try first.stop()

    let second = try FloatViewCoreRuntime()
    try second.start { _ in XCTFail("No replacement command expected") }
    XCTAssertGreaterThan(second.generation, firstGeneration)
    let priorCommand = try XCTUnwrap(capture.value)
    XCTAssertThrowsError(try second.complete(priorCommand)) { error in
      XCTAssertEqual(error as? FloatViewCoreError, .native(.staleGeneration))
    }
    try second.stop()
  }
}

private final class CommandCapture: @unchecked Sendable {
  private let lock = NSLock()
  private var command: FloatViewHostCommand?
  var value: FloatViewHostCommand? {
    lock.withLock { command }
  }

  func store(_ value: FloatViewHostCommand) {
    lock.withLock { command = value }
  }
}
