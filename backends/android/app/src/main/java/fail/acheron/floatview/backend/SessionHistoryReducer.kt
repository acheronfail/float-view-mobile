package fail.acheron.floatview.backend

import org.json.JSONObject
import java.util.Base64
import kotlin.math.abs
import kotlin.math.max
import kotlin.math.min

data class SessionElapsedRange(val startElapsedMs: Long, val endElapsedMs: Long)

data class SessionGapPolicyRecord(val thresholdSeconds: Double?)

data class SessionSeriesColumnRecord(
    val elapsedMs: List<Long>,
    val sequence: List<Long>,
    val values: List<Double?>,
    val generations: List<Int>,
)

data class SessionHistoryCursor(
    val sessionId: String,
    val snapshot: SessionPublishedSnapshotRecord,
    val range: SessionElapsedRange,
    val elapsedMs: Long,
    val sequence: Long,
    val version: Int = 1,
)

object SessionHistoryReducer {
    const val MAX_SERIES_OBSERVATIONS = 65_536
    private const val MAX_CURSOR_BYTES = 4_096

    private data class Candidate(
        val point: SessionSeriesPoint,
        val value: Double,
        val generation: Int,
    )

    fun reduceSeries(
        rows: List<SessionSeriesPoint>,
        fields: List<String>,
        range: SessionElapsedRange,
        buckets: Int,
        gapPolicy: SessionGapPolicyRecord,
        expectedPersistenceMs: Long = 1_000,
        expectedRidingPersistenceMs: Long? = null,
        expectedIdlePersistenceMs: Long? = null,
        retainedResolutionSeconds: Long = 0,
    ): Map<String, SessionSeriesColumnRecord> {
        if (range.startElapsedMs > range.endElapsedMs || buckets !in 1..1_024 ||
            fields.isEmpty() || fields.size > 16 || fields.toSet().size != fields.size
        ) throw SessionStoreException.InvalidRange
        val ordered = rows.filter {
            it.elapsedMs in range.startElapsedMs..range.endElapsedMs
        }.sortedWith(pointComparator)
        val span = max(1L, range.endElapsedMs - range.startElapsedMs + 1L)
        val thresholdMs = gapPolicy.thresholdSeconds?.let { seconds ->
            if (!seconds.isFinite() || seconds < 0.1 || seconds > 3_600) {
                throw SessionStoreException.InvalidRange
            }
            max(1L, (seconds * 1_000.0).toLong())
        }
        val generations = fields.associateWith { 0 }.toMutableMap()
        val byField = fields.associateWith { mutableMapOf<Int, MutableMap<Long, Candidate>>() }
        var previousElapsed: Long? = null
        var previousRideState: String? = null
        for (row in ordered) {
            val phasePersistence = if (previousRideState == "riding") {
                expectedRidingPersistenceMs ?: expectedPersistenceMs
            } else {
                expectedIdlePersistenceMs ?: expectedPersistenceMs
            }
            val edgeThreshold = thresholdMs ?: maxOf(
                1_000L,
                max(1L, phasePersistence) * 3L,
                max(0L, retainedResolutionSeconds) * 3_000L,
            )
            if (previousElapsed != null && row.elapsedMs - previousElapsed!! > edgeThreshold) {
                fields.forEach { field -> generations[field] = generations.getValue(field) + 1 }
            }
            previousElapsed = row.elapsedMs
            previousRideState = row.rideState
            val offset = max(0L, row.elapsedMs - range.startElapsedMs)
            val bucket = min(buckets - 1, (offset * buckets / span).toInt())
            fields.forEach { field ->
                val value = row.values[field]
                if (value == null) {
                    generations[field] = generations.getValue(field) + 1
                } else {
                    byField.getValue(field).getOrPut(bucket) { mutableMapOf() }[row.sequence] = Candidate(
                        row,
                        value,
                        generations.getValue(field),
                    )
                }
            }
        }
        val result = linkedMapOf<String, SessionSeriesColumnRecord>()
        var observations = 0
        fields.forEach { field ->
            val selected = linkedMapOf<Long, Candidate>()
            byField.getValue(field).values.forEach { candidatesBySequence ->
                val candidates = candidatesBySequence.values.toList()
                if (candidates.isEmpty()) return@forEach
                val first = candidates.minWith(candidateComparator)
                val last = candidates.maxWith(candidateComparator)
                val minimum = candidates.minWith(compareBy<Candidate> { it.value }.thenComparator { left, right -> compareCandidates(left, right) })
                val maximum = candidates.maxWith(compareBy<Candidate> { it.value }.thenComparator { left, right -> compareCandidates(left, right) })
                listOf(first, minimum, maximum, last).forEach { selected[it.point.sequence] = it }
            }
            val orderedSelected = selected.values.sortedWith { left, right -> comparePoints(left.point, right.point) }
            observations += orderedSelected.size
            if (observations > MAX_SERIES_OBSERVATIONS) throw SessionStoreException.InvalidRange
            result[field] = SessionSeriesColumnRecord(
                elapsedMs = orderedSelected.map { it.point.elapsedMs },
                sequence = orderedSelected.map { it.point.sequence },
                values = orderedSelected.map { it.value },
                generations = orderedSelected.map { it.generation },
            )
        }
        return result
    }

