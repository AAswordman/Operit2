package app.operit

import android.app.AlarmManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.util.Log
import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject

internal data class AndroidHostEventSchedule(
    val scheduleId: String,
    val kind: String,
    val delayMs: Long,
    val intervalMs: Long?,
    val nextFireAtMillis: Long,
) {
    /** Returns whether an existing system schedule still represents the same hook configuration. */
    fun hasSameConfiguration(other: AndroidHostEventSchedule): Boolean {
        return scheduleId == other.scheduleId &&
            kind == other.kind &&
            delayMs == other.delayMs &&
            intervalMs == other.intervalMs
    }

    /** Encodes the persistent Android schedule state. */
    fun toJson(): JSONObject = JSONObject()
        .put("scheduleId", scheduleId)
        .put("kind", kind)
        .put("delayMs", delayMs)
        .put("intervalMs", intervalMs ?: JSONObject.NULL)
        .put("nextFireAtMillis", nextFireAtMillis)

    companion object {
        /** Shared configuration rules; [source] names where the schedule came from in error messages. */
        private fun requireValidConfiguration(
            scheduleId: String,
            kind: String,
            delayMs: Long,
            intervalMs: Long?,
            source: String,
        ) {
            require(scheduleId.isNotBlank()) { "$source scheduleId is required" }
            require(kind == "timer" || kind == "interval") {
                "$source schedule kind must be timer or interval"
            }
            require(delayMs > 0L) { "$source schedule delayMs must be positive" }
            if (kind == "timer") {
                require(intervalMs == null) { "timer schedule must not define intervalMs" }
            }
            if (kind == "interval") {
                require(intervalMs != null && intervalMs > 0L) {
                    "interval schedule intervalMs must be positive"
                }
            }
        }

        /** Decodes and validates one schedule command received from Core. */
        fun fromCommand(value: JSONObject, nowMillis: Long): AndroidHostEventSchedule {
            val scheduleId = value.getString("scheduleId")
            val kind = value.getString("kind")
            val delayMs = value.getLong("delayMs")
            val intervalMs = if (value.isNull("intervalMs")) null else value.getLong("intervalMs")
            requireValidConfiguration(scheduleId, kind, delayMs, intervalMs, "host event")
            return AndroidHostEventSchedule(
                scheduleId = scheduleId,
                kind = kind,
                delayMs = delayMs,
                intervalMs = intervalMs,
                nextFireAtMillis = Math.addExact(nowMillis, delayMs),
            )
        }

        /**
         * Decodes one schedule persisted by the Android host.
         * Storage is validated with the same rules as incoming commands: corrupt data is
         * rejected here explicitly instead of surfacing later as a swallowed failure.
         */
        fun fromStored(value: JSONObject): AndroidHostEventSchedule {
            val scheduleId = value.getString("scheduleId")
            val kind = value.getString("kind")
            val delayMs = value.getLong("delayMs")
            val intervalMs = if (value.isNull("intervalMs")) null else value.getLong("intervalMs")
            val nextFireAtMillis = value.getLong("nextFireAtMillis")
            requireValidConfiguration(scheduleId, kind, delayMs, intervalMs, "stored host event")
            // Commands can never produce a negative instant; a stored one is corrupt storage.
            // Reject it here, before the rescheduler or the Core payload sees it.
            require(nextFireAtMillis >= 0L) {
                "stored host event schedule nextFireAtMillis must not be negative"
            }
            return AndroidHostEventSchedule(
                scheduleId = scheduleId,
                kind = kind,
                delayMs = delayMs,
                intervalMs = intervalMs,
                nextFireAtMillis = nextFireAtMillis,
            )
        }
    }
}

object AndroidHostEventScheduler {
    private const val logTag = "AndroidHostEventScheduler"
    private const val preferencesName = "operit_host_event_schedules"
    private const val schedulesKey = "schedules"
    private const val undeliveredFiresKey = "undelivered_fires"
    private const val retainedUndeliveredCap = 32
    private const val scheduleScheme = "operit-host-event"
    private const val scheduleAuthority = "schedule"
    private val lock = Any()

    /** Replaces the complete set of ToolPkg system schedules owned by Core. */
    fun replaceSchedules(context: Context, schedulesJson: String) {
        val applicationContext = context.applicationContext
        val nowMillis = System.currentTimeMillis()
        val desired = parseCommands(schedulesJson, nowMillis)
        synchronized(lock) {
            val current = readSchedules(applicationContext)
            for ((scheduleId, existing) in current) {
                if (!desired.containsKey(scheduleId)) {
                    cancelAlarm(applicationContext, existing.scheduleId)
                }
            }
            val reconciled = desired.mapValues { (scheduleId, requested) ->
                val existing = current[scheduleId]
                if (existing != null && existing.hasSameConfiguration(requested)) {
                    existing
                } else {
                    if (existing != null) {
                        cancelAlarm(applicationContext, existing.scheduleId)
                    }
                    requested
                }
            }
            writeSchedules(applicationContext, reconciled)
            for (schedule in reconciled.values) {
                scheduleAlarm(applicationContext, schedule)
            }
        }
    }

