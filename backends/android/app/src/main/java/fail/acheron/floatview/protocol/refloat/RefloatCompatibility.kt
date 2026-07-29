package fail.acheron.floatview.protocol.refloat

enum class RefloatTelemetryFamily { LEGACY_FIXED, DYNAMIC }

data class RefloatCompatibilityEntry(
    val sourceVersion: String,
    val wireVersion: String,
    val family: RefloatTelemetryFamily,
)

object RefloatCompatibility {
    val entries = listOf(
        RefloatCompatibilityEntry("1.0.0", "1.0", RefloatTelemetryFamily.LEGACY_FIXED),
        RefloatCompatibilityEntry("1.1.0", "1.1.0", RefloatTelemetryFamily.DYNAMIC),
        RefloatCompatibilityEntry("1.2.3", "1.2.3", RefloatTelemetryFamily.DYNAMIC),
    )

    fun entry(wireVersion: String): RefloatCompatibilityEntry? =
        entries.firstOrNull { it.wireVersion == wireVersion }
}