    fun nearestSample(rows: List<SessionSeriesPoint>, targetElapsedMs: Long): SessionSeriesPoint? =
        rows.minWithOrNull(Comparator { left, right ->
            val leftDistance = abs(left.elapsedMs - targetElapsedMs)
            val rightDistance = abs(right.elapsedMs - targetElapsedMs)
            if (leftDistance != rightDistance) leftDistance.compareTo(rightDistance)
            else comparePoints(left, right)
        })

    fun selectPruneRows(
        rows: List<SessionSeriesPoint>,
        fields: List<String>,
        targetResolutionSeconds: Long,
        protectedSequences: Set<Long> = emptySet(),
        eventSourceSequences: Set<Long> = emptySet(),
    ): List<Long> {
        if (targetResolutionSeconds !in 1..300 || fields.isEmpty() || fields.size > 16) {
            throw SessionStoreException.InvalidRange
        }
        val ordered = rows.sortedWith(pointComparator)
        if (ordered.isEmpty()) return emptyList()
        val keep = (protectedSequences + eventSourceSequences).toMutableSet()
        keep += ordered.first().sequence
        keep += ordered.last().sequence
        fields.forEach { field -> ordered.firstOrNull { it.values[field] != null }?.let { keep += it.sequence } }
        val width = targetResolutionSeconds * 1_000L
        ordered.groupBy { it.elapsedMs / width }.values.forEach { bucketRows ->
            val first = bucketRows.first()
            val last = bucketRows.last()
            keep += first.sequence
            keep += last.sequence
            val roles = mutableListOf<Pair<String, SessionSeriesPoint>>("first" to first, "last" to last)
            fields.forEach { field ->
                val values = bucketRows.mapNotNull { row -> row.values[field]?.let { row to it } }
                values.minWithOrNull(compareBy<Pair<SessionSeriesPoint, Double>> { it.second }.thenBy { it.first.elapsedMs }.thenBy { it.first.sequence })
                    ?.first?.let { roles += "$field:min" to it }
                values.maxWithOrNull(compareBy<Pair<SessionSeriesPoint, Double>> { it.second }.thenBy { it.first.elapsedMs }.thenBy { it.first.sequence })
                    ?.first?.let { roles += "$field:max" to it }
            }
            val uncovered = roles.mapTo(mutableSetOf()) { it.first }
            val candidates = roles.map { it.second }.distinctBy { it.sequence }
                .filter { it.sequence !in keep }.toMutableList()
            while (uncovered.isNotEmpty() && candidates.isNotEmpty()) {
                candidates.sortWith(compareByDescending<SessionSeriesPoint> { candidate ->
                    roles.count { it.second.sequence == candidate.sequence && it.first in uncovered }
                }.thenComparator { left, right -> comparePoints(left, right) })
                val choice = candidates.removeAt(0)
                keep += choice.sequence
                roles.filter { it.second.sequence == choice.sequence }.forEach { uncovered.remove(it.first) }
            }
        }
        return ordered.filter { it.sequence in keep }.map { it.sequence }
    }

