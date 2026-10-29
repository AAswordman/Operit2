// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Renders generic selection and table nodes shared by Compose packages.
extension _ComposeSelectionControls on _ComposeProgramExecutorState {
  /// Builds a compact segmented selector or a dropdown select.
  Widget _selectionControl(BuildContext context, String type) {
    final options = _optionList(props['options']);
    final selected = _string(props['value']);
    final actionId = _actionId(props['onValueChange']);
    final enabled = _enabled() && actionId != null;
    if (type == 'SegmentedControl') {
      return Wrap(
        spacing: 8,
        children: [
          for (var index = 0; index < options.length; index += 1)
            ChoiceChip(
              label: Text(options[index].label),
              selected: options[index].value == selected,
              onSelected: enabled
                  ? (_) => _invokeAction(
                      props['onValueChange'],
                      options[index].value,
                    )
                  : null,
            ),
        ],
      );
    }
    final label = _string(props['label']);
    return InputDecorator(
      decoration: InputDecoration(labelText: label.isEmpty ? null : label),
      child: DropdownMenu<String>(
        enabled: enabled,
        initialSelection: options.any((option) => option.value == selected)
            ? selected
            : null,
        expandedInsets: EdgeInsets.zero,
        dropdownMenuEntries: [
          for (final option in options)
            DropdownMenuEntry(value: option.value, label: option.label),
        ],
        onSelected: enabled
            ? (value) {
                if (value == null) return;
                _invokeAction(actionId, value);
              }
            : null,
      ),
    );
  }

  /// Builds a compact table from generic row maps.
  Widget _table() {
    final headers = _stringList(props['headers']);
    final rows = _recordList(props['rows']);
    final theme = Theme.of(context);
    return Table(
      border: TableBorder.symmetric(
        inside: BorderSide(color: theme.colorScheme.outlineVariant),
      ),
      defaultVerticalAlignment: TableCellVerticalAlignment.middle,
      children: [
        TableRow(
          children: [
            for (final header in headers)
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                child: Text(header, style: theme.textTheme.labelLarge),
              ),
          ],
        ),
        for (final row in rows)
          TableRow(
            children: [
              for (final header in headers)
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                  child: Text(
                    row[header]?.toString() ?? '',
                    style: theme.textTheme.bodySmall,
                  ),
                ),
            ],
          ),
      ],
    );
  }

  List<String> _stringList(Object? raw) {
    if (raw is! List) return const <String>[];
    return raw.map((item) => item?.toString() ?? '').toList(growable: false);
  }

  List<Map<String, Object?>> _recordList(Object? raw) {
    if (raw is! List) return const <Map<String, Object?>>[];
    return raw.whereType<Map>().map(_stringMap).toList(growable: false);
  }

  List<({String value, String label})> _optionList(Object? raw) {
    if (raw is! List) return const <({String value, String label})>[];
    return raw.map((item) {
      if (item is String) return (value: item, label: item);
      if (item is Map) {
        final map = _stringMap(item);
        final value = map['value']?.toString() ?? '';
        return (value: value, label: map['label']?.toString() ?? value);
      }
      return null;
    }).whereType<({String value, String label})>().toList(growable: false);
  }
}
