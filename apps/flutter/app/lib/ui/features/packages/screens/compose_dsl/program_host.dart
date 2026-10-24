// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Creates a native-owned instruction using the same program protocol as plugin content.
_ComposeProgramNode _composeInlineProgram(
  String type,
  Map<String, Object?> props,
) {
  final store = _ComposeProgramStore();
  store.applyProgram(
    compose_buffer.ComposeProgramTransaction(
      reset: true,
      revision: 1,
      root: const compose_buffer.ToolPkgComposeDslReference(
        instanceId: 'inline',
        node: 0,
      ),
      programs: [
        compose_buffer.ToolPkgComposeDslProgram(
          id: 'inline',
          root: 0,
          nodes: [
            compose_buffer.ToolPkgComposeDslInstruction(
              nodeType: type,
              props: [0, props],
              children: const [0, null],
              explicitChildren: false,
            ),
          ],
        ),
      ],
      instances: const [
        compose_buffer.ToolPkgComposeDslInstance(
          id: 'inline',
          programId: 'inline',
          values: [],
          patches: [],
        ),
      ],
      removed: const [],
    ),
  );
  return store.root;
}

/// Converts explicit widget-test input into the production program protocol outside measured execution.
@visibleForTesting
_ComposeProgramNode _composeProgramFixture(Map<String, Object?> root) {
  final instructions = <compose_buffer.ToolPkgComposeDslInstruction>[];

  /// Compiles a finite test fixture without adding a legacy parser to production execution.
  int emit(Map<String, Object?> node) {
    final index = instructions.length;
    instructions.add(
      compose_buffer.ToolPkgComposeDslInstruction(
        nodeType: node['type'] as String,
        props: const [0, null],
        children: const [0, null],
        explicitChildren: true,
      ),
    );
    final props = _stringMap(node['props']);
    final names = props.keys.toList();
    final values = <Object?>[
      for (final name in names) [0, props[name]],
    ];
    for (final entry in _stringMap(node['slots']).entries) {
      names.add(entry.key);
      values.add([
        3,
        [
          for (final child in entry.value as List) [4, emit(_stringMap(child))],
        ],
      ]);
    }
    final children = node['children'] as List? ?? const [];
    final members = [
      for (final child in children) [4, emit(_stringMap(child))],
    ];
    instructions[index] = compose_buffer.ToolPkgComposeDslInstruction(
      nodeType: node['type'] as String,
      props: [2, names, values],
      children: [3, members],
      explicitChildren: true,
    );
    return index;
  }

  final index = emit(root);
  final store = _ComposeProgramStore();
  store.applyProgram(
    compose_buffer.ComposeProgramTransaction(
      reset: true,
      revision: 1,
      root: compose_buffer.ToolPkgComposeDslReference(
        instanceId: 'fixture',
        node: index,
      ),
      programs: [
        compose_buffer.ToolPkgComposeDslProgram(
          id: 'fixture',
          root: index,
          nodes: instructions,
        ),
      ],
      instances: const [
        compose_buffer.ToolPkgComposeDslInstance(
          id: 'fixture',
          programId: 'fixture',
          values: [],
          patches: [],
        ),
      ],
      removed: const [],
    ),
  );
  return store.root;
}
