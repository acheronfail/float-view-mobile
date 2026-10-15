import AVFoundation
@testable import FloatViewBackend
import Foundation
import Testing

@Suite("Announcement speech requests")
struct SpeechRequestTests {
  private func request(_ load: String = #"{"source":"motor","percent":78}"#, volume: Int = 42) throws -> SpeechRequest {
    try JSONDecoder().decode(SpeechRequest.self, from: Data("""
    {"id":1,"expires_at_ms":10000,"load":\(load),"speed":32,"imperial":false,"volume":\(volume),"lower_other_audio":true}
    """.utf8))
  }

  @Test func combinedPhraseIsLoadThenSpeed() throws {
    let value = try request()
    #expect(value.valid)
    #expect(value.phrase == "motor 78 percent, speed 32 kilometres per hour")
    #expect(value.volume == 42)
  }

  @Test func rejectsInvalidValuesAndSources() throws {
    #expect(try !request(volume: 101).valid)
    #expect(try !request(#"{"source":"other","percent":50}"#).valid)
    #expect(try !request(#"{"source":"battery","percent":101}"#).valid)
  }

  @Test func missingTypedFieldsCannotDecode() {
    #expect(throws: (any Error).self) {
      _ = try JSONDecoder().decode(SpeechRequest.self, from: Data(#"{"id":1,"text":"arbitrary"}"#.utf8))
    }
  }

  @Test func audioCleanupIsExactlyOnceAndIgnoresOldCallbacks() {
    let lease = SpeechLease()
    let first = AVSpeechUtterance(string: "first")
    let second = AVSpeechUtterance(string: "second")
    var stopped = 0
    var released = 0
    #expect(lease.begin(id: 1, utterance: first) { stopped += 1; released += 1 })
    #expect(!lease.begin(id: 2, utterance: second) { released += 100 })
    #expect(lease.finish(id: 1))
    #expect(!lease.finish(id: 1))
    #expect(stopped == 1 && released == 1)
    #expect(lease.begin(id: 2, utterance: second) { released += 1 })
    #expect(!lease.finish(id: 1))
    #expect(lease.active?.0 == 2)
    #expect(released == 1)
    #expect(lease.finish(id: 2))
    #expect(released == 2 && lease.active == nil)
  }
}
