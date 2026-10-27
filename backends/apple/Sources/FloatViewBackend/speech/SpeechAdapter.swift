import AVFoundation
import Foundation

struct SpeechRequest: Decodable, Sendable {
  enum Footpad: String, Decodable, Sendable {
    case left, right, both
    var phrase: String {
      switch self {
      case .left: "left footpad off"
      case .right: "right footpad off"
      case .both: "both footpads off"
      }
    }
  }
  enum Battery: String, Decodable, Sendable {
    case low, overVoltage
    var phrase: String { self == .low ? "Battery low" : "Over voltage" }
  }
  struct Load: Decodable, Sendable { let source: String; let percent: UInt64 }
  let expires_at_ms: UInt64
  let id: UInt64
  let load: Load?
  let speed: UInt64?
  let imperial: Bool
  let volume: UInt64
  let lower_other_audio: Bool
  var footpad: Footpad? = nil
  var battery: Battery? = nil
  var valid: Bool {
    id > 0 && volume <= 100 && (battery != nil || footpad != nil || load != nil || speed != nil)
      && (battery == nil || (footpad == nil && load == nil && speed == nil))
      && (load.map { ["duty", "motor", "battery"].contains($0.source) && $0.percent <= 100 } ?? true)
      && (speed.map { $0 <= 200 } ?? true)
  }

  var phrase: String {
    if let battery { return battery.phrase }
    var parts: [String] = []
    if let footpad { parts.append(footpad.phrase) }
    if let load {
      parts.append("\(load.source) \(load.percent) percent")
    }
    if let speed {
      parts.append("\(speed) \(imperial ? "miles per hour" : "kilometres per hour")")
    }
    return parts.joined(separator: ", ")
  }
}

/// Closed synthesis/session boundary; owned and invoked only on the main queue.
protocol SpeechDriver: AnyObject {
  var completion: ((AVSpeechUtterance, String) -> Void)? { get set }
  func refresh() -> Bool
  func start(_ utterance: AVSpeechUtterance, lowerOtherAudio: Bool) throws
  func stop()
  func releaseAudio()
}

private final class NativeSpeechDriver: NSObject, SpeechDriver, AVSpeechSynthesizerDelegate, @unchecked Sendable {
  var completion: ((AVSpeechUtterance, String) -> Void)?
  private let synthesizer = AVSpeechSynthesizer()
  override init() {
    super.init()
    synthesizer.delegate = self
  }

  func refresh() -> Bool {
    !AVSpeechSynthesisVoice.speechVoices().isEmpty
  }

  func start(_ utterance: AVSpeechUtterance, lowerOtherAudio: Bool) throws {
    // Leave voice selection to the system instead of forcing an Australian English voice.
    #if os(iOS)
      synthesizer.usesApplicationAudioSession = true
      let options: AVAudioSession.CategoryOptions = lowerOtherAudio
        ? [.duckOthers, .interruptSpokenAudioAndMixWithOthers] : [.mixWithOthers]
      try AVAudioSession.sharedInstance().setCategory(.playback, mode: .voicePrompt, options: options)
      try AVAudioSession.sharedInstance().setActive(true)
    #endif
    synthesizer.speak(utterance)
  }

  func stop() {
    synthesizer.stopSpeaking(at: .immediate)
  }

  func releaseAudio() {
    #if os(iOS)
      try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
    #endif
  }

  func speechSynthesizer(_: AVSpeechSynthesizer, didFinish utterance: AVSpeechUtterance) {
    completion?(utterance, "finished")
  }

  func speechSynthesizer(_: AVSpeechSynthesizer, didCancel utterance: AVSpeechUtterance) {
    completion?(utterance, "interrupted")
  }
}

/// All synthesizer/session state is confined to the main queue; no product policy.
final class SpeechAdapter: @unchecked Sendable {
  enum AudioEvent { case interruption, routeRemoved, mediaReset, routeChanged }
  private var driver: (any SpeechDriver)?
  private var ready = false
  private let createDriver: () throws -> any SpeechDriver
  private let now: () -> Double
  private let schedule: (Double, DispatchWorkItem) -> Void
  private let lease = SpeechLease()
  private var watchdog: DispatchWorkItem?
  private var observers: [NSObjectProtocol] = []
  private var closed = false
  var report: (@Sendable (String, UInt64, String?) -> Void)?

