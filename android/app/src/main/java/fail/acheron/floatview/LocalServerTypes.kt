@file:SuppressLint("UnsafeOptInUsageError")

package fail.acheron.floatview

import android.annotation.SuppressLint
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.*

/*
 * WebSocket
 */

@Serializable data class WebSocketCommand(val cmd: String, val data: JsonObject)

fun wsMessage(topic: String, event: String, data: JsonObject) = buildJsonObject {
    put("topic", topic)
    put("event", event)
    put("data", data)
}
