# Flutter Core FFI transport (ABI v3)

Native Flutter Core calls and streams use ABI version `3` exclusively. ABI v1 and v2,
the synchronous C Core-call exports, `OperitByteBuffer`, and the native watch
polling queue have been removed. Dart rejects any descriptor whose `version` is
not `3` before binding session pointers or function addresses.

## Connection and platform ownership

Each Flutter isolate lazily calls `connectCoreFfi` on the `operit/runtime`
platform channel. The host initializes its existing runtime and returns the JSON
descriptor from `operit_flutter_bridge_ffi_connect`:

| Field | Contract |
| --- | --- |
| `version` | Integer `3` |
| `session` | Retained connection pointer |
| `attach` | Registers `NativeApi.postCObject` and the Dart native port |
| `allocate`, `free` | Allocate and release an unsubmitted request allocation |
| `submit` | Consumes a request allocation and retains its binary handles |
| `release` | Closes and releases the connection |
| `bufferAllocate`, `bufferData` | Allocate and fill an unsealed binary buffer |
| `bufferSeal` | Publishes the fully initialized buffer as immutable |
| `bufferRetain`, `bufferRelease` | Acquire and release independent shared ownership |

Pointer and function address fields are decimal strings. Dart parses them with
`BigInt`, validates the native pointer width, and preserves all address bits
before `Pointer.fromAddress`. Platform hosts keep the library mapped until
process exit so outstanding FFI calls and VM finalizers reference live code.

Dart calls `attach(session, NativeApi.postCObject, SendPort.nativePort)` once.
The host adapter requires Dart native API major version `2`; its C object layout
follows `dart_native_api.h`. The Dart native API version and the Core ABI version
are separate checks.

The platform channel owns connection bootstrap, storage configuration,
application lifecycle, notifications, and system capabilities. Android JNI owns
Java host APIs and connection bootstrap. Ordinary Core calls and streams use the
FFI session. Browser Core calls use the JS/WASM transport described below.

## Request allocation and submission

The native submit signature is:

```text
submit(session, operation, requestId, metadataPointer, metadataLength,
       binaryHandlesPointer, binaryHandleCount)
```

Dart obtains one allocation from `allocate(totalLength)` containing:

```text
MessagePack metadata | padding to native pointer alignment | binary handle table
```

`totalLength` is the aligned metadata length plus the handle count multiplied by
the native pointer size. The `metadataLength` argument excludes padding and the
handle table. UTF-8 control operations use the same allocation layout.

`submit` synchronously takes ownership of that entire allocation and retains
all binary handles before returning. It then schedules the request through the
runtime's installed Host task scheduler. Dart keeps the source views alive until
submission returns and calls `free(pointer, totalLength)` only for an allocation
that has not been submitted.

| Operation | Number | Input metadata |
| --- | ---: | --- |
| call | 0 | `[requestId, target, methodName, args]` |
| pushOpen | 1 | `[requestId, target, methodName, args]` |
| pushItem | 2 | `[pushId, sequence, args]` |
| pushClose | 3 | UTF-8 push ID |
| watchSnapshot | 4 | `[requestId, target, propertyName, args]` |
| watchStream | 5 | `[subscriptionId, requestId, target, propertyName, args]` |
| closeWatchStream | 6 | `[subscriptionId, watchOpenFfiRequestId]` |

The integer FFI `requestId` correlates port replies within one connection. The
request ID inside a MessagePack request is the separate Link request ID.
Every submitted operation receives one asynchronous acknowledgement or result;
independent replies may arrive in any order. Control calls can execute while
another call awaits host interaction.

## Shared binary ownership

MessagePack metadata and binary contents are kept separate. Ext type `0x42`
contains three big-endian unsigned 64-bit values: buffer-table index, byte offset,
and byte length. The extension has 24 bytes of content and 27 bytes including its
MessagePack prefix. Decoders validate its type, size, index, range, and overflow
against the current message's retained buffer table.

Rust binary values use `CoreBytes` (`bytes::Bytes`), so retaining and slicing
shared allocations does not copy their contents. Ordinary Dart `Uint8List`
values require one initial copy into host-allocated storage, followed by
`bufferSeal`. `FfiCoreTransport.shareBytes` exposes a reusable read-only shared
view. A shared view returned by Rust can also be submitted again directly.
Dart validates that a submitted shared view belongs to the same host domain.

Sealed storage is immutable. Mutation requires an explicitly owned writable
copy. Handles are trusted live pointers provided by the same in-process host;
they are not network addresses or untrusted serialized pointers.

## Asynchronous replies and events

`NativeApi.postCObject` always delivers a batch of independently owned frames, including singleton batches:

```text
[
  [controlFrame, handle, receipt, address, length, ...],
  [controlFrame, handle, receipt, address, length, ...]
]
```

`controlFrame` is external typed data whose allocation is retained by a native
finalizer. Its first nine bytes are a one-byte frame kind and an eight-byte
little-endian signed FFI request ID. The remaining bytes are MessagePack payload.

| Kind | Payload |
| --- | --- |
| 0 | Core result or acknowledgement |
| 1 | Watch frame, using the original watch-open FFI request ID |
| 2 | Core error for that watch |
| 3 | Empty payload marking watch completion |

Core success is `[0, value]`. Core failure is
`[1, code, message, details, location, backtrace]`. A watch frame is
`[subscriptionId, [requestId, target, propertyName, kind, value]]`.

