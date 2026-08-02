@testable import FloatViewBackend
import XCTest

final class SessionHistoryReducerTests: XCTestCase {
  private let snapshot = SessionPublishedSnapshotRecord(
    historyRevision: 7,
    committedSampleSequence: 18,
    committedEventSequence: 4,
    committedElapsedMs: 10_000,
    recordedSampleCount: 18,
    storedSampleCount: 18
  )

  func testM4RolesGapsAndLateFieldsMatchCanonicalVector() throws {
    let rows = [
      SessionSeriesPoint(elapsedMs: 0, values: ["speedKph": 0, "packVoltageV": 60, "extensionLate": nil], sequence: 10),
      SessionSeriesPoint(elapsedMs: 0, values: ["speedKph": 1, "packVoltageV": 59, "extensionLate": nil], sequence: 11),
      SessionSeriesPoint(
        elapsedMs: 999,
        values: ["speedKph": 10, "packVoltageV": 58, "extensionLate": 0.5],
        sequence: 12
      ),
      SessionSeriesPoint(
        elapsedMs: 1_000,
        values: ["speedKph": 20, "packVoltageV": 57, "extensionLate": 0.6],
        sequence: 13
      ),
      SessionSeriesPoint(
        elapsedMs: 1_001,
        values: ["speedKph": 5, "packVoltageV": 56, "extensionLate": 0.7],
        sequence: 14
      ),
      SessionSeriesPoint(
        elapsedMs: 3_500,
        values: ["speedKph": nil, "packVoltageV": 55, "extensionLate": 0.8],
        sequence: 15
      ),
      SessionSeriesPoint(
        elapsedMs: 3_501,
        values: ["speedKph": 7, "packVoltageV": nil, "extensionLate": 0.9],
        sequence: 16
      ),
      SessionSeriesPoint(
        elapsedMs: 10_000,
        values: ["speedKph": 2, "packVoltageV": 54, "extensionLate": 1],
        sequence: 17
      ),
      SessionSeriesPoint(
        elapsedMs: 10_000,
        values: ["speedKph": 3, "packVoltageV": 53, "extensionLate": 1.1],
        sequence: 18
      ),
    ]
    let result = try SessionHistoryReducer.reduceSeries(
      rows: rows, fields: ["speedKph", "packVoltageV", "extensionLate"],
      range: SessionElapsedRange(startElapsedMs: 0, endElapsedMs: 10_000), buckets: 4,
      gapPolicy: SessionGapPolicyRecord(thresholdSeconds: 2)
    )
    XCTAssertEqual(result["speedKph"]?.values.compactMap { $0 }, [0, 20, 5, 7, 2, 3])
    XCTAssertEqual(result["speedKph"]?.generations, [0, 0, 0, 2, 3, 3])
    XCTAssertEqual(result["packVoltageV"]?.values.compactMap { $0 }, [60, 56, 55, 54, 53])
    XCTAssertEqual(result["packVoltageV"]?.generations, [0, 0, 1, 3, 3])
    XCTAssertEqual(result["extensionLate"]?.values.compactMap { $0 }, [0.5, 0.7, 0.8, 0.9, 1, 1.1])
  }

  func testNearestTieAndCursorPageReplayAreDeterministic() throws {
    let rows = [
      SessionSeriesPoint(elapsedMs: 900, values: [:], sequence: 3),
      SessionSeriesPoint(elapsedMs: 900, values: [:], sequence: 2),
      SessionSeriesPoint(elapsedMs: 1_100, values: [:], sequence: 9),
    ]
    XCTAssertEqual(SessionHistoryReducer.nearestSample(rows: rows, targetElapsedMs: 1_000)?.sequence, 2)

    let range = SessionElapsedRange(startElapsedMs: 0, endElapsedMs: 10_000)
    let seed = SessionHistoryCursor(
      sessionID: "session-1", snapshot: snapshot, range: range, elapsedMs: 0, sequence: 0
    )
    let first = try SessionHistoryReducer.page(rows: [
      SessionSeriesPoint(elapsedMs: 0, values: [:], sequence: 1),
      SessionSeriesPoint(elapsedMs: 500, values: [:], sequence: 2),
      SessionSeriesPoint(elapsedMs: 1_000, values: [:], sequence: 3),
    ], limit: 2, cursorContext: seed)
    XCTAssertEqual(first.rows.map(\.sequence), [1, 2])
    let encoded = try SessionHistoryReducer.encodeCursor(XCTUnwrap(first.next))
    let decoded = try SessionHistoryReducer.decodeCursor(encoded)
    XCTAssertEqual(decoded.sequence, 2)
    let second = try SessionHistoryReducer.page(rows: [
      SessionSeriesPoint(elapsedMs: 0, values: [:], sequence: 1),
      SessionSeriesPoint(elapsedMs: 500, values: [:], sequence: 2),
      SessionSeriesPoint(elapsedMs: 1_000, values: [:], sequence: 3),
    ], limit: 2, cursor: decoded)
    XCTAssertEqual(second.rows.map(\.sequence), [3])
    XCTAssertThrowsError(try SessionHistoryReducer.decodeCursor("not-a-cursor"))
  }

  func testPruneSelectionProtectsBoundariesEventsAndFieldStarts() throws {
    var rows: [SessionSeriesPoint] = []
    for index in 0 ..< 24 {
      var values: [String: Double?] = [:]
      values["speedKph"] = index == 6 ? nil : Double(index * 3 % 19)
      values["packVoltageV"] = 60 - Double(index) * 0.2
      values["extensionLate"] = index >= 12 ? Double(index) / 10 : nil
      rows.append(SessionSeriesPoint(
        elapsedMs: Int64(index * 500), values: values, sequence: Int64(index),
        rideState: index < 12 ? "riding" : "idle"
      ))
    }
    let selected = try SessionHistoryReducer.selectPruneRows(
      rows: rows,
      fields: ["speedKph", "packVoltageV", "extensionLate"],
      targetResolutionSeconds: 5,
      protectedSequences: [5, 12, 13],
      eventSourceSequences: [5, 12]
    )
    XCTAssertEqual(selected, [0, 5, 9, 10, 12, 13, 19, 20, 23])
  }
}
