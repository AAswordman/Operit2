package app.operit

import android.content.Intent

/** Owns the Activity's single document picker across all Flutter engines. */
internal class DocumentPickerRequest {
    private companion object {
        private const val REQUEST_CODE = 46091
    }

    private class Request(
        var owner: Any?,
        var complete: ((Int, Intent?) -> Unit)?,
        var fail: ((String, String?) -> Unit)?,
    )

    private var pending: Request? = null

    /** Starts a picker and keeps its completion bound to the requesting engine. */
    fun launch(
        owner: Any,
        start: (Int) -> Unit,
        complete: (Int, Intent?) -> Unit,
        fail: (String, String?) -> Unit,
    ) {
        if (pending != null) {
            fail("PICK_IN_PROGRESS", "A document picker is already open")
            return
        }
        val request = Request(owner, complete, fail)
        pending = request
        try {
            start(REQUEST_CODE)
        } catch (error: Exception) {
            pending = null
            fail("PICK_FAILED", error.message)
        }
    }

    /** Clears ownership before returning selection or cancellation to its engine. */
    fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?): Boolean {
        if (requestCode != REQUEST_CODE) return false
        val request = pending
        pending = null
        request?.complete?.invoke(resultCode, data)
        return true
    }

    /** Releases an engine while retaining the open Activity's result slot until it returns. */
    fun release(owner: Any) {
        val request = pending ?: return
        if (request.owner !== owner) return
        val fail = request.fail
        request.owner = null
        request.complete = null
        request.fail = null
        fail?.invoke("CHANNEL_CLOSED", "Document input channel closed")
    }
}
