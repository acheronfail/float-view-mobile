import AVFoundation
@testable import FloatViewBackend
import Foundation
import Testing

@Suite("Native speech failure paths")
@MainActor
struct SpeechAdapterTests {
  final class Driver: SpeechDriver {
    var completion: ((AVSpeechUtterance, String) -> Void)?
    var available = true
    var failStart = false
    var stopped = 0
    var released = 0
    var utterances: [AVSpeechUtterance] = []
    var lowering: [Bool] = []
    func refresh() -> Bool {
      available
    }

    func start(_ utterance: AVSpeechUtterance, lowerOtherAudio: Bool) throws {
      utterances.append(utterance); lowering.append(lowerOtherAudio)
      if failStart {
        throw NSError(domain: "speech-test", code: 1)
      }
    }

    func stop() {
      stopped += 1
    }

    func releaseAudio() {
      released += 1
    }
  }

  /// Reports are emitted on the main queue, just like the adapter's injected driver.
  final class Reports: @unchecked Sendable {
    var values: [(String, UInt64, String?)] = []
  }

  final class Rig {
    var time = 0.0
    var timers: [DispatchWorkItem] = []
    var delays: [Double] = []
    var drivers: [Driver] = []
    var failInit = false
    let reports = Reports()
    lazy var adapter = SpeechAdapter(createDriver: { [unowned self] in
      if self.failInit {
        throw NSError(domain: "speech-test", code: 2)
      }
      return self.drivers.last ?? self.addDriver()
    }, now: { [unowned self] in self.time }, schedule: { [unowned self] delay, item in
      self.delays.append(delay); self.timers.append(item)
    })
    func addDriver() -> Driver {
      let d = Driver(); drivers.append(d); return d
    }

    init() {
      adapter.report = { [reports] kind, id, reason in reports.values.append((kind, id, reason)) }
    }

    func request(_ id: UInt64 = 1, lower: Bool = true, volume: UInt64 = 50, expiry: UInt64 = 10000, footpadOnly: Bool = false) -> SpeechRequest {
      SpeechRequest(expires_at_ms: expiry, id: id, load: footpadOnly ? nil : .init(source: "motor", percent: 72), speed: footpadOnly ? nil : 25,
                    imperial: false, volume: volume, lower_other_audio: lower, footpad: footpadOnly ? .both : nil)
    }

    func outcomes(_ id: UInt64) -> [String?] {
      reports.values.filter { $0.0 == "outcome" && $0.1 == id }.map { $0.2 }
    }

    @MainActor func drain() async {
      await withCheckedContinuation { c in DispatchQueue.main.async { c.resume() } }
    }
  }

  @Test func missingEngineAndVoiceRecoverWithoutAudioAcquisition() {
    let r = Rig(); r.failInit = true; r.adapter.speakOnMain(r.request())
    #expect(r.outcomes(1) == ["unavailable"])
    #expect(r.reports.values.contains { $0.2 == "engine_unavailable" })
    r.failInit = false; let d = r.addDriver(); d.available = false
    r.adapter.speakOnMain(r.request(2)); #expect(d.utterances.isEmpty)
    #expect(r.reports.values.contains { $0.2 == "no_offline_voice" })
    d.available = true; r.adapter.refreshOnMain(); r.adapter.speakOnMain(r.request(3))
    #expect(d.utterances.count == 1); r.adapter.closeOnMain()
  }

  @Test func sessionActivationFailureReleasesResourcesAndCanRetry() {
    let r = Rig(); let d = r.addDriver(); d.failStart = true
    r.adapter.speakOnMain(r.request()); #expect(r.outcomes(1) == ["unavailable"])
    #expect(d.stopped == 1 && d.released == 1)
    d.failStart = false; r.adapter.speakOnMain(r.request(2)); r.adapter.cancelOnMain(2)
    #expect(d.released == 2)
  }

  @Test func allTerminalPathsReleaseExactlyOnce() async {
    for terminal in ["finished", "interrupted", "cancel", "route", "reset", "timeout", "close"] {
      let r = Rig(); let d = r.addDriver(); r.adapter.speakOnMain(r.request(footpadOnly: true))
      let utterance = d.utterances[0]
      #expect(utterance.speechString == "both footpads off")
      switch terminal {
      case "cancel": r.adapter.cancelOnMain(1)
      case "route": r.adapter.handleOnMain(.routeRemoved)
      case "reset": r.adapter.handleOnMain(.mediaReset)
      case "timeout": r.timers[0].perform()
      case "close": r.adapter.closeOnMain()
      default: d.completion?(utterance, terminal); await r.drain()
      }
      d.completion?(utterance, "finished"); await r.drain()
      r.adapter.cancelOnMain(1); r.timers.forEach { $0.perform() }
      #expect(d.stopped == 1 && d.released == 1, "\(terminal)")
      #expect(r.outcomes(1).count == 1, "\(terminal)")
      r.adapter.closeOnMain()
    }
  }

  @Test func interruptionAndObsoleteCallbacksCannotReleaseReplacement() async {
    let r = Rig(); let d = r.addDriver(); r.adapter.speakOnMain(r.request())
    let old = d.utterances[0]; let deadline = r.timers[0]
    r.adapter.handleOnMain(.interruption)
    r.time = 1; r.adapter.speakOnMain(r.request(2, expiry: 11000))
    d.completion?(old, "finished"); await r.drain(); deadline.perform(); r.adapter.cancelOnMain(1)
    #expect(d.released == 1 && r.outcomes(2).isEmpty)
    d.completion?(d.utterances[1], "finished"); await r.drain()
    #expect(d.released == 2 && r.outcomes(2) == ["finished"])
  }

  @Test func expiredBusyAndClosedRequestsCannotStartOrRecreateResources() {
    let r = Rig(); let d = r.addDriver()
    r.adapter.speakOnMain(r.request(expiry: 0)); #expect(d.utterances.isEmpty)
    r.adapter.speakOnMain(r.request(2)); r.adapter.speakOnMain(r.request(3))
    #expect(d.utterances.count == 1); #expect(r.outcomes(3) == ["unavailable"])
    r.adapter.closeOnMain(); r.adapter.closeOnMain(); r.adapter.refreshOnMain()
    r.adapter.speakOnMain(r.request(4)); #expect(d.released == 1 && d.utterances.count == 1)
  }

  @Test func loweringGainAndAdmissionDeadlineAreCapturedPerUtterance() {
    let r = Rig(); let d = r.addDriver(); r.time = 2
    r.adapter.speakOnMain(r.request(lower: true, volume: 50))
    #expect(d.lowering == [true]); #expect(d.utterances[0].volume == 0.5)
    #expect(r.delays == [8]); r.adapter.handleOnMain(.routeChanged)
    #expect(d.released == 0)
    r.adapter.cancelOnMain(1); r.adapter.speakOnMain(r.request(2, lower: false, volume: 100))
    #expect(d.lowering == [true, false]); #expect(d.utterances[1].volume == 1)
    r.adapter.closeOnMain(); #expect(d.released == 2)
  }

  @Test func mediaResetRefreshesDriverAndIgnoresOldCompletion() async {
    let r = Rig(); let old = r.addDriver(); r.adapter.speakOnMain(r.request())
    let replacement = r.addDriver(); r.adapter.handleOnMain(.mediaReset)
    r.adapter.speakOnMain(r.request(2)); old.completion?(old.utterances[0], "finished"); await r.drain()
    #expect(old.released == 1 && replacement.released == 0)
    r.adapter.closeOnMain(); #expect(replacement.released == 1)
  }
}
