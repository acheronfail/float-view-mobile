import AVFoundation

/// Main-queue resource ownership; a delayed callback cannot release replacement audio.
final class SpeechLease {
  private(set) var active: (UInt64, AVSpeechUtterance)?
  private var cleanup: (() -> Void)?
  func begin(id: UInt64, utterance: AVSpeechUtterance, cleanup: @escaping () -> Void) -> Bool {
    guard active == nil else { return false }
    active = (id, utterance)
    self.cleanup = cleanup
    return true
  }

  @discardableResult func finish(id: UInt64) -> Bool {
    guard active?.0 == id else { return false }
    active = nil
    let release = cleanup
    cleanup = nil
    release?()
    return true
  }
}