    fun page(
        rows: List<SessionSeriesPoint>,
        limit: Int,
        cursor: SessionHistoryCursor? = null,
        cursorContext: SessionHistoryCursor? = null,
    ): Pair<List<SessionSeriesPoint>, SessionHistoryCursor?> {
        if (limit !in 1..2_000) throw SessionStoreException.InvalidRange
        val sorted = rows.sortedWith(pointComparator)
        val start = cursor?.let { value ->
            sorted.indexOfFirst { compareKey(value.elapsedMs, value.sequence, it.elapsedMs, it.sequence) < 0 }
                .let { if (it < 0) sorted.size else it }
        } ?: 0
        val page = sorted.drop(start).take(limit)
        val context = cursor ?: cursorContext
        val next = if (page.isNotEmpty() && start + page.size < sorted.size && context != null) {
            val last = page.last()
            context.copy(elapsedMs = last.elapsedMs, sequence = last.sequence)
        } else null
        return page to next
    }

    fun encodeCursor(cursor: SessionHistoryCursor): String {
        val snapshot = JSONObject()
            .put("historyRevision", cursor.snapshot.historyRevision)
            .put("committedSampleSequence", cursor.snapshot.committedSampleSequence)
            .put("committedEventSequence", cursor.snapshot.committedEventSequence)
            .put("committedElapsedMs", cursor.snapshot.committedElapsedMs)
        val range = JSONObject()
            .put("startElapsedMs", cursor.range.startElapsedMs)
            .put("endElapsedMs", cursor.range.endElapsedMs)
        val json = JSONObject()
            .put("version", cursor.version)
            .put("sessionId", cursor.sessionId)
            .put("snapshot", snapshot)
            .put("range", range)
            .put("elapsedMs", cursor.elapsedMs)
            .put("sequence", cursor.sequence)
            .toString()
        val encoded = Base64.getUrlEncoder().withoutPadding().encodeToString(json.toByteArray(Charsets.UTF_8))
        if (encoded.toByteArray(Charsets.UTF_8).size > MAX_CURSOR_BYTES) throw SessionStoreException.InvalidCursor
        return encoded
    }

    fun decodeCursor(value: String): SessionHistoryCursor {
        if (value.isEmpty() || value.toByteArray(Charsets.UTF_8).size > MAX_CURSOR_BYTES) {
            throw SessionStoreException.InvalidCursor
        }
        return try {
            val json = JSONObject(String(Base64.getUrlDecoder().decode(value), Charsets.UTF_8))
            val snapshot = json.getJSONObject("snapshot")
            val range = json.getJSONObject("range")
            val cursor = SessionHistoryCursor(
                sessionId = json.getString("sessionId"),
                snapshot = SessionPublishedSnapshotRecord(
                    snapshot.getLong("historyRevision"),
                    snapshot.getLong("committedSampleSequence"),
                    snapshot.getLong("committedEventSequence"),
                    snapshot.getLong("committedElapsedMs"),
                    0,
                    0,
                ),
                range = SessionElapsedRange(range.getLong("startElapsedMs"), range.getLong("endElapsedMs")),
                elapsedMs = json.getLong("elapsedMs"),
                sequence = json.getLong("sequence"),
                version = json.getInt("version"),
            )
            if (cursor.version != 1 || cursor.range.startElapsedMs > cursor.range.endElapsedMs ||
                cursor.elapsedMs !in cursor.range.startElapsedMs..cursor.range.endElapsedMs || cursor.sequence < 0
            ) throw SessionStoreException.InvalidCursor
            cursor
        } catch (_: SessionStoreException) {
            throw SessionStoreException.InvalidCursor
        } catch (_: Exception) {
            throw SessionStoreException.InvalidCursor
        }
    }

    private fun comparePoints(left: SessionSeriesPoint, right: SessionSeriesPoint): Int =
        compareKey(left.elapsedMs, left.sequence, right.elapsedMs, right.sequence)

    private fun compareCandidates(left: Candidate, right: Candidate): Int = comparePoints(left.point, right.point)

    private fun compareKey(leftElapsed: Long, leftSequence: Long, rightElapsed: Long, rightSequence: Long): Int =
        leftElapsed.compareTo(rightElapsed).takeIf { it != 0 } ?: leftSequence.compareTo(rightSequence)

    private val pointComparator = Comparator<SessionSeriesPoint> { left, right -> comparePoints(left, right) }
    private val candidateComparator = Comparator<Candidate> { left, right -> compareCandidates(left, right) }
}
