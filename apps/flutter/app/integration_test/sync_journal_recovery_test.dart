import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:operit2/core/bridge/PlatformCoreProxy.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';

/// Uses the real native Host and application API against a cold persisted journal.
void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  testWidgets(
    'cold journal pages and legacy receive migration survive native calls',
    (tester) async {
      final support = await getApplicationSupportDirectory();
      final parent = await Directory(
        '${support.path}/sync-journal-recovery',
      ).create(recursive: true);
      final root = await parent.createTemp('run-');
      final sync = await Directory(
        '${root.path}/runtime/sync/operations',
      ).create(recursive: true);
      const origin = 'ios-journal-probe';
      const domain = 'journal_probe';
      const count = 4096;
      const queued = 16;
      final fields = <String, int>{
        for (var index = 0; index < 128; index++) 'field_$index': index,
      };
      Map<String, Object?> operation(int sequence) => {
        'opId': '$origin:$sequence',
        'originDeviceId': origin,
        'sequence': sequence,
        'domain': domain,
        'entityType': 'entry',
        'entityId': 'entry-$sequence',
        'operation': 'upsert',
        'semantics': 'entity_state',
        'payload': {'data': fields},
        'createdAt': sequence,
        'schemaVersion': 1,
      };
      final journal = File('${sync.path}/$origin.jsonl');
      final output = journal.openWrite();
      for (var sequence = 1; sequence <= count; sequence++) {
        output.writeln(jsonEncode(operation(sequence)));
        if (sequence % 256 == 0) await output.flush();
      }
      await output.close();
      expect(await journal.length(), greaterThan(8 * 1024 * 1024));
      await File(
        '${sync.parent.path}/devices.json',
      ).writeAsString(jsonEncode([origin]), flush: true);
      final ledger = File('${sync.parent.path}/receive_state.json');
      await ledger.writeAsString(
        jsonEncode({
          'receipt': {
            'ranges': {
              origin: [
                [count + 1, count + queued],
              ],
            },
          },
          'pending': {
            for (
              var sequence = count + 1;
              sequence <= count + queued;
              sequence++
            )
              '$origin:$sequence': operation(sequence),
          },
          'fingerprints': <String, String>{},
        }),
        flush: true,
      );
      await platformCoreProxy.setRuntimeStorageRoots(
        '${root.path}/runtime',
        '${root.path}/workspace',
      );
      const clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
      final clock = Map<String, Object?>.from(
        await clients.application.syncClock() as Map,
      );
      expect(clock['pending'], queued);
      expect(
        (jsonDecode(await ledger.readAsString()) as Map)['formatVersion'],
        2,
      );
      expect(
        await Directory('${sync.parent.path}/pending').list().length,
        queued,
      );
      for (final sequence in [1, 2, count + 1, count + 2]) {
        final page = Map<String, Object?>.from(
          await clients.application.syncOperationsSince(
                clock: {
                  'ranges': {
                    if (sequence > 1)
                      origin: [
                        [1, sequence - 1],
                      ],
                  },
                },
                domains: [domain],
                limit: 1,
              )
              as Map,
        );
        expect(page['errors'], isEmpty);
        final operations = page['operations'] as List;
        expect(operations, hasLength(1));
        expect((operations.single as Map)['opId'], '$origin:$sequence');
      }
      // ignore: avoid_print
      print('SYNC_JOURNAL_NATIVE_OK records=$count pending=$queued');
    },
  );
}
