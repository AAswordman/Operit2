# Chat cancellation and provider locks

Cancellation must not wait for a provider request before it can interrupt that
request. The existing per-send cancellation token now covers runtime preparation
and the complete response worker, including provider lock acquisition, rate and
concurrency waits, plugin calls, tool rounds, and response collection. Cancelling
either the send or its execution drops the pending worker future. Its provider
guard is released before `cancelAllStreaming` acquires that provider to stop
background transport work and settle token usage.

The main, floating, and detached chat cancellation entry points capture owned
futures. They reserve the current generation synchronously under the existing
runtime mutex, then release the chat Holder before awaiting provider cancellation,
usage settlement, and partial-response persistence. Native plugin cancellation
uses the same capture-and-await sequence and retains its execution identity check.
Partial-response persistence and receipt delivery run synchronously inside the
reserved cancellation before its execution lease and busy state are released.
Repeated cancellation cannot settle the receipt early, and a removed or replaced
runtime cannot receive an old cancellation's partial response.

Request concurrency uses asynchronous semaphore guards. Waiting does not block a
Host scheduler thread; dropping a waiting or executing request cannot leak a
permit. The guard covers the existing `send_message` admission interval. This
change does not redefine that interval as the lifetime of the returned stream.

Plugin-provider streaming and final usage share one update function. It acquires
the usage mutex once, merges partial counters, then unlocks before text or error
callbacks. An `if let` condition must never acquire the same mutex again in its
body.

Regression coverage lives in the provider cancellation tests, request semaphore
tests, plugin-provider usage tests, and the native chat lifecycle fixture. It covers cancellation while waiting for a
provider, dropping an in-flight request with a provider guard and permit, returning
permits after dropping waiters, partial streaming usage updates, duplicate
cancellation, and releasing execution ownership after successful or failed
cancellation settlement.
