package app.operit

import org.json.JSONException
import org.json.JSONObject

/**
 * Outcome of decoding one raw notification-activation extra.
 *
 * [Accepted] means the navigation target is well-formed at the Kotlin format layer.
 * [Rejected] means the activation is discarded with a WARN record. **Rejecting an
 * activation never rejects the launch itself** — the host keeps starting; the user
 * action that opened the app stays valid.
 */
sealed interface NotificationActivationDecode {
    data class Accepted(val activation: Map<String, String>) : NotificationActivationDecode

    data class Rejected(val reason: String) : NotificationActivationDecode
}

/**
 * Pure decoder for [MainActivity.NOTIFICATION_ACTIVATION_EXTRA] payloads, split out
 * of the activity so the four behaviours the upstream review asks to verify can be
 * asserted on the JVM: malformed JSON, unknown type, missing chatId and empty chatId.
 *
 * Layering (deliberate): this layer owns the format. An empty chatId is **accepted**
 * here — the key exists with the right type — and rejected by the Dart semantic
 * layer (`chatId.isEmpty` -> StateError). That is a state-machine rejection, not a
 * launch crash, so the two cases stay separate.
 *
 * Warnings go through the injected [warn] sink; the raw extra is quoted truncated
 * so a maliciously large payload cannot flood logcat.
 */
object NotificationActivationCodec {
    /** Max characters of the raw extra quoted in one warning. */
    const val RAW_EXTRA_LOG_LIMIT: Int = 128

    fun decode(
        encodedActivation: String,
        warn: (message: String, error: Throwable?) -> Unit,
    ): NotificationActivationDecode {
        val quotedRaw =
            "raw extra (truncated to $RAW_EXTRA_LOG_LIMIT chars): " +
                encodedActivation.take(RAW_EXTRA_LOG_LIMIT)

        val json = try {
            JSONObject(encodedActivation)
        } catch (error: JSONException) {
            warn("Rejected malformed notification activation payload; $quotedRaw", error)
            return NotificationActivationDecode.Rejected("malformed payload")
        }

        val type = try {
            json.getString("type")
        } catch (error: JSONException) {
            warn("Rejected notification activation without a type field; $quotedRaw", error)
            return NotificationActivationDecode.Rejected("missing type")
        }

        val activation: Map<String, String> = when (type) {
            "open_application" -> mapOf("type" to type)
            "open_chat" -> try {
                mapOf("type" to type, "chatId" to json.getString("chatId"))
            } catch (error: JSONException) {
                warn("Rejected open_chat notification activation without a chatId field; $quotedRaw", error)
                return NotificationActivationDecode.Rejected("missing chatId")
            }
            else -> {
                warn("Rejected notification activation with unsupported type: $type; $quotedRaw", null)
                return NotificationActivationDecode.Rejected("unsupported type")
            }
        }

        return NotificationActivationDecode.Accepted(activation)
    }
}
