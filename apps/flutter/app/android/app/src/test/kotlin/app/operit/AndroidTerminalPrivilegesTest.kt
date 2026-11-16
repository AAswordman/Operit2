package app.operit

import android.os.DeadObjectException
import android.os.IBinder
import android.os.Parcel
import java.util.concurrent.ConcurrentHashMap
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.mockito.Mockito.*

/** Exercises the production Binder failure handling without starting a privileged service. */
class AndroidTerminalPrivilegesTest {
    private class Fixture {
        val binder = mock(IBinder::class.java)
        val data = mock(Parcel::class.java)
        val reply = mock(Parcel::class.java)
        val sessions = ConcurrentHashMap<Int, Any>()
        // Existing-session operations need neither Android's main looper nor a service bind.
        val client = mock(AndroidTerminalPrivileges::class.java, CALLS_REAL_METHODS)
        private val serviceField = AndroidTerminalPrivileges::class.java
            .getDeclaredField("service").apply { isAccessible = true }

        init {
            AndroidTerminalPrivileges::class.java.getDeclaredField("sessions").apply {
                isAccessible = true
                set(client, sessions)
            }
            val remoteSession = AndroidTerminalPrivileges::class.java.declaredClasses
                .single { it.simpleName == "RemoteSession" }
                .getDeclaredConstructor(IBinder::class.java, Int::class.javaPrimitiveType)
                .apply { isAccessible = true }
            sessions[1] = remoteSession.newInstance(binder, 1234)
            sessions[2] = remoteSession.newInstance(binder, 5678)
            cachedService = binder
        }

        var cachedService: IBinder?
            get() = serviceField.get(client) as IBinder?
            set(value) = serviceField.set(client, value)

        fun failTransaction() {
            `when`(binder.transact(1, data, reply, 0))
                .thenThrow(mock(DeadObjectException::class.java))
        }

        fun execute(operation: String, handle: Int = 1): JSONObject =
            mockStatic(Parcel::class.java).use { parcels ->
                parcels.`when`<Parcel> { Parcel.obtain() }.thenReturn(data, reply)
                JSONObject(client.execute(JSONObject()
                    .put("operation", operation).put("pid", handle).toString()))
            }
    }

    /** One lost process must not make the runtime's entire session list fail. */
    @Test
    fun deadBinderPollEndsEveryOwnedSession() {
        val fixture = Fixture()
        fixture.failTransaction()
        for (handle in listOf(1, 2)) {
            val result = fixture.execute("poll", handle)
            assertFalse(result.has("error"))
            assertEquals(-1, result.getInt("exitCode"))
        }
        assertNull(fixture.cachedService)
        verify(fixture.data, times(2)).recycle()
        verify(fixture.reply, times(2)).recycle()
    }

    /** Cleanup releases the closed handle after the privileged process has disappeared. */
    @Test
    fun deadBinderCloseReleasesOnlyTheRequestedHandle() {
        val fixture = Fixture()
        fixture.failTransaction()
        val result = fixture.execute("close")
        assertFalse(result.has("error"))
        assertFalse(fixture.sessions.containsKey(1))
        assertTrue(fixture.sessions.containsKey(2))
        verify(fixture.data).recycle()
        verify(fixture.reply).recycle()
    }

    /** Loss of the service must still be reported for work that cannot succeed. */
    @Test
    fun deadBinderResizeStillFails() {
        val fixture = Fixture()
        fixture.failTransaction()
        assertTrue(fixture.execute("resize").has("error"))
        assertNull(fixture.cachedService)
    }

    /** A new connection must never receive an old service's possibly recycled PID. */
    @Test
    fun oldSessionsRemainPinnedAfterServiceReplacement() {
        val fixture = Fixture()
        fixture.failTransaction()
        val replacement = mock(IBinder::class.java)
        fixture.cachedService = replacement
        assertEquals(-1, fixture.execute("poll").getInt("exitCode"))
        assertSame(replacement, fixture.cachedService)
        verifyNoInteractions(replacement)
        verify(fixture.data).writeString(argThat<String> {
            JSONObject(it).getInt("pid") == 1234
        })
    }

    /** Ordinary live-session status and genuine service errors retain their meaning. */
    @Test
    fun liveBinderResponsesArePreserved() {
        val fixture = Fixture()
        `when`(fixture.binder.transact(1, fixture.data, fixture.reply, 0)).thenReturn(true)
        for (response in listOf("{\"exitCode\":null}", "{\"exitCode\":7}", "{\"error\":\"poll failed\"}")) {
            `when`(fixture.reply.readString()).thenReturn(response)
            assertEquals(JSONObject(response).toString(), fixture.execute("poll").toString())
            assertSame(fixture.binder, fixture.cachedService)
        }
    }
}
