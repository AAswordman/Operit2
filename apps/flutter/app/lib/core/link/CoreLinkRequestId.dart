// ignore_for_file: file_names

import 'dart:math';

final String _requestNamespace = _createRequestNamespace();
int _nextRequest = 0;

/// Allocates distinct request ids across concurrent calls, isolates, and restarts.
String nextCoreLinkRequestId({String prefix = 'flutter'}) =>
    '$prefix-$_requestNamespace-${_nextRequest++}';

/// Creates a 128-bit namespace once per isolate without relying on clock resolution.
String _createRequestNamespace() {
  final random = Random.secure();
  return List.generate(
    16,
    (_) => random.nextInt(256).toRadixString(16).padLeft(2, '0'),
  ).join();
}
