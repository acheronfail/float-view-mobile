package fail.acheron.floatview.application

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class RustApplicationRuntimeTest {
    @Test
    fun coarseRuntimePublishesOneOrderedResult() {
        RustApplicationRuntime().use { runtime ->
            val result = runtime.apply(JSONObject()
                .put("command", "begin")
                .put("now_ms", 10)
                .put("idle_interval_ms", 1_000)
                .put("post_ride_tail_ms", 5_000)
                .put("preferred_controller_can_id", JSONObject.NULL)
                .put("preferred_bms_can_id", JSONObject.NULL)
                .put("routing_hint_can_id", JSONObject.NULL)
                .put("routing_hint_controller_uuid", JSONObject.NULL)
                .put("has_active_session", false)
                .put("continuity_can_id", JSONObject.NULL)
                .put("continuity_controller_uuid", JSONObject.NULL))

            assertEquals(1, result.generation)
            assertEquals("publish_state", result.effects.first().getString("effect"))
            assertTrue(result.effects.any { it.getString("effect") == "write_bytes" })
            assertTrue(result.effects.any { it.getString("effect") == "arm_deadline" })
            assertEquals("publish_state", result.effects.last().getString("effect"))
        }
    }

    @Test
    fun rejectsOpenEnvelope() {
        RustApplicationRuntime().use { runtime ->
            val error = runCatching {
                runtime.apply(JSONObject().put("command", "reset").put("now_ms", 1).put("extra", true))
            }.exceptionOrNull() as RustApplicationRuntimeException
            assertEquals(RustApplicationRuntimeError.INVALID_COMMAND, error.reason)
        }
    }
}
