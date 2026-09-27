package fail.acheron.floatview.transport

import fail.acheron.floatview.application.*
import org.json.JSONArray
import org.json.JSONObject
import java.lang.reflect.Modifier
import java.time.Instant

/** Internal domain encoding only. Rust owns all frontend projection. */
internal object ServerFacts {
    fun objectOf(vararg fields: Pair<String, Any?>) = JSONObject().also { result ->
        fields.forEach { (key, value) -> result.put(key, encode(value)) }
    }
    fun encode(value: Any?): Any = when (value) {
        null -> JSONObject.NULL
        is JSONObject, is JSONArray, is String, is Number, is Boolean -> value
        is Instant -> value.toString()
        is DeviceId -> value.value
        is FirmwareRecord -> objectOf("major" to value.major, "minor" to value.minor, "hardwareName" to value.hardwareName)
        is ControllerRoute.Direct, is BmsConnection.Direct -> objectOf("kind" to "direct")
        is ControllerRoute.Can -> objectOf("kind" to "can", "canId" to value.canId)
        is BmsConnection.Can -> objectOf("kind" to "can", "canId" to value.canId)
        is RefloatConfigurationValue.Decimal -> objectOf("type" to "double", "value" to value.value)
        is RefloatConfigurationValue.Integer -> objectOf("type" to "integer", "value" to value.value)
        is RefloatConfigurationValue.BooleanValue -> objectOf("type" to "boolean", "value" to value.value)
        is RefloatConfigurationValue.StringValue -> objectOf("type" to "string", "value" to value.value)
        is Enum<*> -> value.javaClass.methods.firstOrNull { it.name == "getWireValue" && it.parameterCount == 0 }
            ?.invoke(value) ?: if (value is LocationProviderAuthorization || value is LocationProviderBackgroundCapture || value is RefloatConfigurationOperation) {
                value.name.lowercase().split('_').let { words -> words.first() + words.drop(1).joinToString("") { it.replaceFirstChar(Char::uppercaseChar) } }
            } else value.name.lowercase()
        is Map<*, *> -> JSONObject().also { result -> value.forEach { (key, item) -> result.put(key as String, encode(item)) } }
        is Iterable<*> -> JSONArray(value.map(::encode))
        else -> {
            require(value.javaClass.name.substringBeforeLast('.') == "fail.acheron.floatview.application")
            JSONObject().also { result ->
                value.javaClass.declaredFields.filterNot { Modifier.isStatic(it.modifiers) || it.isSynthetic }.forEach { field ->
                    field.isAccessible = true
                    val name = when {
                        value is DiscoveredDevice && field.name == "id" -> "deviceId"
                        (value is DiscoveredDevice || value is ActiveConnectionSnapshot) && field.name == "advertisedName" -> "name"
                        value is RefloatConfigurationField && field.name == "type" -> "fieldType"
                        value is TelemetrySample && field.name == "extension" -> "extensionFields"
                        else -> field.name
                    }
                    result.put(name, encode(field.get(value)))
                }
            }
        }
    }
    fun values(raw: JSONObject): Map<String, RefloatConfigurationValue> = raw.keys().asSequence().associateWith { name ->
        val item = raw.getJSONObject(name)
        when (item.getString("type")) {
            "double" -> RefloatConfigurationValue.Decimal(item.getDouble("value"))
            "integer" -> RefloatConfigurationValue.Integer(item.getLong("value"))
            "boolean" -> RefloatConfigurationValue.BooleanValue(item.getBoolean("value"))
            "string" -> RefloatConfigurationValue.StringValue(item.getString("value"))
            else -> error("Invalid configuration value")
        }
    }
    fun identity(raw: JSONObject) = RefloatConfigurationIdentityRecord(raw.getString("packageName"), raw.getString("packageVersion"), raw.getString("schemaFingerprint"), raw.getLong("wireSignature"))
    fun nullable(raw: JSONObject, name: String): String? = if (raw.isNull(name)) null else raw.getString(name)
}
