import 'dart:isolate';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/compose_dsl/execution_owner.dart';

void main() {
  test(
    'page owners stay unique when Dart restarts on a retained native runtime',
    () async {
      Future<List<String>> freshIsolateOwners() => Isolate.run(
        () => List.generate(24, (_) => ComposeDslExecutionOwner.allocate()),
      );
      final beforeRestart = await freshIsolateOwners();
      final afterRestart = await freshIsolateOwners();
      expect(beforeRestart.toSet(), hasLength(24));
      expect(afterRestart.toSet(), hasLength(24));
      expect(beforeRestart.toSet().intersection(afterRestart.toSet()), isEmpty);
      expect(beforeRestart.first.endsWith(':0'), isTrue);
      expect(afterRestart.first.endsWith(':0'), isTrue);
    },
  );
}
