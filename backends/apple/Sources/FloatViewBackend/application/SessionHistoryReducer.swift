import Foundation

public struct SessionElapsedRange: Sendable, Codable, Equatable {
  public let startElapsedMs: Int64
  public let endElapsedMs: Int64

  public init(startElapsedMs: Int64, endElapsedMs: Int64) {
    self.startElapsedMs = startElapsedMs
    self.endElapsedMs = endElapsedMs
  }
}

public struct SessionGapPolicyRecord: Sendable, Codable, Equatable {
  public let thresholdSeconds: Double?

  public init(thresholdSeconds: Double?) {
    self.thresholdSeconds = thresholdSeconds
  }
}

public struct SessionSeriesColumnRecord: Sendable, Codable, Equatable {
  public let elapsedMs: [Int64]
  public let sequence: [Int64]
  public let values: [Double?]
  public let generations: [Int]

  public init(
    elapsedMs: [Int64],
    sequence: [Int64],
    values: [Double?],
    generations: [Int]
  ) {
    self.elapsedMs = elapsedMs
    self.sequence = sequence
    self.values = values
    self.generations = generations
  }
}

public struct SessionHistoryCursor: Sendable, Codable, Equatable {
  public let version: Int
  public let sessionID: String
  public let snapshot: SessionPublishedSnapshotRecord
  public let range: SessionElapsedRange
  public let elapsedMs: Int64
  public let sequence: Int64

  public init(
    sessionID: String,
    snapshot: SessionPublishedSnapshotRecord,
    range: SessionElapsedRange,
    elapsedMs: Int64,
    sequence: Int64
  ) {
    version = 1
    self.sessionID = sessionID
    self.snapshot = snapshot
    self.range = range
    self.elapsedMs = elapsedMs
    self.sequence = sequence
  }
}

public enum SessionHistoryReducer {
  private struct Candidate {
    let elapsedMs: Int64
    let sequence: Int64
    let value: Double
    let generation: Int
  }

  private struct BucketCandidates {
    var first: Candidate?
    var minimum: Candidate?
    var maximum: Candidate?
    var last: Candidate?
  }

  public static let maximumSeriesObservations = 65_536

  /// A bounded, ordered reducer for rows supplied directly by a historical
  /// reader. It retains only the four candidate roles for each field/bucket,
  /// rather than keeping the complete source session in memory.
  public final class SeriesAccumulator {
    private let fields: [String]
    private let range: SessionElapsedRange
    private let buckets: Int
    private let configuredThresholdMs: Int64?
    private let expectedPersistenceMs: Int64
    private let expectedRidingPersistenceMs: Int64?
    private let expectedIdlePersistenceMs: Int64?
    private let retainedResolutionSeconds: Int64
    private var generations: [Int]
    private var bucketsByField: [[BucketCandidates?]]
    private var previousElapsed: Int64?
    private var previousRideState: String?

    public init(
      fields: [String],
      range: SessionElapsedRange,
      buckets: Int,
      gapPolicy: SessionGapPolicyRecord,
      expectedPersistenceMs: Int64 = 1_000,
      expectedRidingPersistenceMs: Int64? = nil,
      expectedIdlePersistenceMs: Int64? = nil,
      retainedResolutionSeconds: Int64 = 0
    ) throws {
      guard range.startElapsedMs <= range.endElapsedMs,
            (1 ... 1_024).contains(buckets),
            (1 ... 16).contains(fields.count),
            Set(fields).count == fields.count
      else { throw SessionStoreError.invalidRange }
      if let thresholdSeconds = gapPolicy.thresholdSeconds {
        guard thresholdSeconds.isFinite, thresholdSeconds >= 0.1, thresholdSeconds <= 3_600 else {
          throw SessionStoreError.invalidRange
        }
        configuredThresholdMs = max(1, Int64((thresholdSeconds * 1_000).rounded()))
      } else {
        if let expectedRidingPersistenceMs {
          guard expectedRidingPersistenceMs > 0 else { throw SessionStoreError.invalidRange }
        }
        if let expectedIdlePersistenceMs {
          guard expectedIdlePersistenceMs > 0 else { throw SessionStoreError.invalidRange }
        }
        configuredThresholdMs = nil
      }
      self.fields = fields
      self.range = range
      self.buckets = buckets
      self.expectedPersistenceMs = expectedPersistenceMs
      self.expectedRidingPersistenceMs = expectedRidingPersistenceMs
      self.expectedIdlePersistenceMs = expectedIdlePersistenceMs
      self.retainedResolutionSeconds = retainedResolutionSeconds
      generations = Array(repeating: 0, count: fields.count)
      bucketsByField = fields.map { _ in Array<BucketCandidates?>(repeating: nil, count: buckets) }
    }

