package fail.acheron.floatview.backend

import org.junit.Assert.assertEquals
import org.junit.Test

class SessionHistoryReducerTest {
    @Test
    fun pruneSelectionProtectsBoundariesEventsAndFieldStarts() {
        val rows = (0 until 24).map { index ->
            SessionSeriesPoint(
                elapsedMs = index * 500L,
                sequence = index.toLong(),
                rideState = if (index < 12) "riding" else "idle",
                values = mapOf(
                    "speedKph" to if (index == 6) null else (index * 3 % 19).toDouble(),
                    "packVoltageV" to 60.0 - index * .2,
                    "extensionLate" to if (index >= 12) index / 10.0 else null,
                ),
            )
        }
        val selected = SessionHistoryReducer.selectPruneRows(
            rows,
            listOf("speedKph", "packVoltageV", "extensionLate"),
            5,
            protectedSequences = setOf(5, 12, 13),
            eventSourceSequences = setOf(5, 12),
        )
        assertEquals(listOf(0L, 5L, 9L, 10L, 12L, 13L, 19L, 20L, 23L), selected)
    }
}
