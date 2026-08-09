package fail.acheron.floatview.application

import org.json.JSONObject
import java.util.Base64
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
        val elapsedMs: Long,
        val sequence: Long,
        val value: Double,
        val generation: Int,
    )

    private data class BucketCandidates(
        var first: Candidate? = null,
        var minimum: Candidate? = null,
        var maximum: Candidate? = null,
        var last: Candidate? = null,
    )

    /**
     * Bounded ordered reducer used while SQLite is walking a historical
     * cursor. Only four candidate roles per field/bucket are retained.
     */
    class SeriesAccumulator(
        private val fields: List<String>,
        private val range: SessionElapsedRange,
        private val buckets: Int,
        private val gapPolicy: SessionGapPolicyRecord,
        private val expectedPersistenceMs: Long = 1_000,
        private val expectedRidingPersistenceMs: Long? = null,
        private val expectedIdlePersistenceMs: Long? = null,
        private val retainedResolutionSeconds: Long = 0,
    ) {
        private val configuredThresholdMs: Long? = gapPolicy.thresholdSeconds?.let { seconds ->
            if (!seconds.isFinite() || seconds < 0.1 || seconds > 3_600) {
                throw SessionStoreException.InvalidRange
            }
            max(1L, (seconds * 1_000.0).toLong())
        }
        private val generations = IntArray(fields.size)
        private val byField = List(fields.size) { arrayOfNulls<BucketCandidates>(buckets.coerceAtLeast(0)) }
        private var previousElapsed: Long? = null
        private var previousRideState: String? = null

        init {
            if (range.startElapsedMs > range.endElapsedMs || buckets !in 1..1_024 ||
                fields.isEmpty() || fields.size > 16 || fields.toSet().size != fields.size
            ) throw SessionStoreException.InvalidRange
            if (expectedRidingPersistenceMs != null && expectedRidingPersistenceMs <= 0) {
                throw SessionStoreException.InvalidRange
            }
            if (expectedIdlePersistenceMs != null && expectedIdlePersistenceMs <= 0) {
                throw SessionStoreException.InvalidRange
            }
        }

        fun append(row: SessionSeriesPoint) {
            append(
                elapsedMs = row.elapsedMs,
                sequence = row.sequence,
                rideState = row.rideState,
                values = fields.map { row.values[it] },
            )
        }

        fun append(
            elapsedMs: Long,
            sequence: Long,
            rideState: String?,
            values: List<Double?>,
        ) {
            val phasePersistence = if (previousRideState == "riding") {
                expectedRidingPersistenceMs ?: expectedPersistenceMs
            } else {
                expectedIdlePersistenceMs ?: expectedPersistenceMs
            }
            val edgeThreshold = configuredThresholdMs ?: maxOf(
                1_000L,
                max(1L, phasePersistence) * 3L,
                max(0L, retainedResolutionSeconds) * 3_000L,
            )
            if (previousElapsed != null && elapsedMs - previousElapsed!! > edgeThreshold) {
                generations.indices.forEach { index -> generations[index] += 1 }
            }
            previousElapsed = elapsedMs
            previousRideState = rideState
            val span = max(1L, range.endElapsedMs - range.startElapsedMs + 1L)
            val offset = max(0L, elapsedMs - range.startElapsedMs)
            val bucket = min(buckets - 1, (offset * buckets / span).toInt())
            fields.indices.forEach { index ->
                val value = values[index]
                if (value == null) {
                    generations[index] += 1
                } else {
                    val candidate = Candidate(elapsedMs, sequence, value, generations[index])
                    val bucketCandidates = byField[index][bucket] ?: BucketCandidates()
                    if (bucketCandidates.first == null || compareCandidates(candidate, bucketCandidates.first!!) < 0) {
                        bucketCandidates.first = candidate
                    }
                    if (
                        bucketCandidates.minimum == null ||
                        candidate.value < bucketCandidates.minimum!!.value ||
                        (candidate.value == bucketCandidates.minimum!!.value &&
                            compareCandidates(candidate, bucketCandidates.minimum!!) < 0)
                    ) bucketCandidates.minimum = candidate
                    if (
                        bucketCandidates.maximum == null ||
                        candidate.value > bucketCandidates.maximum!!.value ||
                        (candidate.value == bucketCandidates.maximum!!.value &&
                            compareCandidates(candidate, bucketCandidates.maximum!!) < 0)
                    ) bucketCandidates.maximum = candidate
                    if (bucketCandidates.last == null || compareCandidates(candidate, bucketCandidates.last!!) > 0) {
                        bucketCandidates.last = candidate
                    }
                    byField[index][bucket] = bucketCandidates
                }
            }
        }

        fun finish(): Map<String, SessionSeriesColumnRecord> {
            val result = linkedMapOf<String, SessionSeriesColumnRecord>()
            var observations = 0
            fields.forEachIndexed { index, field ->
                val selected = linkedMapOf<Long, Candidate>()
                byField[index].forEach { bucketCandidates ->
                    listOf(
                        bucketCandidates?.first,
                        bucketCandidates?.minimum,
                        bucketCandidates?.maximum,
                        bucketCandidates?.last,
                        ).filterNotNull().forEach { selected[it.sequence] = it }
                }
                val orderedSelected = selected.values.sortedWith { left, right ->
                    compareKey(left.elapsedMs, left.sequence, right.elapsedMs, right.sequence)
                }
                observations += orderedSelected.size
                if (observations > MAX_SERIES_OBSERVATIONS) throw SessionStoreException.InvalidRange
                result[field] = SessionSeriesColumnRecord(
                    elapsedMs = orderedSelected.map { it.elapsedMs },
                    sequence = orderedSelected.map { it.sequence },
                    values = orderedSelected.map { it.value },
                    generations = orderedSelected.map { it.generation },
                )
            }
            return result
        }
    }

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
        rowsAreOrdered: Boolean = false,
    ): Map<String, SessionSeriesColumnRecord> {
        val accumulator = SeriesAccumulator(
            fields, range, buckets, gapPolicy, expectedPersistenceMs,
            expectedRidingPersistenceMs, expectedIdlePersistenceMs, retainedResolutionSeconds,
        )
        val ordered = if (rowsAreOrdered) rows else rows.filter {
            it.elapsedMs in range.startElapsedMs..range.endElapsedMs
        }.sortedWith(pointComparator)
        ordered.forEach(accumulator::append)
        return accumulator.finish()
    }

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

    private fun compareCandidates(left: Candidate, right: Candidate): Int =
        compareKey(left.elapsedMs, left.sequence, right.elapsedMs, right.sequence)

    private fun compareKey(leftElapsed: Long, leftSequence: Long, rightElapsed: Long, rightSequence: Long): Int =
        leftElapsed.compareTo(rightElapsed).takeIf { it != 0 } ?: leftSequence.compareTo(rightSequence)

    private val pointComparator = Comparator<SessionSeriesPoint> { left, right -> comparePoints(left, right) }
}
