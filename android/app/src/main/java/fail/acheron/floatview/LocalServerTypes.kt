@file:SuppressLint("UnsafeOptInUsageError")

package fail.acheron.floatview

import android.annotation.SuppressLint
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.*

// Helper functions for creating JSON responses concisely
fun successResponse(data: JsonElement) = buildJsonObject {
    put("kind", "success")
    put("data", data)
}

fun errorResponse(message: String) = buildJsonObject {
    put("kind", "error")
    put("data", buildJsonObject {
        put("message", message)
    })
}

/*
 * Requests
 */

@Serializable data class BleConnectRequest(val deviceId: String)
