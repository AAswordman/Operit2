// ignore_for_file: file_names

enum ClientLogLevel {
  verbose(2, 'V'),
  debug(3, 'D'),
  info(4, 'I'),
  warn(5, 'W'),
  error(6, 'E'),
  assert_(7, 'A');

  const ClientLogLevel(this.value, this.code);

  final int value;
  final String code;
}

/// Diagnostic depth shared with Rust AppLogger and the plugin console.
/// A maximum of N includes V1 through VN; severity remains independent.
enum VerboseLevel {
  /// Process and configuration lifecycle diagnostics.
  level1(1),

  /// User-operation stages and aggregate timings.
  level2(2),

  /// Service, provider, plugin, and tool invocation boundaries.
  level3(3),

  /// Component, session, and resource lifecycle diagnostics.
  level4(4),

  /// Routing, scheduling, persistence, and state propagation internals.
  level5(5),

  /// Per-item, per-chunk, repeated-loop, and payload diagnostics.
  level6(6);

  const VerboseLevel(this.value);

  final int value;
}
