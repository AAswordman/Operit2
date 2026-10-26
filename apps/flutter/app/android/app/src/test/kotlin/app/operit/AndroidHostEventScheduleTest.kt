package app.operit

import org.json.JSONException
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/** Schedule configuration, storage decoding, and interval boundary tests. */
class AndroidHostEventScheduleTest {
    private fun command(kind: String, delayMs: Long, intervalMs: Any?): JSONObject =
        JSONObject()
            .put("scheduleId", "hook-1")
            .put("kind", kind)
            .put("delayMs", delayMs)
            .put("intervalMs", intervalMs ?: JSONObject.NULL)

    private fun stored(kind: String, delayMs: Long, intervalMs: Any?, nextFireAtMillis: Long): JSONObject =
        JSONObject()
            .put("scheduleId", "hook-1")
            .put("kind", kind)
            .put("delayMs", delayMs)
            .put("intervalMs", intervalMs ?: JSONObject.NULL)
            .put("nextFireAtMillis", nextFireAtMillis)


    @Test
    fun validTimerCommandParses() {
        val schedule = AndroidHostEventSchedule.fromCommand(command("timer", 1000L, null), 100L)
        assertEquals("hook-1", schedule.scheduleId)
        assertEquals(1100L, schedule.nextFireAtMillis)
    }

    @Test
    fun storedScheduleRoundTripsThroughJson() {
        val original = AndroidHostEventSchedule.fromCommand(command("interval", 1000L, 5000L), 100L)
        val decoded = AndroidHostEventSchedule.fromStored(original.toJson())
        assertEquals(original, decoded)
    }


    @Test
    fun commandRejectsNonPositiveInterval() {
        val error = assertThrows(IllegalArgumentException::class.java) {
            AndroidHostEventSchedule.fromCommand(command("interval", 1000L, 0L), 0L)
        }
        assertTrue(error.message!!.contains("interval"))
    }

    @Test
    fun storedRejectsNonPositiveInterval() {
        val error = assertThrows(IllegalArgumentException::class.java) {
            AndroidHostEventSchedule.fromStored(stored("interval", 1000L, -5L, 42L))
        }
        assertTrue(error.message!!.contains("interval"))
    }

    @Test
    fun storedRejectsNegativeDelay() {
        assertThrows(IllegalArgumentException::class.java) {
            AndroidHostEventSchedule.fromStored(stored("timer", -1L, null, 42L))
        }
    }


    @Test
    fun commandOverflowPropagatesAsArithmeticException() {
        // nowMillis + delayMs overflows Long.MAX_VALUE
        assertThrows(ArithmeticException::class.java) {
            AndroidHostEventSchedule.fromCommand(command("timer", 1000L, null), Long.MAX_VALUE - 1)
        }
    }


    @Test
    fun storedRejectsUnknownKind() {
        val error = assertThrows(IllegalArgumentException::class.java) {
            AndroidHostEventSchedule.fromStored(stored("reboot", 1000L, null, 42L))
        }
        assertTrue(error.message!!.contains("kind"))
    }

    @Test
    fun storedRejectsMissingField() {
        assertThrows(JSONException::class.java) {
            AndroidHostEventSchedule.fromStored(JSONObject())
        }
    }

    @Test
    fun storedTimerRejectsDefinedInterval() {
        assertThrows(IllegalArgumentException::class.java) {
            AndroidHostEventSchedule.fromStored(stored("timer", 1000L, 5000L, 42L))
        }
    }

    @Test
    fun storedIntervalRejectsMissingInterval() {
        assertThrows(IllegalArgumentException::class.java) {
            AndroidHostEventSchedule.fromStored(stored("interval", 1000L, null, 42L))
        }
    }

    @Test
    fun storedRejectsNegativeNextFireAtMillis() {
        val error = assertThrows(IllegalArgumentException::class.java) {
            AndroidHostEventSchedule.fromStored(stored("interval", 1000L, 5000L, -1L))
        }
        assertTrue(error.message!!.contains("nextFireAtMillis"))
    }

    // Interval re-arm boundaries and arithmetic overflow.

    @Test
    fun nextIntervalTimeAdvancesOneBoundaryWhenScheduledInFuture() {
        val next = AndroidHostEventScheduler.nextIntervalTime(
            scheduledAtMillis = 1_000L,
            intervalMs = 500L,
            nowMillis = 900L,
        )
        assertEquals(1_500L, next)
    }

    @Test
    fun nextIntervalTimeCatchUpSkipsMissedBoundaries() {
        // The alarm fired 2.5 intervals late: skip the missed boundaries instead of
        // re-arming them one by one (multiplyExact path).
        val next = AndroidHostEventScheduler.nextIntervalTime(
            scheduledAtMillis = 0L,
            intervalMs = 1_000L,
            nowMillis = 2_500L,
        )
        assertEquals(3_000L, next)
    }

    @Test
    fun nextIntervalTimeFutureBranchOverflowPropagates() {
        // addExact(scheduledAt, intervalMs) overflows on the future branch.
        assertThrows(ArithmeticException::class.java) {
            AndroidHostEventScheduler.nextIntervalTime(
                scheduledAtMillis = Long.MAX_VALUE - 5,
                intervalMs = 10L,
                nowMillis = 0L,
            )
        }
    }

    @Test
    fun nextIntervalTimeCatchUpOverflowPropagates() {
        // multiplyExact(steps, intervalMs) overflows on the catch-up branch.
        assertThrows(ArithmeticException::class.java) {
            AndroidHostEventScheduler.nextIntervalTime(
                scheduledAtMillis = 0L,
                intervalMs = 2L,
                nowMillis = Long.MAX_VALUE,
            )
        }
    }
}