    public func append(_ row: SessionSeriesPoint) {
      append(
        elapsedMs: row.elapsedMs,
        sequence: row.sequence,
        rideState: row.rideState,
        values: fields.map { row.values[$0] ?? nil }
      )
    }

    public func append(
      elapsedMs: Int64,
      sequence: Int64,
      rideState: String?,
      values: [Double?]
    ) {
      let edgeThreshold = thresholdForCurrentPhase()
      if let previousElapsed, elapsedMs - previousElapsed > edgeThreshold {
        for index in generations.indices { generations[index] += 1 }
      }
      previousElapsed = elapsedMs
      previousRideState = rideState
      let span = max(1, range.endElapsedMs - range.startElapsedMs + 1)
      let offset = max(0, elapsedMs - range.startElapsedMs)
      let bucket = min(buckets - 1, Int((offset * Int64(buckets)) / span))
      for index in fields.indices {
        guard let value = values[index] else {
          generations[index] += 1
          continue
        }
        let candidate = Candidate(
          elapsedMs: elapsedMs,
          sequence: sequence,
          value: value,
          generation: generations[index]
        )
        var bucketCandidates = bucketsByField[index][bucket] ?? BucketCandidates()
        if bucketCandidates.first == nil || SessionHistoryReducer.compareCandidate(candidate, bucketCandidates.first!) {
          bucketCandidates.first = candidate
        }
        if bucketCandidates.minimum == nil || candidate.value < bucketCandidates.minimum!.value ||
          (candidate.value == bucketCandidates.minimum!.value &&
            SessionHistoryReducer.compareCandidate(candidate, bucketCandidates.minimum!))
        {
          bucketCandidates.minimum = candidate
        }
        if bucketCandidates.maximum == nil || candidate.value > bucketCandidates.maximum!.value ||
          (candidate.value == bucketCandidates.maximum!.value &&
            SessionHistoryReducer.compareCandidate(candidate, bucketCandidates.maximum!))
        {
          bucketCandidates.maximum = candidate
        }
        if bucketCandidates.last == nil ||
          SessionHistoryReducer.compareCandidate(bucketCandidates.last!, candidate)
        {
          bucketCandidates.last = candidate
        }
        bucketsByField[index][bucket] = bucketCandidates
      }
    }

    public func finish() throws -> [String: SessionSeriesColumnRecord] {
      var result: [String: SessionSeriesColumnRecord] = [:]
      var observationCount = 0
      for (index, field) in fields.enumerated() {
        var selected: [Int64: Candidate] = [:]
        for bucketCandidates in bucketsByField[index] {
          for candidate in [
            bucketCandidates?.first,
            bucketCandidates?.minimum,
            bucketCandidates?.maximum,
            bucketCandidates?.last,
          ].compactMap({ $0 }) {
            selected[candidate.sequence] = candidate
          }
        }
        let orderedSelected = selected.values.sorted {
          SessionHistoryReducer.compare($0.elapsedMs, $0.sequence, $1.elapsedMs, $1.sequence)
        }
        observationCount += orderedSelected.count
        if observationCount > SessionHistoryReducer.maximumSeriesObservations {
          throw SessionStoreError.invalidRange
        }
        result[field] = SessionSeriesColumnRecord(
          elapsedMs: orderedSelected.map { $0.elapsedMs },
          sequence: orderedSelected.map { $0.sequence },
          values: orderedSelected.map { $0.value },
          generations: orderedSelected.map { $0.generation }
        )
      }
      return result
    }

