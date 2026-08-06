package fail.acheron.floatview.backend

import fail.acheron.floatview.protocol.refloat.RefloatBooleanControl
import fail.acheron.floatview.protocol.refloat.RefloatControlCodec
import fail.acheron.floatview.protocol.refloat.RefloatControlProfile
import fail.acheron.floatview.protocol.refloat.RefloatLightingState
import fail.acheron.floatview.protocol.vesc.ControllerRoute

data class RefloatMutationContext(
    val generation: VescRequestGeneration,
    val route: ControllerRoute,
    val profile: RefloatControlProfile,
    val connectionReady: Boolean,
    val writable: Boolean,
    val rideState: RideState,
    val packageState: PackageState?,
    val packageMode: PackageMode?,
)

data class AuthorizedRefloatMutation(
    val action: RefloatPendingAction,
    val payload: ByteArray,
    val oneWay: Boolean,
    val expectedResponse: VescExpectedResponse,
)

sealed interface RefloatMutationAuthorization {
    data class Idempotent(val controls: RefloatControls) : RefloatMutationAuthorization
    data class Mutation(val mutation: AuthorizedRefloatMutation, val controls: RefloatControls) : RefloatMutationAuthorization
}

class RefloatMutationException(val unavailable: Boolean = false) : Exception()

enum class ControllerPendingAction(val wireValue: String) { SHUTDOWN("shutdown") }

data class ControllerActions(
    val shutdownAvailable: Boolean = false,
    val pendingAction: ControllerPendingAction? = null,
)

data class ControllerShutdownContext(
    val generation: VescRequestGeneration,
    val connectionReady: Boolean,
    val shutdownSupported: Boolean,
    val rideState: RideState,
    val packageMode: PackageMode?,
)

class DeviceMutationCoordinator {
    private var generation = VescRequestGeneration(0)
    private var controls: RefloatControls? = null
    private var expected: Boolean? = null
    private var controllerPendingAction: ControllerPendingAction? = null

    @Synchronized fun configure(
        generation: VescRequestGeneration,
        profile: RefloatControlProfile,
        writable: Boolean,
    ): RefloatControls {
        this.generation = generation
        expected = null
        controllerPendingAction = null
        return RefloatControls(
            writable = writable,
            lightingAvailable = true,
            fatalErrorClearAvailable = profile.fatalClearAvailable,
            fatalErrorLatched = if (profile.fatalClearAvailable) null else false,
            fatalErrorActive = if (profile.fatalClearAvailable) null else false,
            fatalErrorClearable = if (profile.fatalClearAvailable) null else false,
        ).also { controls = it }
    }

    @Synchronized fun snapshot(): RefloatControls? = controls

    @Synchronized fun controllerActions(shutdownAvailable: Boolean): ControllerActions =
        ControllerActions(shutdownAvailable && controllerPendingAction == null, controllerPendingAction)

    @Synchronized fun authorizeShutdown(context: ControllerShutdownContext): ControllerActions {
        if (
            context.generation != generation || !context.connectionReady || !context.shutdownSupported ||
            context.rideState != RideState.IDLE || context.packageMode == PackageMode.FLYWHEEL ||
            controls?.pendingAction != null || controllerPendingAction != null
        ) throw RefloatMutationException()
        controllerPendingAction = ControllerPendingAction.SHUTDOWN
        return ControllerActions(pendingAction = ControllerPendingAction.SHUTDOWN)
    }

    @Synchronized fun clearShutdown(generation: VescRequestGeneration): ControllerActions? {
        if (generation != this.generation || controllerPendingAction != ControllerPendingAction.SHUTDOWN) return null
        controllerPendingAction = null
        return ControllerActions()
    }

    @Synchronized fun observeTelemetry(value: RefloatTelemetryRecord): RefloatControls? {
        val current = controls ?: return null
        val next = current.copy(
            packageEnabled = value.packageState?.let { it != PackageState.DISABLED } ?: current.packageEnabled,
            flywheelEnabled = value.packageMode?.let { it == PackageMode.FLYWHEEL } ?: current.flywheelEnabled,
            fatalErrorLatched = value.fatalErrorLatched ?: current.fatalErrorLatched,
            fatalErrorActive = value.fatalErrorLatched?.let { value.firmwareFault != null } ?: current.fatalErrorActive,
            fatalErrorClearable = value.fatalErrorLatched?.let { it && value.firmwareFault == null } ?: current.fatalErrorClearable,
        )
        controls = confirm(next.copy(status = if (ready(next)) RefloatControlStatus.READY else RefloatControlStatus.LOADING))
        return controls
    }

    @Synchronized fun observeLighting(value: RefloatLightingState): RefloatControls? {
        val current = controls ?: return null
        val next = current.copy(ledsEnabled = value.ledsEnabled, headlightsEnabled = value.headlightsEnabled)
        controls = confirm(next.copy(status = if (ready(next)) RefloatControlStatus.READY else RefloatControlStatus.LOADING))
        return controls
    }

