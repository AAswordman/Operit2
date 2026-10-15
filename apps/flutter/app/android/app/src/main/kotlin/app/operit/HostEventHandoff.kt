package app.operit

import org.json.JSONObject

/** The delivery step that failed after a schedule firing was consumed. */
internal enum class HostEventHandoffStage {
    START_CORE,
    SUBMIT_BACKGROUND,
    DELIVERY,
}

/** A diagnostic record of an undelivered firing; recording it does not schedule a retry. */
internal data class UndeliveredHostEventFire(
    val scheduleId: String,
    val firedAtMillis: Long,
    val stage: HostEventHandoffStage,
    val reason: String,
) {
    fun toJson(): JSONObject = JSONObject(toMap())

    fun toMap(): Map<String, Any> = mapOf(
        "scheduleId" to scheduleId,
        "firedAtMillis" to firedAtMillis,
        "stage" to stage.name,
        "reason" to reason,
    )

    companion object {
        fun fromJson(value: JSONObject): UndeliveredHostEventFire {
            val scheduleId = value.getString("scheduleId")
            val firedAtMillis = value.getLong("firedAtMillis")
            require(scheduleId.isNotBlank()) { "undelivered host event scheduleId is required" }
            require(firedAtMillis >= 0L) { "undelivered host event firedAtMillis must not be negative" }
            return UndeliveredHostEventFire(
                scheduleId = scheduleId,
                firedAtMillis = firedAtMillis,
                stage = HostEventHandoffStage.valueOf(value.getString("stage")),
                reason = value.getString("reason"),
            )
        }
    }
}

internal sealed class HostEventHandoffOutcome {
    /** Accepted by the executor; delivery has not necessarily completed. */
    object Submitted : HostEventHandoffOutcome()

    object Delivered : HostEventHandoffOutcome()

    /** Keeps the delivery error and any separate diagnostic storage error. */
    data class Undelivered(
        val fire: UndeliveredHostEventFire,
        val cause: Throwable,
        val retentionError: Throwable?,
    ) : HostEventHandoffOutcome()
}

/** Separates delivery failures from failures to persist their diagnostic records. */
internal object HostEventHandoff {
    fun begin(
        schedule: AndroidHostEventSchedule,
        firedAtMillis: Long,
        startCore: () -> Unit,
        submitBackground: (AndroidHostEventSchedule, Long) -> Unit,
        retain: (UndeliveredHostEventFire) -> Unit,
    ): HostEventHandoffOutcome {
        try {
            startCore()
        } catch (error: Throwable) {
            return retainFailure(retain, schedule, firedAtMillis, HostEventHandoffStage.START_CORE, error)
        }
        try {
            submitBackground(schedule, firedAtMillis)
        } catch (error: Throwable) {
            return retainFailure(retain, schedule, firedAtMillis, HostEventHandoffStage.SUBMIT_BACKGROUND, error)
        }
        return HostEventHandoffOutcome.Submitted
    }

    fun completeDelivery(
        schedule: AndroidHostEventSchedule,
        firedAtMillis: Long,
        emit: () -> Unit,
        retain: (UndeliveredHostEventFire) -> Unit,
    ): HostEventHandoffOutcome {
        try {
            emit()
        } catch (error: Throwable) {
            return retainFailure(retain, schedule, firedAtMillis, HostEventHandoffStage.DELIVERY, error)
        }
        return HostEventHandoffOutcome.Delivered
    }

    private fun retainFailure(
        retain: (UndeliveredHostEventFire) -> Unit,
        schedule: AndroidHostEventSchedule,
        firedAtMillis: Long,
        stage: HostEventHandoffStage,
        error: Throwable,
    ): HostEventHandoffOutcome.Undelivered {
        val fire = UndeliveredHostEventFire(schedule.scheduleId, firedAtMillis, stage, error.toString())
        // Catch Throwable only after consumption, preserving both failures in the result.
        val retentionError = try {
            retain(fire)
            null
        } catch (storageError: Throwable) {
            storageError
        }
        return HostEventHandoffOutcome.Undelivered(fire, error, retentionError)
    }
}