    private func thresholdForCurrentPhase() -> Int64 {
      if let configuredThresholdMs { return configuredThresholdMs }
      let persistence = previousRideState == "riding"
        ? (expectedRidingPersistenceMs ?? expectedPersistenceMs)
        : (expectedIdlePersistenceMs ?? expectedPersistenceMs)
      return max(
        1_000,
        max(1, persistence) * 3,
        max(0, retainedResolutionSeconds) * 3 * 1_000
      )
    }
  }

  public static func reduceSeries(
    rows: [SessionSeriesPoint],
    fields: [String],
    range: SessionElapsedRange,
    buckets: Int,
    gapPolicy: SessionGapPolicyRecord,
    expectedPersistenceMs: Int64 = 1_000,
    expectedRidingPersistenceMs: Int64? = nil,
    expectedIdlePersistenceMs: Int64? = nil,
    retainedResolutionSeconds: Int64 = 0,
    rowsAreOrdered: Bool = false
  ) throws -> [String: SessionSeriesColumnRecord] {
    let accumulator = try SeriesAccumulator(
      fields: fields,
      range: range,
      buckets: buckets,
      gapPolicy: gapPolicy,
      expectedPersistenceMs: expectedPersistenceMs,
      expectedRidingPersistenceMs: expectedRidingPersistenceMs,
      expectedIdlePersistenceMs: expectedIdlePersistenceMs,
      retainedResolutionSeconds: retainedResolutionSeconds
    )
    let ordered = rowsAreOrdered
      ? rows
        : rows
        .filter { $0.elapsedMs >= range.startElapsedMs && $0.elapsedMs <= range.endElapsedMs }
        .sorted(by: compare)
    for row in ordered { accumulator.append(row) }
    return try accumulator.finish()
  }

  public static func nearestSample(
    rows: [SessionSeriesPoint],
    targetElapsedMs: Int64
  ) -> SessionSeriesPoint? {
    rows.min { left, right in
      let leftDistance = abs(Double(left.elapsedMs) - Double(targetElapsedMs))
      let rightDistance = abs(Double(right.elapsedMs) - Double(targetElapsedMs))
      if leftDistance != rightDistance { return leftDistance < rightDistance }
      return compare(left, right)
    }
  }

  /// Selects real source rows for a deterministic, bounded-resolution prune.
  /// Semantic evidence is protected first; remaining bucket roles are covered
  /// greedily with at most eight additional rows per bucket.
  public static func selectPruneRows(
    rows: [SessionSeriesPoint],
    fields: [String],
    targetResolutionSeconds: Int64,
    protectedSequences: Set<Int64> = [],
    eventSourceSequences: Set<Int64> = []
  ) throws -> [Int64] {
    guard (1 ... 300).contains(targetResolutionSeconds), !fields.isEmpty, fields.count <= 16 else {
      throw SessionStoreError.invalidRange
    }
    let ordered = rows.sorted(by: compare)
    guard !ordered.isEmpty else { return [] }
    var keep = protectedSequences.union(eventSourceSequences)
    keep.insert(ordered[0].sequence)
    keep.insert(ordered[ordered.count - 1].sequence)
    for field in fields {
      if let first = ordered.first(where: { $0.values[field] ?? nil != nil }) {
        keep.insert(first.sequence)
      }
    }
    var buckets: [Int64: [SessionSeriesPoint]] = [:]
    let width = targetResolutionSeconds * 1_000
    for row in ordered {
      buckets[row.elapsedMs / width, default: []].append(row)
    }
    for bucketRows in buckets.values {
      guard let first = bucketRows.first, let last = bucketRows.last else { continue }
      keep.insert(first.sequence)
      keep.insert(last.sequence)
      var roles: [(String, SessionSeriesPoint)] = [("first", first), ("last", last)]
      for field in fields {
        let values = bucketRows.compactMap { row -> (SessionSeriesPoint, Double)? in
          guard let value = row.values[field] ?? nil else { return nil }
          return (row, value)
        }
        if let minimum = values.min(by: { $0.1 == $1.1 ? compare($0.0, $1.0) : $0.1 < $1.1 }) {
          roles.append(("\(field):min", minimum.0))
        }
        if let maximum = values.max(by: { $0.1 == $1.1 ? compare($0.0, $1.0) : $0.1 < $1.1 }) {
          roles.append(("\(field):max", maximum.0))
        }
      }
      var uncovered = Set(roles.map(\.0))
      var candidates = Array(Set(roles.map(\.1.sequence))).compactMap { sequence in
        bucketRows.first(where: { $0.sequence == sequence })
      }.filter { !keep.contains($0.sequence) }
      while !uncovered.isEmpty, candidates.count > 0 {
        candidates.sort { left, right in
          let leftCoverage = roles.filter { role in
            role.1.sequence == left.sequence && uncovered.contains(role.0)
          }.count
          let rightCoverage = roles.filter { role in
            role.1.sequence == right.sequence && uncovered.contains(role.0)
          }.count
          return leftCoverage != rightCoverage ? leftCoverage > rightCoverage : compare(left, right)
        }
        let choice = candidates.removeFirst()
        keep.insert(choice.sequence)
        roles.filter { $0.1.sequence == choice.sequence }.forEach { uncovered.remove($0.0) }
      }
    }
    return ordered.filter { keep.contains($0.sequence) }.map(\.sequence)
  }