Each binary entry supplies a retained native handle, a one-byte external typed
data receipt, the allocation address, and its length. The receipt's native
finalizer releases the delivered handle. Dart keeps the envelope alive while
receiving it, retains newly adopted views, and binds their native finalizers.
Views are read-only. A weak identity table reuses still-live views with the same
address and length without keeping dead buffers alive. New views account for
their actual byte length in VM external memory.

Rejected port messages immediately release their native ownership. Delivered
receipts and views release ownership through VM finalization. Closing a connection
does not invalidate shared views already returned to Dart.

Watch frames may precede the open acknowledgement; Dart installs the receiver
before submission. Events from one watch retain source order. Source errors and
panics reach the stream listener. Cancellation releases the connection-owned
watch. Push items carry increasing sequence numbers; duplicate push IDs and
invalid sequences are rejected.

## Lifecycle and browser boundary

Runtime handles use `Arc` ownership. Each session retains the host-owned runtime,
owns its watch and push registrations, and has a dedicated Dart receive port.
`release` closes the connection and cancels its subscriptions; in-flight tasks
retain their own references. Dart registers a `NativeFinalizer` and supports
explicit closure. Closing one connection does not cancel another's watches.

Web uses `WebWasmCoreProxy`, JS interop, the Runtime Worker, and Rust WASM. It
shares the Core request/result/watch protocol, while `CoreLinkMessage.toBytes()`
materializes portable inline MessagePack binary values. Native shared pointers
and ABI-v3 port envelopes remain within the native host boundary.

## Host and bridge ownership

| Module | Responsibility |
| --- | --- |
| `BridgeRuntime` | One local Core lifecycle, permission requester, and runtime-owned scheduler |
| `BridgeTransport` | Shared push registration, sequence validation, browser watch opening, cancellation, and registration generations |
| `BridgeCodec` | Compact MessagePack requests/results, shared binary capture, and event frames |
| `FlutterHostAdapters` | Browser automation, browser sessions, web visits, and Compose owner boundaries |
| `FlutterOwnerCapabilities` | Typed media, inference, system, and terminal host adapters |
| `PlatformRuntimeFactory` | Common startup pipeline without target selection |
| `platform_runtime/*` | Platform host assembly selected at the host boundary |
| `PlatformRuntimeExecution` | Native lifecycle/owner-event scheduling and browser Promise execution |
| `PlatformRuntimeAbi` | ABI module selection |
| `BridgeExports` | Runtime lifecycle, host exports, and browser ABI |
| `FfiTransport` | Exclusive native Core ABI-v3 session, dispatch, and port delivery |
| `AndroidJni` | Android host integration and connection bootstrap |

Platform-specific dependencies and conditional compilation stay at host or ABI
dispatch boundaries. Shared runtime and stream code do not select platforms.
Each connection uses the scheduler installed on its Core host manager.

## Validation

Architecture contracts can be checked without building the application:

```powershell
node --test tools/tests/flutter_bridge_architecture.test.mjs tools/tests/android_runtime_contracts.test.mjs tools/tests/web_runtime_events.test.mjs
```

From `apps/flutter/app`, use FVM for Dart and Flutter checks:

```powershell
fvm flutter test --no-pub test/core/bridge/ffi_core_abi_version_test.dart test/core/bridge/ffi_core_address_test.dart test/core/link/CoreLinkCodec_test.dart
```

`test/core/bridge/ffi_core_transport_test.dart` exercises real FFI pointers and
VM port delivery, including shared-buffer identity, finalization, concurrent
requests, early events, cancellation, errors, and connection ownership. Running
that suite builds its native Rust test fixture. These checks do not replace
integration testing of the production library with each platform host.

## Stream ownership and scheduling

Each native or browser connection owns one stream coordinator. Multiple push and
watch IDs share its command channel (capacity 64). Each push preserves FIFO with
a bounded queue (capacity 16), and watches share an outbox (capacity 64). Source
opening, delivery, cancellation and ID reuse are owned by this coordinator.
Independent streams progress while another source is waiting or backpressured.

A close identifies both the subscription ID and its opening request generation.
It cancels pending source creation and blocked delivery immediately. A stale close
cannot remove a newer registration with the same ID. Native delivery batches
multiple subscriptions in one VM message; binary attachment tables belong to
individual frames. Browser delivery carries the same nine-byte kind/correlation
header, with explicit watch value, error and close frames.

The Host owns process-wide worker allocation. Cooperative I/O and timers remain
separate from synchronous blocking work. Native HTTP and WebSocket futures share
four network workers. Long-lived blocking OS listeners use admitted service
capacity; a multi-pipe process reserves its complete pipeline atomically.
Java owner callbacks, script execution, compilation and COM operations preserve
independent execution capacity when synchronous callers can wait for them.

Desktop and mobile worker limits are process-wide and initialized when first used:

| Host capacity | Worker limit |
| --- | ---: |
| Asynchronous coordination | 16 |
| Finite blocking work | 16 |
| HTTP and WebSocket I/O | 4 |
| JavaScript execution states | 4 |
| JavaScript compilation | 1 |
| Persistent blocking OS services | 16 |
| Android Java owner callbacks | 8 |
| Isolated platform operations | 1 |

These are allocation limits for application-owned workers, not a count of all
threads inside Flutter, the JVM or third-party libraries. Device embeddings use
a process-shared asynchronous worker and a separate blocking worker with their
own stack and I/O policy. Waiting futures retain their owner thread; new
subscriptions and HTTP requests do not allocate new operating-system threads.
