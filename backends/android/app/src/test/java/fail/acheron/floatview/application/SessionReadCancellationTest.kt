package fail.acheron.floatview.application

import org.junit.Assert.assertEquals
import org.junit.Test

class SessionReadCancellationTest {
    @Test
    fun cancellationIsIsolatedToOneRequest() {
        val first = SessionReadCancellation()
        val second = SessionReadCancellation()
        var firstCallbacks = 0
        var secondCallbacks = 0
        first.register { firstCallbacks += 1 }
        second.register { secondCallbacks += 1 }

        second.cancel()
        second.cancel()

        assertEquals(0, firstCallbacks)
        assertEquals(1, secondCallbacks)
    }

    @Test
    fun lateRegistrationIsCancelledImmediately() {
        val cancellation = SessionReadCancellation()
        var callbacks = 0
        cancellation.cancel()

        cancellation.register { callbacks += 1 }

        assertEquals(1, callbacks)
    }
}
