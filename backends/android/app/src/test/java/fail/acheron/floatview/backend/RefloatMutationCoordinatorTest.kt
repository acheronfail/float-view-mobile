package fail.acheron.floatview.backend

import fail.acheron.floatview.protocol.refloat.RefloatBooleanControl
import fail.acheron.floatview.protocol.refloat.RefloatControlCodec
import fail.acheron.floatview.protocol.refloat.RefloatControlProfile
import fail.acheron.floatview.protocol.refloat.RefloatLightingProfile
import fail.acheron.floatview.protocol.refloat.RefloatLightingState
import fail.acheron.floatview.protocol.vesc.ControllerRoute
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Test

class RefloatMutationCoordinatorTest {
    private val profile = RefloatControlProfile(RefloatLightingProfile.STABLE_COMMAND_20, true)

    private fun context(
        generation: Long = 1,
        route: ControllerRoute = ControllerRoute.Direct,
        writable: Boolean = true,
        rideState: RideState = RideState.IDLE,
        packageState: PackageState = PackageState.READY,
        packageMode: PackageMode = PackageMode.NORMAL,
    ) = RefloatMutationContext(
        VescRequestGeneration(generation), route, profile, true, writable,
        rideState, packageState, packageMode,
    )

    private fun ready(mode: PackageMode = PackageMode.NORMAL, latched: Boolean = false, fault: FirmwareFault? = null) =
        DeviceMutationCoordinator().also {
            it.configure(VescRequestGeneration(1), profile, true)
            it.observeLighting(RefloatLightingState(false, false))
            it.observeTelemetry(RefloatTelemetryRecord(RideState.IDLE, packageState = PackageState.READY, packageMode = mode, firmwareFault = fault, fatalErrorLatched = latched))
        }

    @Test fun serializesDirectAndCanAndConfirmsAuthoritatively() {
        listOf(ControllerRoute.Direct, ControllerRoute.Can(42)).forEach { route ->
            val owner = ready()
            val result = owner.authorizeSet(RefloatBooleanControl.PACKAGE, false, context(route = route))
                as RefloatMutationAuthorization.Mutation
            assertEquals(RefloatPendingAction.PACKAGE, result.controls.pendingAction)
            assertThrows(RefloatMutationException::class.java) {
                owner.authorizeSet(RefloatBooleanControl.LEDS, true, context(route = route))
            }
            val confirmed = owner.observeTelemetry(RefloatTelemetryRecord(RideState.IDLE, packageState = PackageState.DISABLED, packageMode = PackageMode.NORMAL, fatalErrorLatched = false))
            assertNull(confirmed?.pendingAction)
            assertEquals(false, confirmed?.packageEnabled)
        }
    }

    @Test fun flywheelModeOnlyAllowsEligibleDisable() {
        RefloatBooleanControl.entries.forEach { control ->
            val owner = ready(PackageMode.FLYWHEEL)
            if (control == RefloatBooleanControl.FLYWHEEL) {
                val mutation = owner.authorizeSet(control, false, context(rideState = RideState.RIDING, packageState = PackageState.RUNNING, packageMode = PackageMode.FLYWHEEL)) as RefloatMutationAuthorization.Mutation
                assertArrayEquals(RefloatControlCodec.flywheelRequest(false), mutation.mutation.payload)
            } else {
                assertThrows(RefloatMutationException::class.java) {
                    owner.authorizeSet(control, true, context(rideState = RideState.RIDING, packageState = PackageState.RUNNING, packageMode = PackageMode.FLYWHEEL))
                }
            }
        }
    }

    @Test fun failsClosedForSafetyAccessGenerationAndActiveFault() {
        listOf(
            context(writable = false), context(rideState = RideState.RIDING), context(generation = 2),
        ).forEach { denied ->
            assertThrows(RefloatMutationException::class.java) {
                ready().authorizeSet(RefloatBooleanControl.PACKAGE, false, denied)
            }
        }
        assertThrows(RefloatMutationException::class.java) {
            ready(latched = true, fault = FirmwareFault.OVER_VOLTAGE).authorizeFatalClear(context())
        }
        val clearable = ready(latched = true)
        val clear = clearable.authorizeFatalClear(context()) as RefloatMutationAuthorization.Mutation
        assertArrayEquals(byteArrayOf(36, 101, 36, 1), clear.mutation.payload)
        assertEquals(RefloatPendingAction.CLEAR_FATAL_ERROR, clearable.failPending(VescRequestGeneration(2))?.pendingAction)
        assertNull(clearable.failPending(VescRequestGeneration(1))?.pendingAction)
    }

    @Test fun shutdownSharesIdleFlywheelGenerationAndPendingMutationGates() {
        val owner = ready()
        val context = ControllerShutdownContext(
            VescRequestGeneration(1), true, true, RideState.IDLE, PackageMode.NORMAL,
        )
        assertEquals(ControllerPendingAction.SHUTDOWN, owner.authorizeShutdown(context).pendingAction)
        assertThrows(RefloatMutationException::class.java) {
            owner.authorizeSet(RefloatBooleanControl.LEDS, true, context())
        }
        assertNull(owner.clearShutdown(VescRequestGeneration(2)))
        assertEquals(ControllerPendingAction.SHUTDOWN, owner.controllerActions(false).pendingAction)
        assertNull(owner.clearShutdown(VescRequestGeneration(1))?.pendingAction)

        listOf(
            ControllerShutdownContext(VescRequestGeneration(2), true, true, RideState.IDLE, PackageMode.NORMAL),
            ControllerShutdownContext(VescRequestGeneration(1), true, true, RideState.RIDING, PackageMode.NORMAL),
            ControllerShutdownContext(VescRequestGeneration(1), true, true, RideState.IDLE, PackageMode.FLYWHEEL),
            ControllerShutdownContext(VescRequestGeneration(1), true, false, RideState.IDLE, PackageMode.NORMAL),
        ).forEach { denied ->
            assertThrows(RefloatMutationException::class.java) { ready().authorizeShutdown(denied) }
        }

        val competing = ready()
        competing.authorizeSet(RefloatBooleanControl.LEDS, true, context())
        assertThrows(RefloatMutationException::class.java) { competing.authorizeShutdown(context) }
    }

    @Test fun configurationSharesSoleMutationGateAndEnforcesSafety() {
        val valid = RefloatConfigurationMutationContext(
            VescRequestGeneration(1), true, true, RideState.IDLE, FootpadState.NONE,
            PackageState.READY, PackageMode.NORMAL,
        )
        val owner = ready()
        owner.authorizeConfiguration(valid)
        assertEquals(false, owner.controllerActions(true).shutdownAvailable)
        assertThrows(RefloatMutationException::class.java) {
            owner.authorizeSet(RefloatBooleanControl.LEDS, true, context())
        }
        assertThrows(RefloatMutationException::class.java) { owner.authorizeConfiguration(valid) }
        owner.finishConfiguration(VescRequestGeneration(2))
        assertThrows(RefloatMutationException::class.java) { owner.authorizeConfiguration(valid) }
        owner.finishConfiguration(VescRequestGeneration(1))
        owner.authorizeConfiguration(valid)

        listOf(
            valid.copy(generation = VescRequestGeneration(2)),
            valid.copy(rideState = RideState.RIDING, packageState = PackageState.RUNNING),
            valid.copy(footpadState = FootpadState.LEFT),
            valid.copy(packageMode = PackageMode.FLYWHEEL),
        ).forEach { denied ->
            assertThrows(RefloatMutationException::class.java) { ready().authorizeConfiguration(denied) }
        }
    }
}
