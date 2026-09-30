package fail.acheron.floatview.permissions

/** Rationale=false alone never distinguishes first request from blocked retry. */
data class PermissionGrant(val access: String, val requestable: Boolean, val rationale: Boolean) {
    companion object {
        fun observe(required: Boolean, granted: Boolean, resolved: Boolean, rationale: Boolean, restricted: Boolean) =
            PermissionGrant(
                when { !required -> "notRequired"; restricted -> "restricted"; granted -> "granted"; resolved -> "denied"; else -> "notDetermined" },
                required && !restricted && (!resolved || rationale),
                required && !restricted && rationale,
            )
    }
}
