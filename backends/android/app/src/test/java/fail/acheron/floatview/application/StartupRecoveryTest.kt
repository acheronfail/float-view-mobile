package fail.acheron.floatview.application

import org.junit.Assert.*
import org.junit.Test

class StartupRecoveryTest {
    private class Server : AutoCloseable {
        var closed = false
        override fun close() { closed = true }
    }

    @Test fun `data failure is recoverable without publishing a partial runtime`() {
        var fail = true
        val owner = Any()
        val startup = StartupRecovery(
            createRuntime = { if (fail) error("storage unavailable") else owner },
            createServer = { _: Any -> Server() }, startServer = {},
        )
        startup.retry()
        assertEquals(StartupFailure.DATA, startup.failure)
        assertNull(startup.runtime)
        assertNull(startup.server)
        fail = false
        startup.retry()
        assertSame(owner, startup.runtime)
        assertNotNull(startup.server)
        assertNull(startup.failure)
    }

    @Test fun `connection retry closes failed listener and retains one runtime`() {
        var owners = 0
        var fail = true
        val servers = mutableListOf<Server>()
        val startup = StartupRecovery(
            createRuntime = { owners++; Any() },
            createServer = { _: Any -> Server().also { servers.add(it) } },
            startServer = { if (fail) error("port occupied") },
        )
        startup.retry()
        val owner = startup.runtime
        assertEquals(StartupFailure.CONNECTION, startup.failure)
        assertTrue(servers.single().closed)
        fail = false
        startup.retry()
        startup.retry()
        assertSame(owner, startup.runtime)
        assertEquals(1, owners)
        assertEquals(2, servers.size)
        assertFalse(servers.last().closed)
        assertNull(startup.failure)
    }

    @Test fun `missing interface assets retain the runtime and classify separately`() {
        var fail = true
        val startup = StartupRecovery(
            createRuntime = { Any() },
            createServer = { _: Any -> if (fail) error("missing HTML") else Server() },
            startServer = {},
        )
        startup.retry()
        val owner = startup.runtime
        assertEquals(StartupFailure.INTERFACE, startup.failure)
        fail = false
        startup.retry()
        assertSame(owner, startup.runtime)
        assertNull(startup.failure)
    }
}
