package app.operit

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * JVM unit tests for the notification-activation decoder — the four behaviour
 * cases the upstream review of PR #210 asked to verify (see #196 card 3):
 * malformed JSON, unknown type, missing chatId and empty chatId.
 *
 * "Rejected" means: a WARN was recorded AND no navigation target was produced.
 * "Accepted" means a navigation target is handed to the wiring. Rejection never
 * terminates the launch itself — that part is the activity wiring and is asserted
 * by reading the code, not here.
 *
 * What still needs a device (or the Dart layer) is not covered here: the empty
 * chatId case is accepted at this layer by design and errors in the Dart state
 * machine (`NotificationActivationService.dart`).
 */
class NotificationActivationCodecTest {
    private val warnings = mutableListOf<Pair<String, Throwable?>>()
    private val warn: (String, Throwable?) -> Unit = { message, error ->
        warnings.add(message to error)
    }

    private fun decode(payload: String): NotificationActivationDecode =
        NotificationActivationCodec.decode(payload, warn)

    private fun assertRejected(
        decoded: NotificationActivationDecode,
        expectedReason: String,
    ) {
        assertTrue("expected Rejected but was $decoded", decoded is NotificationActivationDecode.Rejected)
        assertEquals(expectedReason, (decoded as NotificationActivationDecode.Rejected).reason)
        assertEquals("a rejection must be observable in the log", 1, warnings.size)
        assertTrue(
            "the warning must quote the raw extra, truncated",
            warnings.single().first.contains("raw extra (truncated to"),
        )
    }

    @Test
    fun malformedJsonIsRejectedWithWarning() {
        val decoded = decode("{\"type\": \"open_chat\", ")

        assertRejected(decoded, "malformed payload")
    }

    @Test
    fun unknownTypeIsRejectedWithWarning() {
        val decoded = decode("{\"type\": \"teleport\", \"chatId\": \"c1\"}")

        assertRejected(decoded, "unsupported type")
        assertTrue(warnings.single().first.contains("teleport"))
    }

    @Test
    fun missingTypeIsRejectedWithWarning() {
        val decoded = decode("{\"chatId\": \"c1\"}")

        assertRejected(decoded, "missing type")
    }

    @Test
    fun missingChatIdIsRejectedWithWarning() {
        val decoded = decode("{\"type\": \"open_chat\"}")

        assertRejected(decoded, "missing chatId")
    }

    @Test
    fun emptyChatIdIsAcceptedForTheDartLayer() {
        // Key exists with the right type: the format layer lets it through; the
        // Dart semantic layer is where an empty chatId becomes a StateError.
        val decoded = decode("{\"type\": \"open_chat\", \"chatId\": \"\"}")

        assertTrue("expected Accepted but was $decoded", decoded is NotificationActivationDecode.Accepted)
        assertEquals(
            mapOf("type" to "open_chat", "chatId" to ""),
            (decoded as NotificationActivationDecode.Accepted).activation,
        )
        assertEquals("an acceptance must not log a warning", 0, warnings.size)
    }

    @Test
    fun openApplicationAndOpenChatAreAccepted() {
        val application = decode("{\"type\": \"open_application\"}")
        assertTrue(
            application is NotificationActivationDecode.Accepted &&
                application.activation == mapOf("type" to "open_application"),
        )

        val chat = decode("{\"type\": \"open_chat\", \"chatId\": \"chat-42\"}")
        assertTrue(
            chat is NotificationActivationDecode.Accepted &&
                chat.activation == mapOf("type" to "open_chat", "chatId" to "chat-42"),
        )
        assertEquals(0, warnings.size)
    }

    @Test
    fun oversizedRawExtraIsTruncatedInTheWarning() {
        // No closing quote/brace on purpose: malformed, with an oversized chatId.
        val payload = "{\"type\": \"open_chat\", \"chatId\": \"" + "x".repeat(400)

        val decoded = decode(payload) // missing closing brace -> malformed

        assertRejected(decoded, "malformed payload")
        val message = warnings.single().first
        assertTrue(
            "warning must cap the quoted payload, was ${message.length} chars",
            message.length < "raw extra (truncated to 128 chars): ".length +
                NotificationActivationCodec.RAW_EXTRA_LOG_LIMIT + 64,
        )
    }
}
