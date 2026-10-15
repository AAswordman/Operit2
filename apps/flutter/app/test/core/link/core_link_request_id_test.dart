import 'dart:isolate';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/link/CoreLinkRequestId.dart';

/// Allocates a burst of ids from the current isolate's namespace.
List<String> _allocateBatch() =>
    List.generate(2048, (_) => nextCoreLinkRequestId());

/// Verifies independent callers can share a runtime without reusing request ids.
void main() {
  test(
    'concurrent isolates and fresh isolates allocate distinct ids',
    () async {
      final batches = await Future.wait([
        Isolate.run(_allocateBatch),
        Isolate.run(_allocateBatch),
        Future.value(_allocateBatch()),
      ]);
      batches.add(await Isolate.run(_allocateBatch));
      final ids = batches.expand((batch) => batch).toList();

      expect(ids, hasLength(8192));
      expect(ids.toSet(), hasLength(ids.length));
    },
  );
}
