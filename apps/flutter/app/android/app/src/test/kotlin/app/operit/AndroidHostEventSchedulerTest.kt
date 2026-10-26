package app.operit

import android.app.AlarmManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.net.Uri
import android.util.Log
import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.mockito.ArgumentMatchers.anyString
import org.mockito.Mockito

class AndroidHostEventSchedulerTest {
    private val context = Mockito.mock(Context::class.java)
    private val preferences = Mockito.mock(SharedPreferences::class.java)
    private val editor = Mockito.mock(SharedPreferences.Editor::class.java)
    private val stored = mutableMapOf<String, String>()
    private val writes = mutableMapOf<String, String>()
    private val logs = Mockito.mockStatic(Log::class.java)
    private var commitsSucceed = true

    private fun fire(id: String = "hook-1") = UndeliveredHostEventFire(
        id, 1234L, HostEventHandoffStage.DELIVERY, "Core rejected firing",
    )

    @Before
    fun setUp() {
        Mockito.`when`(context.applicationContext).thenReturn(context)
        Mockito.`when`(context.getSharedPreferences("operit_host_event_schedules", Context.MODE_PRIVATE))
            .thenReturn(preferences)
        Mockito.`when`(preferences.getString(anyString(), anyString())).thenAnswer {
            stored[it.getArgument<String>(0)] ?: it.getArgument<String>(1)
        }
        Mockito.`when`(preferences.edit()).thenReturn(editor)
        Mockito.`when`(editor.putString(anyString(), anyString())).thenAnswer {
            writes[it.getArgument(0)] = it.getArgument(1)
            editor
        }
        Mockito.`when`(editor.commit()).thenAnswer {
            // SharedPreferences updates its cache even when the disk commit fails.
            stored.putAll(writes)
            writes.clear()
            commitsSucceed
        }
    }

    @After
    fun tearDown() {
        logs.close()
    }

    @Test
    fun retainedFailureCanBeReadWithoutConsumingIt() {
        assertTrue(AndroidHostEventScheduler.readUndelivered(context).isEmpty())
        AndroidHostEventScheduler.retainUndelivered(context, fire())
        val expected = listOf(fire())
        assertEquals(expected, AndroidHostEventScheduler.readUndelivered(context))
        assertEquals(expected, AndroidHostEventScheduler.readUndelivered(context))
        assertEquals("DELIVERY", expected.single().toMap()["stage"])
        assertEquals(1234L, expected.single().toMap()["firedAtMillis"])
    }

    @Test
    fun retainedFailuresAreBoundedAndPreserveTheNewestRecords() {
        repeat(35) { AndroidHostEventScheduler.retainUndelivered(context, fire("hook-$it")) }
        val retained = AndroidHostEventScheduler.readUndelivered(context)
        assertEquals(32, retained.size)
        assertEquals("hook-3", retained.first().scheduleId)
        assertEquals("hook-34", retained.last().scheduleId)
    }

    @Test
    fun malformedStorageIsReportedWithoutOverwritingIt() {
        stored["undelivered_fires"] = "not-json"
        assertThrows(JSONException::class.java) {
            AndroidHostEventScheduler.readUndelivered(context)
        }
        assertThrows(JSONException::class.java) {
            AndroidHostEventScheduler.retainUndelivered(context, fire())
        }
        assertEquals("not-json", stored["undelivered_fires"])
        Mockito.verify(preferences, Mockito.never()).edit()
    }

    @Test
    fun unavailableStorageIsReportedInsteadOfReturningAnEmptyHistory() {
        Mockito.`when`(preferences.getString("undelivered_fires", "[]")).thenReturn(null)
        assertThrows(IllegalStateException::class.java) {
            AndroidHostEventScheduler.readUndelivered(context)
        }
    }

    @Test
    fun invalidStoredFailureIsRejected() {
        stored["undelivered_fires"] = """[{"scheduleId":"x","firedAtMillis":1,"stage":"UNKNOWN","reason":"x"}]"""
        assertThrows(IllegalArgumentException::class.java) {
            AndroidHostEventScheduler.readUndelivered(context)
        }
    }

    @Test
    fun failedCommitIsReportedAlongsideTheOriginalDeliveryFailure() {
        commitsSucceed = false
        val deliveryError = IllegalStateException("service unavailable")
        val schedule = AndroidHostEventSchedule("hook-1", "timer", 1000L, null, 1234L)
        val outcome = HostEventHandoff.begin(
            schedule, 1234L,
            storageReady = { true },
            startCore = { throw deliveryError },
            submitBackground = { _, _ -> throw AssertionError("must not submit") },
            retain = { AndroidHostEventScheduler.retainUndelivered(context, it) },
        ) as HostEventHandoffOutcome.Undelivered

        assertSame(deliveryError, outcome.cause)
        assertNotNull(outcome.retentionError)
        assertTrue(outcome.retentionError!!.message!!.contains("failed to persist"))
        assertEquals(1, AndroidHostEventScheduler.readUndelivered(context).size)
    }

