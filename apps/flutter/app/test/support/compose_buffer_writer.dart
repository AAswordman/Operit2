import 'dart:typed_data';
import 'package:operit2/core/compose/compose_program_buffer.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;

/// Writes fixture data into the sole production binary transaction format.
core.ToolPkgComposeDslTransaction composeBufferTransaction({
  required bool reset,
  required int revision,
  required ToolPkgComposeDslReference root,
  required List<ToolPkgComposeDslProgram> programs,
  required List<ToolPkgComposeDslInstance> instances,
  required List<String> removed,
}) {
  final writer = _FixtureBufferWriter();
  final update = writer.write([
    reset,
    revision,
    root.instanceId,
    root.node,
    [
      for (final program in programs)
        {
          'id': program.id,
          'root': program.root,
          'nodes': [
            for (final node in program.nodes)
              {
                'type': node.nodeType,
                'props': node.props,
                'children': node.children,
                'explicitChildren': node.explicitChildren,
              },
          ],
        },
    ],
    [
      for (final instance in instances)
        [
          instance.id,
          instance.programId,
          instance.values,
          [
            for (final patch in instance.patches)
              [patch.node, patch.field, patch.value],
          ],
        ],
    ],
    removed,
  ]);
  final navigation = writer.write(const []);
  writer.view.setUint32(0, 0x31425043, Endian.little);
  writer.view.setUint32(4, 1, Endian.little);
  writer.view.setUint32(8, writer.used, Endian.little);
  writer.view.setUint32(12, 1, Endian.little);
  writer.view.setUint32(16, update, Endian.little);
  writer.view.setUint32(24, navigation, Endian.little);
  return core.ToolPkgComposeDslTransaction(
    buffer: Uint8List.sublistView(writer.bytes, 0, writer.used),
  );
}

/// Produces independent reference fixtures rather than reusing the production reader.
class _FixtureBufferWriter {
  Uint8List bytes = Uint8List(4096);
  int used = 32;

  /// Views the current allocation after any construction-time growth.
  ByteData get view => ByteData.sublistView(bytes);

  /// Reserves aligned fixture storage.
  int allocate(int length) {
    final start = used;
    used = (used + length + 7) & ~7;
    if (used > bytes.length) {
      final next = Uint8List(used * 2);
      next.setRange(0, bytes.length, bytes);
      bytes = next;
    }
    return start;
  }

  /// Writes one logical value with fixed descriptors and indexed children.
  int write(Object? value) {
    final offset = allocate(16);
    if (value == null) return offset;
    if (value is bool) {
      view.setUint32(offset, value ? 2 : 1, Endian.little);
      return offset;
    }
    if (value is num) {
      view.setUint32(offset, 3, Endian.little);
      view.setFloat64(offset + 8, value.toDouble(), Endian.little);
      return offset;
    }
    if (value is String) {
      final data = allocate(value.length * 2);
      view.setUint32(offset, 4, Endian.little);
      view.setUint32(offset + 4, value.length, Endian.little);
      view.setUint32(offset + 8, data, Endian.little);
      for (var i = 0; i < value.length; i++) {
        view.setUint16(data + i * 2, value.codeUnitAt(i), Endian.little);
      }
      return offset;
    }
    final array = value is List;
    if (!array && value is! Map) {
      throw ArgumentError('Unsupported fixture value');
    }
    final addresses = <int>[];
    if (array) {
      for (final item in value) {
        addresses.add(write(item));
      }
    } else {
      for (final entry in (value as Map).entries) {
        addresses.add(write(entry.key));
        addresses.add(write(entry.value));
      }
    }
    final data = allocate(addresses.length * 4);
    view.setUint32(offset, array ? 5 : 6, Endian.little);
    view.setUint32(
      offset + 4,
      array ? addresses.length : addresses.length ~/ 2,
      Endian.little,
    );
    view.setUint32(offset + 8, data, Endian.little);
    for (var i = 0; i < addresses.length; i++) {
      view.setUint32(data + i * 4, addresses[i], Endian.little);
    }
    return offset;
  }
}