  init() {
    #if FLOAT_VIEW_CONFORMANCE
      createDriver = { ReplaySpeechDriver() }
    #else
      createDriver = { NativeSpeechDriver() }
    #endif
    now = { ProcessInfo.processInfo.systemUptime }
    schedule = { delay, item in DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: item) }
  }

  init(createDriver: @escaping () throws -> any SpeechDriver,
       now: @escaping () -> Double,
       schedule: @escaping (Double, DispatchWorkItem) -> Void)
  {
    self.createDriver = createDriver
    self.now = now
    self.schedule = schedule
  }

  func refresh() {
    DispatchQueue.main.async { self.refreshOnMain() }
  }

  func refreshOnMain() {
    guard !closed else { return }
    if driver == nil {
      do {
        let current = try createDriver()
        current.completion = { [weak self] utterance, outcome in self?.complete(utterance, outcome: outcome) }
        driver = current
      } catch { ready = false; report?("unavailable", 0, "engine_unavailable"); return }
      #if os(iOS)
        if observers.isEmpty {
          for name in [AVAudioSession.interruptionNotification, AVAudioSession.routeChangeNotification,
                       AVAudioSession.mediaServicesWereResetNotification]
          {
            observers.append(NotificationCenter.default.addObserver(forName: name, object: nil, queue: .main) {
              [weak self] notification in
              if name == AVAudioSession.mediaServicesWereResetNotification {
                self?.handleOnMain(.mediaReset)
              } else if name == AVAudioSession.interruptionNotification {
                self?.handleOnMain(.interruption)
              } else if let raw = notification.userInfo?[AVAudioSessionRouteChangeReasonKey] as? UInt,
                        raw == AVAudioSession.RouteChangeReason.oldDeviceUnavailable.rawValue
              {
                self?.handleOnMain(.routeRemoved)
              }
            })
          }
        }
      #endif
    }
    ready = driver?.refresh() == true
    report?(ready ? "ready" : "unavailable", 0, ready ? nil : "no_offline_voice")
  }

  func handleOnMain(_ event: AudioEvent) {
    guard !closed else { return }
    switch event {
    case .interruption, .routeRemoved: finish("interrupted")
    case .mediaReset:
      finish("interrupted")
      driver = nil
      refreshOnMain()
    case .routeChanged: break // Our own category changes must not interrupt speech.
    }
  }

  func speak(_ request: SpeechRequest) {
    DispatchQueue.main.async { self.speakOnMain(request) }
  }

  func speakOnMain(_ request: SpeechRequest) {
    guard !closed, request.valid, now() * 1000 < Double(request.expires_at_ms), lease.active == nil
    else { report?("outcome", request.id, "unavailable"); return }
    refreshOnMain()
    guard let current = driver, ready else {
      report?("outcome", request.id, "unavailable"); return
    }
    let utterance = AVSpeechUtterance(string: request.phrase)
    utterance.volume = Float(request.volume) / 100
    guard lease.begin(id: request.id, utterance: utterance, cleanup: {
      current.stop()
      current.releaseAudio()
    }) else { return }
    do { try current.start(utterance, lowerOtherAudio: request.lower_other_audio) }
    catch { finish("unavailable"); return }
    // A synchronous terminal callback from start must not leave a watchdog behind.
    guard lease.active?.0 == request.id else { return }
    let timeout = DispatchWorkItem { [weak self] in
      guard self?.lease.active?.0 == request.id else { return }
      self?.finish("unavailable")
    }
    watchdog = timeout
    schedule(max(0, Double(request.expires_at_ms) / 1000 - now()), timeout)
  }

  func cancel(_ id: UInt64) {
    DispatchQueue.main.async { self.cancelOnMain(id) }
  }

  func cancelOnMain(_ id: UInt64) {
    if lease.active?.0 == id {
      finish("interrupted")
    }
  }

  func close() {
    DispatchQueue.main.async { self.closeOnMain() }
  }

  func closeOnMain() {
    guard !closed else { return }
    closed = true
    finish("interrupted")
    observers.forEach(NotificationCenter.default.removeObserver)
    observers.removeAll()
    driver = nil
  }

  private func finish(_ outcome: String) {
    guard let (id, _) = lease.active else { return }
    lease.finish(id: id)
    watchdog?.cancel(); watchdog = nil
    report?("outcome", id, outcome)
  }

  private func complete(_ utterance: AVSpeechUtterance, outcome: String) {
    let identity = ObjectIdentifier(utterance)
    DispatchQueue.main.async {
      if self.lease.active.map({ ObjectIdentifier($0.1) }) == identity {
        self.finish(outcome)
      }
    }
  }
}
