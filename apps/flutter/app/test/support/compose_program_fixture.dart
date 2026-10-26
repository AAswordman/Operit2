import 'compose_buffer_writer.dart';
import 'package:operit2/core/compose/compose_program_buffer.dart'
    as compose_buffer;
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;

/// Encodes a test source with the production dynamic-node parameter ABI.
compose_buffer.ToolPkgComposeDslInstance composeTestInstance({
  required String id,
  required String nodeType,
  required Map<String, Object?> props,
  List<String> children = const [],
  Map<String, List<String>> slots = const {},
}) {
  /// Encodes one logical child address without expanding its target.
  Object reference(String id) => {
    '__composeRef': [id, 0],
  };
  return compose_buffer.ToolPkgComposeDslInstance(
    id: id,
    programId: 'test:node',
    values: [
      {
        ...props,
        for (final entry in slots.entries)
          entry.key: entry.value.map(reference).toList(),
      },
      children.map(reference).toList(),
    ],
    patches: [
      compose_buffer.ToolPkgComposeDslPatch(
        node: 0,
        field: 'type',
        value: nodeType,
      ),
    ],
  );
}

/// Installs one shared fixture program and streams subsequent parameter replacements.
core.ToolPkgComposeDslTransaction composeTestTransaction({
  required bool reset,
  required int revision,
  required String rootInstanceId,
  required List<compose_buffer.ToolPkgComposeDslInstance> instances,
  List<String> removed = const [],
}) => composeBufferTransaction(
  reset: reset,
  revision: revision,
  root: compose_buffer.ToolPkgComposeDslReference(
    instanceId: rootInstanceId,
    node: 0,
  ),
  programs: reset
      ? [
          const compose_buffer.ToolPkgComposeDslProgram(
            id: 'test:node',
            root: 0,
            nodes: [
              compose_buffer.ToolPkgComposeDslInstruction(
                nodeType: 'Box',
                props: [1, 0],
                children: [1, 1],
                explicitChildren: true,
              ),
            ],
          ),
        ]
      : const [],
  instances: instances,
  removed: removed,
);
