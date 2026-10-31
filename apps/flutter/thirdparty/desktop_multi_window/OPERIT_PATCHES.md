# Operit patches

Vendored from desktop_multi_window 0.3.1 in the local pub cache. Upstream Apache-2.0 license is preserved in LICENSE.

- Add WindowController.close() and native window_close handlers on Windows, macOS, and GTK. This allows the owner to destroy a secondary engine when Dart startup fails before its message receiver is installed. Replies precede deferred window destruction.

- Allow explicit keyboard focus on borderless macOS CustomWindow instances. The optional floatingWindowFocusable property defaults to nil, preserving ordinary window behavior.

- Keep hidden-at-launch macOS windows hidden until their content is prepared.

- Add a transparent configuration hint for GTK, selecting RGBA visuals for the native window and Flutter view before realization.

- Start the macOS child engine explicitly when hidden at launch, after channel registration. Flutter normally starts Dart from `viewWillAppear`; waiting for Dart readiness before ever displaying a window otherwise deadlocks.
