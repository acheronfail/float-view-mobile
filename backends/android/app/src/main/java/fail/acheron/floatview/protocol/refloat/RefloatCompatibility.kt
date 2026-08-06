package fail.acheron.floatview.protocol.refloat

enum class RefloatTelemetryFamily { LEGACY_FIXED, DYNAMIC }

enum class RefloatWireProfile(val family: RefloatTelemetryFamily) {
    LEGACY_FIXED(RefloatTelemetryFamily.LEGACY_FIXED),
    DYNAMIC(RefloatTelemetryFamily.DYNAMIC),
    DYNAMIC_ALERTS(RefloatTelemetryFamily.DYNAMIC),
}

enum class RefloatLightingProfile { LEGACY_COMMAND_202, STABLE_COMMAND_20 }

data class RefloatControlProfile(
    val lighting: RefloatLightingProfile,
    val fatalClearAvailable: Boolean,
)

data class RefloatCompatibilityEntry(
    val sourceVersion: String,
    val wireVersion: String,
    val profile: RefloatWireProfile,
) {
    val family: RefloatTelemetryFamily
        get() = profile.family
    val controlProfile: RefloatControlProfile
        get() = when (profile) {
            RefloatWireProfile.LEGACY_FIXED, RefloatWireProfile.DYNAMIC ->
                RefloatControlProfile(RefloatLightingProfile.LEGACY_COMMAND_202, false)
            RefloatWireProfile.DYNAMIC_ALERTS ->
                RefloatControlProfile(RefloatLightingProfile.STABLE_COMMAND_20, true)
        }
}

object RefloatCompatibility {
    val entries = listOf(
        RefloatCompatibilityEntry("1.0.0", "1.0", RefloatWireProfile.LEGACY_FIXED),
        RefloatCompatibilityEntry("1.0.1", "1.0", RefloatWireProfile.LEGACY_FIXED),
        RefloatCompatibilityEntry("1.1.0", "1.1.0", RefloatWireProfile.DYNAMIC),
        RefloatCompatibilityEntry("1.1.1", "1.1.1", RefloatWireProfile.DYNAMIC),
        RefloatCompatibilityEntry("1.1.2", "1.1.2", RefloatWireProfile.DYNAMIC),
        RefloatCompatibilityEntry("1.2.0", "1.2.0", RefloatWireProfile.DYNAMIC_ALERTS),
        RefloatCompatibilityEntry("1.2.1", "1.2.1", RefloatWireProfile.DYNAMIC_ALERTS),
        RefloatCompatibilityEntry("1.2.2", "1.2.2", RefloatWireProfile.DYNAMIC_ALERTS),
        RefloatCompatibilityEntry("1.2.3", "1.2.3", RefloatWireProfile.DYNAMIC_ALERTS),
    )

    fun entry(wireVersion: String): RefloatCompatibilityEntry? =
        entries.firstOrNull { it.wireVersion == wireVersion }

    fun controlProfile(wireVersion: String): RefloatControlProfile? = entry(wireVersion)?.controlProfile

    val requiredFields = listOf(
        "motor.speed", "motor.erpm", "motor.current", "motor.dir_current",
        "motor.filt_current", "motor.duty_cycle", "motor.batt_voltage",
        "motor.batt_current", "motor.mosfet_temp", "motor.motor_temp", "imu.pitch",
        "imu.balance_pitch", "imu.roll", "footpad.adc1", "footpad.adc2", "remote.input",
    )

    val requiredRuntimeFields = listOf(
        "setpoint", "atr.setpoint", "brake_tilt.setpoint", "torque_tilt.setpoint",
        "turn_tilt.setpoint", "remote.setpoint", "balance_current", "atr.accel_diff",
        "atr.speed_boost", "booster.current",
    )

    fun supportsDynamicLayout(fields: List<String>, runtimeFields: List<String>): Boolean =
        fields.containsAll(requiredFields) && runtimeFields.containsAll(requiredRuntimeFields)
}