    @Test
    fun missingScheduleIdFinishesBroadcastExactlyOnce() {
        receive(null)
    }

    @Test
    fun unknownScheduleFinishesBroadcastExactlyOnce() {
        receive("unknown")
    }

    @Test
    fun corruptScheduleStorageFinishesBroadcastExactlyOnce() {
        stored["schedules"] = "not-json"
        receive("hook-1")
    }

    @Test
    fun rejectedScheduleCommitFinishesBroadcastExactlyOnce() {
        stored["schedules"] = """[{"scheduleId":"hook-1","kind":"timer","delayMs":1000,"intervalMs":null,"nextFireAtMillis":1234}]"""
        commitsSucceed = false
        receive("hook-1")
        Mockito.verify(context, Mockito.never()).startService(Mockito.any(Intent::class.java))
        Mockito.verify(context, Mockito.never()).startForegroundService(Mockito.any(Intent::class.java))
    }

    @Test
    fun fatalStorageErrorStillFinishesBroadcast() {
        val receiver = Mockito.mock(HostEventScheduleReceiver::class.java, Mockito.CALLS_REAL_METHODS)
        val pending = Mockito.mock(BroadcastReceiver.PendingResult::class.java)
        val intent = Mockito.mock(Intent::class.java)
        Mockito.doReturn(pending).`when`(receiver).goAsync()
        Mockito.`when`(intent.getStringExtra("scheduleId")).thenReturn("hook-1")
        val fatal = AssertionError("storage failed")
        Mockito.`when`(preferences.getString("schedules", "[]")).thenThrow(fatal)

        val thrown = assertThrows(AssertionError::class.java) { receiver.onReceive(context, intent) }
        assertSame(fatal, thrown)
        Mockito.verify(pending, Mockito.times(1)).finish()
    }


    @Test
    fun coreStartFailureIsRetainedAndFinishesBroadcast() {
        stored["schedules"] = """[{"scheduleId":"hook-1","kind":"timer","delayMs":1000,"intervalMs":null,"nextFireAtMillis":1234}]"""
        val failure = SecurityException("service start denied")
        Mockito.`when`(context.startService(Mockito.any(Intent::class.java))).thenThrow(failure)
        Mockito.`when`(context.startForegroundService(Mockito.any(Intent::class.java))).thenThrow(failure)
        Mockito.mockConstruction(Intent::class.java).use {
            receive("hook-1")
        }

        val retained = AndroidHostEventScheduler.readUndelivered(context).single()
        assertEquals(HostEventHandoffStage.START_CORE, retained.stage)
        assertTrue(retained.reason.contains("service start denied"))
        assertEquals("[]", stored["schedules"])
        assertEquals(1, AndroidHostEventScheduler.consecutiveDeliveryFailures(context, "hook-1"))
    }

    @Test
    fun waitingForStorageSkipsDeliveryWithoutRetentionAndCountsTowardBackoff() {
        val fireAt = System.currentTimeMillis() + 60_000L
        stored["schedules"] = """[{"scheduleId":"hook-1","kind":"interval","delayMs":1000,"intervalMs":60000,"nextFireAtMillis":$fireAt}]"""
        withAlarms {
            receive("hook-1", storageReady = false)
        }
        assertTrue(AndroidHostEventScheduler.readUndelivered(context).isEmpty())
        assertEquals(1, AndroidHostEventScheduler.consecutiveDeliveryFailures(context, "hook-1"))
        Mockito.verify(context, Mockito.never()).startForegroundService(Mockito.any(Intent::class.java))
        Mockito.verify(context, Mockito.never()).startService(Mockito.any(Intent::class.java))
    }

    @Test
    fun deliveryBackoffStartsAfterTheThresholdAndIsCapped() {
        assertEquals(0L, AndroidHostEventScheduler.deliveryBackoffMillis(60_000L, 0))
        assertEquals(0L, AndroidHostEventScheduler.deliveryBackoffMillis(60_000L, AndroidHostEventScheduler.deliveryBackoffThreshold - 1))
        assertEquals(
            60_000L,
            AndroidHostEventScheduler.deliveryBackoffMillis(60_000L, AndroidHostEventScheduler.deliveryBackoffThreshold),
        )
        assertEquals(120_000L, AndroidHostEventScheduler.deliveryBackoffMillis(60_000L, AndroidHostEventScheduler.deliveryBackoffThreshold + 1))
        assertEquals(
            AndroidHostEventScheduler.maxDeliveryBackoffMillis,
            AndroidHostEventScheduler.deliveryBackoffMillis(60_000L, 500),
        )
        assertEquals(0L, AndroidHostEventScheduler.deliveryBackoffMillis(60_000L, 0))
    }

