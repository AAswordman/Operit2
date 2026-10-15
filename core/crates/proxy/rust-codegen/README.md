# operit-proxy-rust-codegen

Rust emitter for the core-app proxy scan model.

## Scope

- Consumes `CoreProxyScanOutput` data produced by `operit-proxy-scan`.
- Generates `generated_core_dispatch.rs` for `operit-proxy-local`.
- Generates the JSON proxy schema consumed by external clients.

## Boundaries

- Does not scan source roots.
- Does not generate Dart files.
- Does not implement runtime protocol conversion; runtime conversion lives in `operit-rslink-runtime`.

## Owned asynchronous calls

A method on a shared `Arc<Mutex<T>>` object must return
`impl Future<Output = T> + 'static` for asynchronous dispatch. Its synchronous
body captures only the operation's required data and service handles. The
emitter decodes arguments and creates that future under the receiver lock,
then drops the guard before polling. Neither the receiver nor request argument
borrows can escape into the response future; Rust checks this contract.

Implicit receiver cloning is forbidden at generation time. Capture methods can
mutate the canonical receiver, and cancellation drops the owned future normally.
The same contract releases Application and resolved-holder guards before polling.
Ordinary borrowing `async fn` methods on those async-mutex owners retain their
existing serialized semantics; converting one requires an explicit ownership
review of that method. Wire method names, arguments, results, and Dart Futures
remain unchanged.

The codegen regression fixture compiles the generated dispatcher with a
non-Clone receiver and tests reentry, canonical mutation, cancellation, malformed
arguments, and business errors.
