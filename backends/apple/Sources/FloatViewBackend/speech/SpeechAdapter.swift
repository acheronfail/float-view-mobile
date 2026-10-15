import AVFoundation
import Foundation

struct SpeechRequest: Decodable, Sendable {
  struct Load: Decodable, Sendable { let source: String; let percent: UInt64 }
  let expires_at_ms: UInt64
  let id: UInt64
  let load: Load?
  let speed: UInt64?
  let imperial: Bool
  let volume: UInt64
  let lower_other_audio: Bool
  var valid: Bool {
    id > 0 && volume <= 100 && (load != nil || speed != nil)
      && (load.map { ["duty", "motor", "battery"].contains($0.source) && $0.percent <= 100 } ?? true)
      && (speed.map { $0 <= 200 } ?? true)
  }

  var phrase: String {
    var parts: [String] = []
    if let load {
      parts.append("\(load.source) \(load.percent) percent")
    }
    if let speed {
      parts.append("speed \(speed) \(imperial ? "miles per hour" : "kilometres per hour")")
    }
    return parts.joined(separator: ", ")
  }
}

/// All synthesizer/session state is confined to the main queue; no product policy.
final class SpeechAdapter: NSObject, AVSpeechSynthesizerDelegate, @unchecked Sendable {
  private var synthesizer: AVSpeechSynthesizer?
  private let lease = SpeechLease()
  private var voice: AVSpeechSynthesisVoice?
  private var watchdog: DispatchWorkItem?
  private var observers: [NSObjectProtocol] = []
  var report: (@Sendable (String, UInt64, String?) -> Void)?

  func refresh() {
    DispatchQueue.main.async { self.refreshOnMain() }
  }

  private func refreshOnMain() {
    if synthesizer == nil {
      let synth = AVSpeechSynthesizer()
      synth.delegate = self
      synthesizer = synth
      #if os(iOS)
        if observers.isEmpty {
          for name in [
            AVAudioSession.interruptionNotification,
            AVAudioSession.routeChangeNotification,
            AVAudioSession.mediaServicesWereResetNotification,
          ] {
            observers
              .append(NotificationCenter.default
                .addObserver(forName: name, object: nil, queue: .main) { [weak self] notification in
                  // Category changes during our own activation do not interrupt us.
                  if name != AVAudioSession.routeChangeNotification {
                    self?.finish("interrupted")
                    if name == AVAudioSession.mediaServicesWereResetNotification {
                      self?.synthesizer = nil
                      self?.refreshOnMain()
                    }
                  } else if let raw = notification.userInfo?[AVAudioSessionRouteChangeReasonKey] as? UInt,
                            raw == AVAudioSession.RouteChangeReason.oldDeviceUnavailable
                            .rawValue
                  {
                    self?.finish("interrupted")
                  }
                })
          }
        }
      #endif
    }
    // Use an installed English system voice; phrases are English until app localisation exists.
    voice = AVSpeechSynthesisVoice(language: "en-AU") ?? AVSpeechSynthesisVoice(language: "en-US")
    report?(voice == nil ? "unavailable" : "ready", 0, voice == nil ? "no_offline_voice" : nil)
  }

  func speak(_ request: SpeechRequest) {
    DispatchQueue.main.async {
      guard request.valid, UInt64(ProcessInfo.processInfo.systemUptime * 1000) < request.expires_at_ms,
            self.lease.active == nil
      else { self.report?(
        "outcome",
        request.id,
        "unavailable"
      ); return }
      self.refreshOnMain()
      guard let voice = self.voice, let synth = self.synthesizer else { self.report?(
        "outcome",
        request.id,
        "unavailable"
      ); return }
      let utterance = AVSpeechUtterance(string: request.phrase)
      utterance.voice = voice
      utterance.volume = Float(request.volume) / 100
      guard self.lease.begin(id: request.id, utterance: utterance, cleanup: { [weak self] in
        self?.synthesizer?.stopSpeaking(at: .immediate)
        #if os(iOS)
          try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
        #endif
      }) else { return }
      #if os(iOS)
        do {
          synth.usesApplicationAudioSession = true
          let options: AVAudioSession.CategoryOptions = request.lower_other_audio
            ? [.duckOthers, .interruptSpokenAudioAndMixWithOthers] : [.mixWithOthers]
          try AVAudioSession.sharedInstance().setCategory(.playback, mode: .voicePrompt, options: options)
          try AVAudioSession.sharedInstance().setActive(true)
        } catch { self.finish("unavailable"); return }
      #endif
      let timeout = DispatchWorkItem { [weak self] in
        guard self?.lease.active?.0 == request.id else { return }
        self?.finish("unavailable")
      }
      self.watchdog = timeout
      DispatchQueue.main.asyncAfter(
        deadline: .now() + max(0, Double(request.expires_at_ms) / 1000 - ProcessInfo.processInfo.systemUptime),
        execute: timeout
      )
      synth.speak(utterance)
    }
  }

  func cancel(_ id: UInt64) {
    DispatchQueue.main.async {
      if self.lease.active?.0 == id {
        self.finish("interrupted")
      }
    }
  }

  func close() {
    DispatchQueue.main.async {
      self.finish("interrupted")
      self.observers.forEach(NotificationCenter.default.removeObserver)
      self.observers.removeAll()
      self.synthesizer = nil
    }
  }

  private func finish(_ outcome: String) {
    guard let (id, _) = lease.active else { return }
    lease.finish(id: id)
    watchdog?.cancel(); watchdog = nil
    report?("outcome", id, outcome)
  }

  func speechSynthesizer(_: AVSpeechSynthesizer, didFinish utterance: AVSpeechUtterance) {
    complete(utterance, outcome: "finished")
  }

  func speechSynthesizer(_: AVSpeechSynthesizer, didCancel utterance: AVSpeechUtterance) {
    complete(utterance, outcome: "interrupted")
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
