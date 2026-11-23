package app.operit

import android.app.Activity
import org.junit.Assert.*
import org.junit.Test

/** Verifies picker ownership across main and floating Flutter engines. */
class DocumentPickerRequestTest {
    private class Engine(val picker: DocumentPickerRequest) {
        var code = 0
        val results = mutableListOf<Int>()
        val errors = mutableListOf<String>()

        /** Records one engine's launch and method-channel completions. */
        fun launch(start: (Int) -> Unit = { code = it }) {
            picker.launch(
                owner = this,
                start = start,
                complete = { resultCode, _ -> results.add(resultCode) },
                fail = { errorCode, _ -> errors.add(errorCode) },
            )
        }

        /** Delivers Android's result through the Activity's shared request owner. */
        fun finish(resultCode: Int = Activity.RESULT_OK) {
            assertTrue(picker.onActivityResult(code, resultCode, null))
        }
    }

    /** Returns a floating selection to its engine and permits a subsequent main selection. */
    @Test
    fun floatingResultDoesNotGetConsumedByMainEngine() {
        val picker = DocumentPickerRequest()
        val main = Engine(picker)
        val floating = Engine(picker)
        floating.launch()
        assertFalse(picker.onActivityResult(floating.code + 1, Activity.RESULT_OK, null))
        floating.finish()
        assertEquals(listOf(Activity.RESULT_OK), floating.results)
        assertTrue(main.results.isEmpty())
        main.launch()
        main.finish()
        assertEquals(listOf(Activity.RESULT_OK), main.results)
    }

    /** Rejects another engine without replacing the currently open picker. */
    @Test
    fun concurrentEngineCannotStealPendingSelection() {
        val picker = DocumentPickerRequest()
        val first = Engine(picker)
        val second = Engine(picker)
        first.launch()
        second.launch()
        assertEquals(listOf("PICK_IN_PROGRESS"), second.errors)
        assertEquals(0, second.code)
        first.finish()
        assertEquals(listOf(Activity.RESULT_OK), first.results)
        second.launch()
        second.finish()
        assertEquals(listOf(Activity.RESULT_OK), second.results)
    }

    /** Cancellation clears the request so the same engine can choose again immediately. */
    @Test
    fun cancellationAllowsImmediateRetry() {
        val engine = Engine(DocumentPickerRequest())
        engine.launch()
        engine.finish(Activity.RESULT_CANCELED)
        engine.launch()
        engine.finish()
        assertEquals(listOf(Activity.RESULT_CANCELED, Activity.RESULT_OK), engine.results)
        assertTrue(engine.errors.isEmpty())
    }

    /** Launch errors release the picker instead of poisoning all later selections. */
    @Test
    fun launchFailureAllowsRetry() {
        val engine = Engine(DocumentPickerRequest())
        engine.launch { throw IllegalStateException("No image provider") }
        assertEquals(listOf("PICK_FAILED"), engine.errors)
        engine.launch()
        engine.finish()
        assertEquals(listOf(Activity.RESULT_OK), engine.results)
    }

    /** Destroying an unrelated engine leaves the active engine's completion intact. */
    @Test
    fun releasingAnotherEngineDoesNotCancelSelection() {
        val picker = DocumentPickerRequest()
        val engine = Engine(picker)
        engine.launch()
        picker.release(Any())
        engine.finish()
        assertEquals(listOf(Activity.RESULT_OK), engine.results)
        assertTrue(engine.errors.isEmpty())
    }

    /** A destroyed engine's late result cannot complete a different engine's selection. */
    @Test
    fun engineDestructionReleasesCallbackAndDrainsItsActivityResult() {
        val picker = DocumentPickerRequest()
        val first = Engine(picker)
        val second = Engine(picker)
        first.launch()
        picker.release(first)
        picker.release(first)
        assertEquals(listOf("CHANNEL_CLOSED"), first.errors)
        second.launch()
        assertEquals(listOf("PICK_IN_PROGRESS"), second.errors)
        first.finish()
        assertTrue(first.results.isEmpty())
        second.launch()
        second.finish()
        assertEquals(listOf(Activity.RESULT_OK), second.results)
    }
}
