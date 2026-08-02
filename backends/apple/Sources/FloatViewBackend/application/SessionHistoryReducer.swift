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
    let point: SessionSeriesPoint
    let value: Double
    let generation: Int
  }

  public static let maximumSeriesObservations = 65_536

  public static func reduceSeries(
    rows: [SessionSeriesPoint],
    fields: [String],
    range: SessionElapsedRange,
    buckets: Int,
    gapPolicy: SessionGapPolicyRecord,
    expectedPersistenceMs: Int64 = 1_000,
    expectedRidingPersistenceMs: Int64? = nil,
    expectedIdlePersistenceMs: Int64? = nil,
    retainedResolutionSeconds: Int64 = 0
  ) throws -> [String: SessionSeriesColumnRecord] {
    guard range.startElapsedMs <= range.endElapsedMs,
          (1 ... 1_024).contains(buckets),
          (1 ... 16).contains(fields.count),
          Set(fields).count == fields.count
    else { throw SessionStoreError.invalidRange }

    let ordered = rows
      .filter { $0.elapsedMs >= range.startElapsedMs && $0.elapsedMs <= range.endElapsedMs }
      .sorted(by: compare)
    let span = max(1, range.endElapsedMs - range.startElapsedMs + 1)
    let thresholdMs: Int64
    if let thresholdSeconds = gapPolicy.thresholdSeconds {
      guard thresholdSeconds.isFinite, thresholdSeconds >= 0.1, thresholdSeconds <= 3_600 else {
        throw SessionStoreError.invalidRange
      }
      thresholdMs = max(1, Int64((thresholdSeconds * 1_000).rounded()))
    } else {
      thresholdMs = max(1_000, max(1, expectedPersistenceMs) * 3,
                        max(0, retainedResolutionSeconds) * 3 * 1_000)
      if let expectedRidingPersistenceMs {
        guard expectedRidingPersistenceMs > 0 else { throw SessionStoreError.invalidRange }
      }
      if let expectedIdlePersistenceMs {
        guard expectedIdlePersistenceMs > 0 else { throw SessionStoreError.invalidRange }
      }
    }

    var generations = Dictionary(uniqueKeysWithValues: fields.map { ($0, 0) })
    var bucketsByField: [String: [Int: [Int64: Candidate]]] = Dictionary(
      uniqueKeysWithValues: fields.map { ($0, [:]) }
    )
    var previousElapsed: Int64?
    var previousRideState: String?

    for row in ordered {
      let edgeThreshold = gapPolicy.thresholdSeconds == nil
        ? max(
          1_000,
          max(
            1,
            previousRideState == "riding"
              ? (expectedRidingPersistenceMs ?? expectedPersistenceMs)
              : (expectedIdlePersistenceMs ?? expectedPersistenceMs)
          ) * 3,
          max(0, retainedResolutionSeconds) * 3 * 1_000
        )
        : thresholdMs
      if let previousElapsed, row.elapsedMs - previousElapsed > edgeThreshold {
        for field in fields { generations[field, default: 0] += 1 }
      }
      previousElapsed = row.elapsedMs
      previousRideState = row.rideState
      let offset = max(0, row.elapsedMs - range.startElapsedMs)
      let bucket = min(buckets - 1, Int((offset * Int64(buckets)) / span))
      for field in fields {
        guard let wrapped = row.values[field], let value = wrapped else {
          generations[field, default: 0] += 1
          continue
        }
        let candidate = Candidate(
          point: row,
          value: value,
          generation: generations[field, default: 0]
        )
        var fieldBuckets = bucketsByField[field, default: [:]]
        var bucketRows = fieldBuckets[bucket, default: [:]]
        bucketRows[row.sequence] = candidate
        fieldBuckets[bucket] = bucketRows
        bucketsByField[field] = fieldBuckets
      }
    }

    var result: [String: SessionSeriesColumnRecord] = [:]
    var observationCount = 0
    for field in fields {
      var selected: [Int64: Candidate] = [:]
      for bucketRows in bucketsByField[field, default: [:]].values {
        let candidates = Array(bucketRows.values)
        guard let first = candidates.min(by: compareCandidate),
              let last = candidates.max(by: compareCandidate)
        else { continue }
        let minimum = candidates.reduce(first) { best, candidate in
          candidate.value < best.value ||
            (candidate.value == best.value && compareCandidate(candidate, best)) ? candidate : best
        }
        let maximum = candidates.reduce(first) { best, candidate in
          candidate.value > best.value ||
            (candidate.value == best.value && compareCandidate(candidate, best)) ? candidate : best
        }
        for candidate in [first, minimum, maximum, last] {
          selected[candidate.point.sequence] = candidate
        }
      }
      let orderedSelected = selected.values.sorted { compare($0.point, $1.point) }
      observationCount += orderedSelected.count
      if observationCount > maximumSeriesObservations {
        throw SessionStoreError.invalidRange
      }
      result[field] = SessionSeriesColumnRecord(
        elapsedMs: orderedSelected.map { $0.point.elapsedMs },
        sequence: orderedSelected.map { $0.point.sequence },
        values: orderedSelected.map { $0.value },
        generations: orderedSelected.map { $0.generation }
      )
    }
    return result
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
    for row in ordered { buckets[row.elapsedMs / width, default: []].append(row) }
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
      while !uncovered.isEmpty && candidates.count > 0 {
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
    compare(left.point, right.point)
  }
}
