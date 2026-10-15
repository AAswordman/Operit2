package app.operit

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

class HostEventHandoffTest {
    private val schedule = AndroidHostEventSchedule(
        scheduleId = "hook-1",
        kind = "timer",
        delayMs = 1000L,
        intervalMs = null,
        nextFireAtMillis = 1100L,
    )
    private val firedAtMillis = 5000L
    private val retained = mutableListOf<UndeliveredHostEventFire>()


    @Test
    fun broadcastCompletionDeliversWithoutRetaining() {
        var started = 0
        var submitted = 0
        var emitted = 0

        val beginOutcome = HostEventHandoff.begin(
            schedule = schedule,
            firedAtMillis = firedAtMillis,
            startCore = { started++ },
            submitBackground = { _, _ -> submitted++ },
            retain = { retained.add(it) },
        )
        val deliveryOutcome = HostEventHandoff.completeDelivery(
            schedule = schedule,
            firedAtMillis = firedAtMillis,
            emit = { emitted++ },
            retain = { retained.add(it) },
        )

        assertEquals(HostEventHandoffOutcome.Submitted, beginOutcome)
        assertEquals(HostEventHandoffOutcome.Delivered, deliveryOutcome)
        assertEquals(1, started)
        assertEquals(1, submitted)
        assertEquals(1, emitted)
        assertTrue(retained.isEmpty())
    }


    @Test
    fun coreStartFailureIsRetainedAsStartCore() {
        val outcome = HostEventHandoff.begin(
            schedule = schedule,
            firedAtMillis = firedAtMillis,
            startCore = { throw IllegalStateException("core service unavailable") },
            submitBackground = { _, _ -> throw AssertionError("must not submit after a start failure") },
            retain = { retained.add(it) },
        )

        assertTrue(outcome is HostEventHandoffOutcome.Undelivered)
        assertNull((outcome as HostEventHandoffOutcome.Undelivered).retentionError)
        val fire = outcome.fire
        assertEquals(HostEventHandoffStage.START_CORE, fire.stage)
        assertEquals("hook-1", fire.scheduleId)
        assertEquals(firedAtMillis, fire.firedAtMillis)
        assertTrue(fire.reason.contains("core service unavailable"))
        assertEquals(1, retained.size)
    }

    @Test
    fun backgroundSubmitFailureIsRetainedOnceWithoutRetry() {
        var submitCalls = 0

        val outcome = HostEventHandoff.begin(
            schedule = schedule,
            firedAtMillis = firedAtMillis,
            startCore = {},
            submitBackground = { _, _ ->
                submitCalls++
                throw IllegalStateException("bridge not ready")
            },
            retain = { retained.add(it) },
        )

        assertTrue(outcome is HostEventHandoffOutcome.Undelivered)
        val undelivered = outcome as HostEventHandoffOutcome.Undelivered
        assertEquals(HostEventHandoffStage.SUBMIT_BACKGROUND, undelivered.fire.stage)
        assertTrue(undelivered.fire.reason.contains("bridge not ready"))
        assertEquals(1, submitCalls)
        assertEquals(1, retained.size)
    }


    @Test
    fun coreRejectionAnswerIsRetainedAsDelivery() {
        val outcome = HostEventHandoff.completeDelivery(
            schedule = schedule,
            firedAtMillis = firedAtMillis,
            emit = { check(false) { "Core rejected Android host event schedule" } },
            retain = { retained.add(it) },
        )

        assertTrue(outcome is HostEventHandoffOutcome.Undelivered)
        val undelivered = outcome as HostEventHandoffOutcome.Undelivered
        assertEquals(HostEventHandoffStage.DELIVERY, undelivered.fire.stage)
        assertTrue(undelivered.fire.reason.contains("Core rejected"))
        assertTrue(undelivered.cause is IllegalStateException)
        assertEquals(1, retained.size)
    }