  public static func encodeCursor(_ cursor: SessionHistoryCursor) throws -> String {
    let data = try JSONEncoder().encode(cursor)
    let value = data.base64EncodedString()
      .replacingOccurrences(of: "+", with: "-")
      .replacingOccurrences(of: "/", with: "_")
      .trimmingCharacters(in: CharacterSet(charactersIn: "="))
    guard value.utf8.count <= 4_096 else { throw SessionStoreError.invalidCursor }
    return value
  }

  public static func decodeCursor(_ value: String) throws -> SessionHistoryCursor {
    guard !value.isEmpty, value.utf8.count <= 4_096 else { throw SessionStoreError.invalidCursor }
    let padded = value.replacingOccurrences(of: "-", with: "+")
      .replacingOccurrences(of: "_", with: "/")
      + String(repeating: "=", count: (4 - value.count % 4) % 4)
    guard let data = Data(base64Encoded: padded),
          let cursor = try? JSONDecoder().decode(SessionHistoryCursor.self, from: data),
          cursor.version == 1,
          cursor.range.startElapsedMs <= cursor.range.endElapsedMs,
          cursor.elapsedMs >= cursor.range.startElapsedMs,
          cursor.elapsedMs <= cursor.range.endElapsedMs,
          cursor.sequence >= 0
    else { throw SessionStoreError.invalidCursor }
    return cursor
  }

  public static func page(
    rows: [SessionSeriesPoint],
    limit: Int,
    cursor: SessionHistoryCursor? = nil,
    cursorContext: SessionHistoryCursor? = nil
  ) throws -> (rows: [SessionSeriesPoint], next: SessionHistoryCursor?) {
    guard (1 ... 2_000).contains(limit) else { throw SessionStoreError.invalidRange }
    let ordered = rows.sorted(by: compare)
    let start = cursor.map { value in
      let boundary = SessionSeriesPoint(elapsedMs: value.elapsedMs, values: [:], sequence: value.sequence)
      return ordered.firstIndex { compare(boundary, $0) } ?? ordered.count
    } ?? 0
    let page = Array(ordered.dropFirst(start).prefix(limit))
    guard let last = page.last, start + page.count < ordered.count,
          let context = cursor ?? cursorContext
    else {
      return (page, nil)
    }
    return (page, SessionHistoryCursor(
      sessionID: context.sessionID, snapshot: context.snapshot, range: context.range,
      elapsedMs: last.elapsedMs, sequence: last.sequence
    ))
  }

  private static func compare(_ left: SessionSeriesPoint, _ right: SessionSeriesPoint) -> Bool {
    left.elapsedMs < right.elapsedMs || (left.elapsedMs == right.elapsedMs && left.sequence < right.sequence)
  }

  private static func compareCandidate(_ left: Candidate, _ right: Candidate) -> Bool {
    compare(left.elapsedMs, left.sequence, right.elapsedMs, right.sequence)
  }

  private static func compare(
    _ leftElapsedMs: Int64,
    _ leftSequence: Int64,
    _ rightElapsedMs: Int64,
    _ rightSequence: Int64
  ) -> Bool {
    leftElapsedMs != rightElapsedMs
      ? leftElapsedMs < rightElapsedMs
      : leftSequence < rightSequence
  }
}
