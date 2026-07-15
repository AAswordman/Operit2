package app.operit

import android.app.AlarmManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.net.Uri
import org.json.JSONArray
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
        /** Decodes and validates one schedule command received from Core. */
        fun fromCommand(value: JSONObject, nowMillis: Long): AndroidHostEventSchedule {
            val scheduleId = value.getString("scheduleId")
            require(scheduleId.isNotBlank()) { "host event scheduleId is required" }
            val kind = value.getString("kind")
            require(kind == "timer" || kind == "interval") {
                "host event schedule kind must be timer or interval"
            }
            val delayMs = value.getLong("delayMs")
            require(delayMs > 0L) { "host event schedule delayMs must be positive" }
            val intervalMs = if (value.isNull("intervalMs")) null else value.getLong("intervalMs")
            if (kind == "timer") {
                require(intervalMs == null) { "timer schedule must not define intervalMs" }
            }
            if (kind == "interval") {
                require(intervalMs != null && intervalMs > 0L) {
                    "interval schedule intervalMs must be positive"
                }
            }
            return AndroidHostEventSchedule(
                scheduleId = scheduleId,
                kind = kind,
                delayMs = delayMs,
                intervalMs = intervalMs,
                nextFireAtMillis = Math.addExact(nowMillis, delayMs),
            )
        }

        /** Decodes one schedule persisted by the Android host. */
        fun fromStored(value: JSONObject): AndroidHostEventSchedule {
            return AndroidHostEventSchedule(
                scheduleId = value.getString("scheduleId"),
                kind = value.getString("kind"),
                delayMs = value.getLong("delayMs"),
                intervalMs = if (value.isNull("intervalMs")) null else value.getLong("intervalMs"),
                nextFireAtMillis = value.getLong("nextFireAtMillis"),
            )
        }
    }
}

object AndroidHostEventScheduler {
    private const val preferencesName = "operit_host_event_schedules"
    private const val schedulesKey = "schedules"
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
                val intervalMs = requireNotNull(schedule.intervalMs)
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
    private fun nextIntervalTime(
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
    /** Restores the persistent Core and forwards one system schedule firing. */
    override fun onReceive(context: Context, intent: Intent) {
        val pendingResult = goAsync()
        val scheduleId = requireNotNull(intent.getStringExtra("scheduleId")) {
            "host event scheduleId is required"
        }
        val schedule = AndroidHostEventScheduler.consumeFire(context, scheduleId)
        if (schedule == null) {
            pendingResult.finish()
            return
        }
        val firedAtMillis = System.currentTimeMillis()
        OperitCoreService.start(context)
        val runtimeHost = AndroidCoreRuntime.get(context)
        runtimeHost.runBackground {
            try {
                val handle = runtimeHost.ensureRuntimeHandle()
                val response = JSONObject(OperitRuntimeNative.emitHostRuntimeEventSchedule(
                    handle,
                    schedule.scheduleId,
                    schedule.nextFireAtMillis,
                    firedAtMillis,
                ))
                check(response.getBoolean("ok")) {
                    response.optString("error", "Core rejected Android host event schedule")
                }
            } catch (error: Throwable) {
                AndroidClientLogger.e(
                    context,
                    "HostEventScheduleReceiver",
                    "Host event schedule delivery failed: ${error.stackTraceToString()}",
                )
            } finally {
                pendingResult.finish()
            }
        }
    }
}
