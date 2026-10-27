#if FLOAT_VIEW_CONFORMANCE
import AVFoundation
import Foundation

/// Software-host playback boundary, excluded from production and physical builds.
final class ReplaySpeechDriver: SpeechDriver, @unchecked Sendable {
  var completion: ((AVSpeechUtterance, String) -> Void)?
  private var pending: AVSpeechUtterance?
  private var epoch: UInt64 = 0

  func refresh() -> Bool { true }

  func start(_ utterance: AVSpeechUtterance, lowerOtherAudio: Bool) throws {
    epoch += 1
    let operation = epoch
    pending = utterance
    let data = try JSONSerialization.data(withJSONObject: ["phrase": utterance.speechString])
    FileHandle.standardError.write(Data("FLOAT_VIEW_SPEECH_REPLAY ".utf8) + data + Data("\n".utf8))
    DispatchQueue.main.async { [weak self] in
      guard let self, self.epoch == operation, let current = self.pending else { return }
      self.pending = nil
      self.completion?(current, "finished")
    }
  }

  func stop() { epoch += 1; pending = nil }
  func releaseAudio() {}
}
#endif