    /** Consumes one system alarm and persists the next interval occurrence before dispatch. */
    internal fun consumeFire(context: Context, scheduleId: String): AndroidHostEventSchedule? {
        val applicationContext = context.applicationContext
        synchronized(lock) {
            val schedules = readSchedules(applicationContext).toMutableMap()
            val schedule = schedules[scheduleId] ?: return null
            if (schedule.kind == "timer") {
                schedules.remove(scheduleId)
            } else {
                val intervalMs = requireNotNull(schedule.intervalMs) {
                    "stored interval schedule ${schedule.scheduleId} is missing intervalMs"
                }
                val nextFireAtMillis = nextIntervalTime(
                    schedule.nextFireAtMillis,
                    intervalMs,
                    System.currentTimeMillis(),
                )
                val nextSchedule = schedule.copy(nextFireAtMillis = nextFireAtMillis)
                schedules[scheduleId] = nextSchedule
                scheduleAlarm(applicationContext, nextSchedule)
            }
            writeSchedules(applicationContext, schedules)
            return schedule
        }
    }

    /** Parses the complete Core schedule command without accepting duplicate identities. */
    private fun parseCommands(
        schedulesJson: String,
        nowMillis: Long,
    ): Map<String, AndroidHostEventSchedule> {
        val values = JSONArray(schedulesJson)
        val schedules = linkedMapOf<String, AndroidHostEventSchedule>()
        for (index in 0 until values.length()) {
            val schedule = AndroidHostEventSchedule.fromCommand(values.getJSONObject(index), nowMillis)
            require(schedules.put(schedule.scheduleId, schedule) == null) {
                "duplicate host event scheduleId: ${schedule.scheduleId}"
            }
        }
        return schedules
    }

    /** Reads all persisted system schedule states. */
    private fun readSchedules(context: Context): Map<String, AndroidHostEventSchedule> {
        val encoded = context.getSharedPreferences(preferencesName, Context.MODE_PRIVATE)
            .getString(schedulesKey, "[]")
            ?: error("host event schedule storage is unavailable")
        val values = JSONArray(encoded)
        val schedules = linkedMapOf<String, AndroidHostEventSchedule>()
        for (index in 0 until values.length()) {
            val schedule = AndroidHostEventSchedule.fromStored(values.getJSONObject(index))
            require(schedules.put(schedule.scheduleId, schedule) == null) {
                "duplicate stored host event scheduleId: ${schedule.scheduleId}"
            }
        }
        return schedules
    }

    /** Atomically persists the complete Android schedule state. */
    private fun writeSchedules(
        context: Context,
        schedules: Map<String, AndroidHostEventSchedule>,
    ) {
        val values = JSONArray()
        for (schedule in schedules.values.sortedBy { it.scheduleId }) {
            values.put(schedule.toJson())
        }
        val committed = context.getSharedPreferences(preferencesName, Context.MODE_PRIVATE)
            .edit()
            .putString(schedulesKey, values.toString())
            .commit()
        check(committed) { "failed to persist Android host event schedules" }
    }

    /** Reads retained failures without clearing them or scheduling retries. */
    internal fun readUndelivered(context: Context): List<UndeliveredHostEventFire> {
        synchronized(lock) {
            val encoded = context.applicationContext
                .getSharedPreferences(preferencesName, Context.MODE_PRIVATE)
                .getString(undeliveredFiresKey, "[]")
                ?: error("undelivered host event storage is unavailable")
            val values = JSONArray(encoded)
            return (0 until values.length()).map { index ->
                UndeliveredHostEventFire.fromJson(values.getJSONObject(index))
            }
        }
    }

    /** Appends a diagnostic record under the schedule lock; storage failures propagate. */
    internal fun retainUndelivered(context: Context, fire: UndeliveredHostEventFire) {
        synchronized(lock) {
            val retained = readUndelivered(context).toMutableList()
            retained.add(fire)
            while (retained.size > retainedUndeliveredCap) {
                retained.removeAt(0)
                Log.w(logTag, "Undelivered host event records at cap ($retainedUndeliveredCap); dropping the oldest record")
            }
            val values = JSONArray()
            retained.forEach { values.put(it.toJson()) }
            val committed = context.applicationContext
                .getSharedPreferences(preferencesName, Context.MODE_PRIVATE)
                .edit()
                .putString(undeliveredFiresKey, values.toString())
                .commit()
            check(committed) { "failed to persist undelivered host event ${fire.scheduleId}" }
        }
    }

    /** Registers one wakeup alarm with Android's system AlarmManager. */
    private fun scheduleAlarm(context: Context, schedule: AndroidHostEventSchedule) {
        val alarmManager = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
        alarmManager.setAndAllowWhileIdle(
            AlarmManager.RTC_WAKEUP,
            schedule.nextFireAtMillis,
            pendingIntent(context, schedule.scheduleId),
        )
    }

