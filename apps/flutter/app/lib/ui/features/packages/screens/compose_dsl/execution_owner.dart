import 'dart:math';

/// Identifies a page lease across isolates sharing the same retained native runtime.
class ComposeDslExecutionOwner {
  static final String _isolateId =
      '${DateTime.now().microsecondsSinceEpoch.toRadixString(16)}-'
      '${Random.secure().nextInt(1 << 32).toRadixString(16)}-'
      '${Random.secure().nextInt(1 << 32).toRadixString(16)}';
  static int _next = 0;

  /// Hot reload keeps the namespace; hot restart must never reuse an old lease key.
  static String allocate() => '$_isolateId:${_next++}';
}