    @Test
    fun thrownEmitErrorIsRetainedAsDelivery() {
        val nativeError = RuntimeException("native emit failed")

        val outcome = HostEventHandoff.completeDelivery(
            schedule = schedule,
            firedAtMillis = firedAtMillis,
            emit = { throw nativeError },
            retain = { retained.add(it) },
        )

        assertTrue(outcome is HostEventHandoffOutcome.Undelivered)
        val undelivered = outcome as HostEventHandoffOutcome.Undelivered
        assertEquals(HostEventHandoffStage.DELIVERY, undelivered.fire.stage)
        assertSame(nativeError, undelivered.cause)
        assertEquals(1, retained.size)
    }


    @Test
    fun retainedRecordEncodesStageAndReason() {
        val fire = UndeliveredHostEventFire(
            scheduleId = "hook-1",
            firedAtMillis = firedAtMillis,
            stage = HostEventHandoffStage.DELIVERY,
            reason = "boom",
        )

        val json = fire.toJson()

        assertEquals("hook-1", json.getString("scheduleId"))
        assertEquals(firedAtMillis, json.getLong("firedAtMillis"))
        assertEquals("DELIVERY", json.getString("stage"))
        assertEquals("boom", json.getString("reason"))
        assertEquals(fire, UndeliveredHostEventFire.fromJson(json))
    }

    @Test
    fun coreStartErrorIsRetainedInsteadOfEscaping() {
        val failure = Error("core start exploded")

        val outcome = HostEventHandoff.begin(
            schedule = schedule,
            firedAtMillis = firedAtMillis,
            startCore = { throw failure },
            submitBackground = { _, _ -> throw AssertionError("must not submit after a start failure") },
            retain = { retained.add(it) },
        )

        assertTrue(outcome is HostEventHandoffOutcome.Undelivered)
        val undelivered = outcome as HostEventHandoffOutcome.Undelivered
        assertEquals(HostEventHandoffStage.START_CORE, undelivered.fire.stage)
        assertSame(failure, undelivered.cause)
        assertEquals(1, retained.size)
    }
    @Test
    fun startFailureAndRetentionFailureAreBothReported() {
        val deliveryError = IllegalStateException("service unavailable")
        val storageError = IllegalStateException("storage unavailable")
        var retentionCalls = 0
        val outcome = HostEventHandoff.begin(
            schedule, firedAtMillis,
            startCore = { throw deliveryError },
            submitBackground = { _, _ -> throw AssertionError("must not submit") },
            retain = { retentionCalls++; throw storageError },
        ) as HostEventHandoffOutcome.Undelivered

        assertSame(deliveryError, outcome.cause)
        assertSame(storageError, outcome.retentionError)
        assertEquals(HostEventHandoffStage.START_CORE, outcome.fire.stage)
        assertEquals(1, retentionCalls)
    }

    @Test
    fun submissionFailureAndRetentionErrorAreBothReported() {
        val deliveryError = java.util.concurrent.RejectedExecutionException("executor stopped")
        val storageError = AssertionError("storage failed")
        val outcome = HostEventHandoff.begin(
            schedule, firedAtMillis,
            startCore = {},
            submitBackground = { _, _ -> throw deliveryError },
            retain = { throw storageError },
        ) as HostEventHandoffOutcome.Undelivered

        assertSame(deliveryError, outcome.cause)
        assertSame(storageError, outcome.retentionError)
        assertEquals(HostEventHandoffStage.SUBMIT_BACKGROUND, outcome.fire.stage)
    }

    @Test
    fun deliveryFailureAndRetentionFailureAreBothReported() {
        val deliveryError = IllegalStateException("Core rejected firing")
        val storageError = IllegalStateException("disk full")
        val outcome = HostEventHandoff.completeDelivery(
            schedule, firedAtMillis,
            emit = { throw deliveryError },
            retain = { throw storageError },
        ) as HostEventHandoffOutcome.Undelivered

        assertSame(deliveryError, outcome.cause)
        assertSame(storageError, outcome.retentionError)
        assertEquals(HostEventHandoffStage.DELIVERY, outcome.fire.stage)
    }

}