    @Synchronized fun authorizeSet(
        control: RefloatBooleanControl,
        enabled: Boolean,
        context: RefloatMutationContext,
    ): RefloatMutationAuthorization {
        var current = validate(context)
        if (current.pendingAction != null || controllerPendingAction != null) throw RefloatMutationException()
        if (current.flywheelEnabled == true && !(control == RefloatBooleanControl.FLYWHEEL && !enabled)) throw RefloatMutationException()
        val activeFlywheelDisable = control == RefloatBooleanControl.FLYWHEEL && !enabled && context.packageMode == PackageMode.FLYWHEEL
        if ((control == RefloatBooleanControl.PACKAGE || control == RefloatBooleanControl.FLYWHEEL && !activeFlywheelDisable) && context.rideState != RideState.IDLE) throw RefloatMutationException()
        if (control == RefloatBooleanControl.FLYWHEEL && enabled && (context.packageState != PackageState.READY || context.packageMode != PackageMode.NORMAL)) throw RefloatMutationException()
        if (control == RefloatBooleanControl.FLYWHEEL && !enabled && context.packageMode != PackageMode.FLYWHEEL) throw RefloatMutationException()
        val confirmed = when (control) {
            RefloatBooleanControl.PACKAGE -> current.packageEnabled
            RefloatBooleanControl.FLYWHEEL -> current.flywheelEnabled
            RefloatBooleanControl.LEDS -> current.ledsEnabled
            RefloatBooleanControl.HEADLIGHTS -> current.headlightsEnabled
        }
        if (confirmed == enabled) return RefloatMutationAuthorization.Idempotent(current)
        val payload: ByteArray
        val oneWay: Boolean
        when (control) {
            RefloatBooleanControl.PACKAGE -> { payload = RefloatControlCodec.packageRequest(enabled); oneWay = true }
            RefloatBooleanControl.FLYWHEEL -> { payload = RefloatControlCodec.flywheelRequest(enabled); oneWay = true }
            RefloatBooleanControl.LEDS, RefloatBooleanControl.HEADLIGHTS -> {
                payload = RefloatControlCodec.lightingRequest(
                    context.profile, control, enabled,
                    RefloatLightingState(current.ledsEnabled ?: throw RefloatMutationException(), current.headlightsEnabled ?: throw RefloatMutationException()),
                ); oneWay = false
            }
        }
        val action = RefloatPendingAction.entries.first { it.wireValue == control.wireValue }
        current = current.copy(pendingAction = action)
        controls = current; expected = enabled
        return RefloatMutationAuthorization.Mutation(
            AuthorizedRefloatMutation(action, payload, oneWay, VescExpectedResponse(36, payload[2].toInt() and 0xff)), current,
        )
    }

    @Synchronized fun authorizeFatalClear(context: RefloatMutationContext): RefloatMutationAuthorization {
        var current = validate(context)
        if (current.pendingAction != null || controllerPendingAction != null || current.flywheelEnabled == true || context.rideState != RideState.IDLE ||
            !current.fatalErrorClearAvailable || current.fatalErrorLatched != true || current.fatalErrorActive != false || current.fatalErrorClearable != true
        ) throw RefloatMutationException()
        val payload = RefloatControlCodec.fatalClearRequest(context.profile)
        current = current.copy(pendingAction = RefloatPendingAction.CLEAR_FATAL_ERROR)
        controls = current; expected = false
        return RefloatMutationAuthorization.Mutation(
            AuthorizedRefloatMutation(RefloatPendingAction.CLEAR_FATAL_ERROR, payload, true, VescExpectedResponse(36, payload[2].toInt() and 0xff)), current,
        )
    }

    @Synchronized fun failPending(generation: VescRequestGeneration): RefloatControls? {
        if (generation != this.generation) return controls
        controls = controls?.copy(pendingAction = null); expected = null
        return controls
    }

    @Synchronized fun cancel(): RefloatControls? {
        controls = controls?.copy(pendingAction = null)
        expected = null
        controllerPendingAction = null
        return controls
    }

    private fun validate(context: RefloatMutationContext): RefloatControls {
        val current = controls
        if (context.generation != generation || !context.connectionReady || !context.writable || current?.status != RefloatControlStatus.READY || !current.writable) throw RefloatMutationException()
        return current
    }

    private fun ready(value: RefloatControls) = value.packageEnabled != null && value.flywheelEnabled != null &&
        (!value.lightingAvailable || value.ledsEnabled != null && value.headlightsEnabled != null) &&
        (!value.fatalErrorClearAvailable || value.fatalErrorLatched != null && value.fatalErrorActive != null)

    private fun confirm(value: RefloatControls): RefloatControls {
        val satisfied = when (value.pendingAction) {
            RefloatPendingAction.PACKAGE -> value.packageEnabled == expected
            RefloatPendingAction.FLYWHEEL -> value.flywheelEnabled == expected
            RefloatPendingAction.LEDS -> value.ledsEnabled == expected
            RefloatPendingAction.HEADLIGHTS -> value.headlightsEnabled == expected
            RefloatPendingAction.CLEAR_FATAL_ERROR -> expected == false && value.fatalErrorLatched == false && value.fatalErrorActive == false
            null -> false
        }
        if (!satisfied) return value
        expected = null
        return value.copy(pendingAction = null)
    }
}