    /** Cancels one previously registered Android system alarm. */
    private fun cancelAlarm(context: Context, scheduleId: String) {
        val alarmManager = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
        alarmManager.cancel(pendingIntent(context, scheduleId))
    }

    /** Builds the stable PendingIntent identity for one ToolPkg schedule. */
    private fun pendingIntent(context: Context, scheduleId: String): PendingIntent {
        val intent = Intent(context, HostEventScheduleReceiver::class.java)
            .setData(
                Uri.Builder()
                    .scheme(scheduleScheme)
                    .authority(scheduleAuthority)
                    .appendPath(scheduleId)
                    .build(),
            )
            .putExtra("scheduleId", scheduleId)
        return PendingIntent.getBroadcast(
            context,
            0,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
    }

    /** Computes the first future interval boundary after one alarm firing. */
    internal fun nextIntervalTime(
        scheduledAtMillis: Long,
        intervalMs: Long,
        nowMillis: Long,
    ): Long {
        if (scheduledAtMillis > nowMillis) {
            return Math.addExact(scheduledAtMillis, intervalMs)
        }
        val elapsed = nowMillis - scheduledAtMillis
        val steps = Math.addExact(elapsed / intervalMs, 1L)
        return Math.addExact(scheduledAtMillis, Math.multiplyExact(steps, intervalMs))
    }
}

class HostEventScheduleReceiver : BroadcastReceiver() {
    companion object {
        private const val LOG_TAG = "HostEventScheduleReceiver"
    }

    /** Consumes one alarm and transfers broadcast completion to an accepted background task. */
    override fun onReceive(context: Context, intent: Intent) {
        val pendingResult = goAsync()
        var submitted = false
        try {
            val scheduleId = intent.getStringExtra("scheduleId")
            if (scheduleId.isNullOrBlank()) {
                Log.w(LOG_TAG, "Host event broadcast without scheduleId; dropping")
                return
            }
            val schedule = try {
                AndroidHostEventScheduler.consumeFire(context, scheduleId)
            } catch (error: Exception) {
                val failure = when (error) {
                    is IllegalArgumentException -> "Invalid host event schedule"
                    is ArithmeticException -> "Host event interval overflow"
                    is JSONException -> "Corrupt host event schedule storage"
                    is IllegalStateException -> "Host event schedule storage failed"
                    else -> "Host event consumption failed"
                }
                Log.e(LOG_TAG, "$failure (scheduleId=$scheduleId)", error)
                return
            }
            if (schedule == null) {
                Log.w(LOG_TAG, "No stored host event schedule for scheduleId=$scheduleId; dropping broadcast")
                return
            }
            val retain: (UndeliveredHostEventFire) -> Unit = { fire ->
                AndroidHostEventScheduler.retainUndelivered(context, fire)
            }
            val outcome = HostEventHandoff.begin(
                schedule = schedule,
                firedAtMillis = System.currentTimeMillis(),
                startCore = { OperitCoreService.start(context) },
                submitBackground = { fired, atMillis ->
                    val runtimeHost = AndroidCoreRuntime.get(context)
                    runtimeHost.runBackground {
                        try {
                            val delivery = HostEventHandoff.completeDelivery(
                                schedule = fired,
                                firedAtMillis = atMillis,
                                emit = {
                                    val handle = runtimeHost.ensureRuntimeHandle()
                                    val response = JSONObject(OperitRuntimeNative.emitHostRuntimeEventSchedule(
                                        handle,
                                        fired.scheduleId,
                                        fired.nextFireAtMillis,
                                        atMillis,
                                    ))
                                    check(response.getBoolean("ok")) {
                                        response.optString("error", "Core rejected Android host event schedule")
                                    }
                                },
                                retain = retain,
                            )
                            if (delivery is HostEventHandoffOutcome.Undelivered) {
                                logUndelivered(delivery)
                            }
                        } finally {
                            pendingResult.finish()
                        }
                    }
                },
                retain = retain,
            )
            submitted = outcome is HostEventHandoffOutcome.Submitted
            if (outcome is HostEventHandoffOutcome.Undelivered) {
                logUndelivered(outcome)
            }
        } catch (error: Exception) {
            Log.e(LOG_TAG, "Host event schedule dispatch failed", error)
        } finally {
            if (!submitted) {
                pendingResult.finish()
            }
        }
    }

    private fun logUndelivered(outcome: HostEventHandoffOutcome.Undelivered) {
        val retention = if (outcome.retentionError == null) "failure retained" else "failure NOT retained"
        Log.e(
            LOG_TAG,
            "Host event delivery failed (scheduleId=${outcome.fire.scheduleId}, " +
                "stage=${outcome.fire.stage}); $retention; no retry",
            outcome.cause,
        )
        outcome.retentionError?.let { error ->
            Log.e(LOG_TAG, "Could not persist the undelivered host event ${outcome.fire.scheduleId}", error)
        }
    }
}