    @Test
    fun consumeFireDelaysTheNextIntervalFireAfterRecordedFailures() {
        val fireAt = System.currentTimeMillis() + 60_000L
        stored["schedules"] = """[{"scheduleId":"hook-1","kind":"interval","delayMs":1000,"intervalMs":60000,"nextFireAtMillis":$fireAt}]"""
        withAlarms {
            repeat(AndroidHostEventScheduler.deliveryBackoffThreshold) {
                AndroidHostEventScheduler.recordDeliveryFailure(context, "hook-1")
            }
            val consumed = AndroidHostEventScheduler.consumeFire(context, "hook-1")!!
            assertEquals(fireAt, consumed.nextFireAtMillis)
        }
        val storedNext = JSONArray(stored["schedules"]!!)
            .getJSONObject(0)
            .getLong("nextFireAtMillis")
        assertEquals(fireAt + 120_000L, storedNext)
    }

    @Test
    fun deliverySuccessClearsRecordedFailuresSoTheNextFireKeepsTheRegularBoundary() {
        val fireAt = System.currentTimeMillis() + 60_000L
        stored["schedules"] = """[{"scheduleId":"hook-1","kind":"interval","delayMs":1000,"intervalMs":60000,"nextFireAtMillis":$fireAt}]"""
        AndroidHostEventScheduler.recordDeliveryFailure(context, "hook-1")
        AndroidHostEventScheduler.recordDeliveryFailure(context, "hook-1")
        assertEquals(2, AndroidHostEventScheduler.consecutiveDeliveryFailures(context, "hook-1"))
        AndroidHostEventScheduler.recordDeliverySuccess(context, "hook-1")
        assertEquals(0, AndroidHostEventScheduler.consecutiveDeliveryFailures(context, "hook-1"))
        withAlarms {
            AndroidHostEventScheduler.consumeFire(context, "hook-1")
        }
        val storedNext = JSONArray(stored["schedules"]!!)
            .getJSONObject(0)
            .getLong("nextFireAtMillis")
        assertEquals(fireAt + 60_000L, storedNext)
    }

    @Test
    fun replaceSchedulesPrunesDeliveryFailureCountsForRemovedSchedules() {
        AndroidHostEventScheduler.recordDeliveryFailure(context, "hook-1")
        AndroidHostEventScheduler.recordDeliveryFailure(context, "hook-2")
        withAlarms {
            AndroidHostEventScheduler.replaceSchedules(
                context,
                """[{"scheduleId":"hook-1","kind":"interval","delayMs":1000,"intervalMs":60000}]""",
            )
        }
        assertEquals("""{"hook-1":1}""", stored["delivery_failure_counts"])
    }

    @Test
    fun corruptDeliveryFailureCountsFailOpenWithoutBlockingScheduling() {
        stored["delivery_failure_counts"] = "not-json"
        assertEquals(0, AndroidHostEventScheduler.consecutiveDeliveryFailures(context, "hook-1"))
        assertEquals(1, AndroidHostEventScheduler.recordDeliveryFailure(context, "hook-1"))
        assertEquals("""{"hook-1":1}""", stored["delivery_failure_counts"])
    }

    /** Installs alarm-manager, PendingIntent, Intent, and Uri.Builder test doubles for scheduling code paths. */
    private fun withAlarms(block: () -> Unit) {
        val alarmManager = Mockito.mock(AlarmManager::class.java)
        Mockito.`when`(context.getSystemService(Context.ALARM_SERVICE)).thenReturn(alarmManager)
        Mockito.mockStatic(PendingIntent::class.java).use { pendingIntents ->
            val pendingIntent = Mockito.mock(PendingIntent::class.java)
            pendingIntents.`when`<PendingIntent> {
                PendingIntent.getBroadcast(
                    Mockito.any(),
                    Mockito.anyInt(),
                    Mockito.any(),
                    Mockito.anyInt(),
                )
            }.thenReturn(pendingIntent)
            Mockito.mockConstruction(
                Intent::class.java,
                Mockito.withSettings().defaultAnswer(Mockito.RETURNS_DEEP_STUBS),
            ).use {
                Mockito.mockConstruction(
                    Uri.Builder::class.java,
                    Mockito.withSettings().defaultAnswer(Mockito.RETURNS_DEEP_STUBS),
                ).use {
                    block()
                }
            }
        }
    }

    private fun receive(
        scheduleId: String?,
        storageReady: Boolean = true,
    ) {
        val receiver = Mockito.mock(HostEventScheduleReceiver::class.java, Mockito.CALLS_REAL_METHODS)
        val pending = Mockito.mock(BroadcastReceiver.PendingResult::class.java)
        val intent = Mockito.mock(Intent::class.java)
        Mockito.doReturn(pending).`when`(receiver).goAsync()
        Mockito.doReturn(storageReady).`when`(receiver).storageConfigured(context)
        Mockito.`when`(intent.getStringExtra("scheduleId")).thenReturn(scheduleId)

        receiver.onReceive(context, intent)

        Mockito.verify(pending, Mockito.times(1)).finish()
    }
}
