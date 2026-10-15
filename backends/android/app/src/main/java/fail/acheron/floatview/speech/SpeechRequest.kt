package fail.acheron.floatview.speech

import org.json.JSONObject

internal data class SpeechRequest(
    val id: Long, val expires: Long, val load: Pair<String, Int>?, val speed: Int?,
    val imperial: Boolean, val volume: Int,
) {
    val phrase: String get() = buildList {
        load?.let { add("${it.first} ${it.second} percent") }
        speed?.let { add("speed $it ${if (imperial) "miles per hour" else "kilometres per hour"}") }
    }.joinToString(", ")
    companion object {
        fun decode(value: JSONObject): SpeechRequest? = runCatching {
            require(value.keySet() == setOf("id", "expires_at_ms", "load", "speed", "imperial", "volume", "lower_other_audio"))
            fun integer(objectValue: JSONObject, key: String, max: Long): Long {
                val raw = objectValue.get(key)
                require(raw is Number)
                val number = raw.toDouble()
                require(number.isFinite() && number >= 0 && number <= max && number % 1.0 == 0.0)
                return number.toLong()
            }
            val id = integer(value, "id", 9_007_199_254_740_991)
            require(id > 0)
            require(value.get("imperial") is Boolean && value.get("lower_other_audio") is Boolean)
            val load = if (value.isNull("load")) null else value.getJSONObject("load").let {
                require(it.keySet() == setOf("source", "percent"))
                val source = it.get("source")
                require(source is String && source in listOf("duty", "motor", "battery"))
                source to integer(it, "percent", 100).toInt()
            }
            val speed = if (value.isNull("speed")) null else integer(value, "speed", 200).toInt()
            require(load != null || speed != null)
            SpeechRequest(id, integer(value, "expires_at_ms", Long.MAX_VALUE), load, speed,
                value.getBoolean("imperial"), integer(value, "volume", 100).toInt())
        }.getOrNull()
    }
}
