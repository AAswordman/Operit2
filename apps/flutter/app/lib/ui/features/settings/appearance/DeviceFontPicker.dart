// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/components/OperitDialog.dart';
import '../../../theme/DeviceFontService.dart';

/// Opens device font access from a user gesture, including browser permission requests.
Future<PreparedDeviceFont?> showDeviceFontPicker(
  BuildContext context, {
  String? selectedName,
  DeviceFontService? service,
}) {
  final fonts = service ?? DeviceFontService();
  final catalog = fonts.listFamilies();
  catalog
      .ignore(); // The route subscribes on its first frame; errors remain on this future.
  return showDialog<PreparedDeviceFont>(
    context: context,
    builder: (context) => DeviceFontPicker(
      service: fonts,
      catalog: catalog,
      selectedName: selectedName,
    ),
  );
}

/// Searches the native catalog and previews the selected registered face.
class DeviceFontPicker extends StatefulWidget {
  /// Retains one explicit catalog request for the lifetime of this dialog.
  const DeviceFontPicker({
    super.key,
    required this.service,
    required this.catalog,
    this.selectedName,
  });
  final DeviceFontService service;
  final Future<List<String>> catalog;
  final String? selectedName;

  /// Creates the selection and asynchronous loading state.
  @override
  State<DeviceFontPicker> createState() => _DeviceFontPickerState();
}

class _DeviceFontPickerState extends State<DeviceFontPicker> {
  late Future<List<String>> _catalog = widget.catalog;
  final _search = TextEditingController();
  String _query = '';
  String? _loadingName;
  Object? _error;
  PreparedDeviceFont? _selected;
  var _revision = 0;

  /// Releases the search controller and invalidates outstanding face requests.
  @override
  void dispose() {
    _revision++;
    _search.dispose();
    super.dispose();
  }

  /// Publishes only the latest requested font and exposes native failures in the dialog.
  Future<void> _select(String name) async {
    final revision = ++_revision;
    setState(() {
      _loadingName = name;
      _selected = null;
      _error = null;
    });
    try {
      final selected = await widget.service.prepare(name);
      if (!mounted || revision != _revision) return;
      setState(() {
        _selected = selected;
        _loadingName = null;
      });
    } catch (error) {
      if (!mounted || revision != _revision) return;
      setState(() {
        _error = error;
        _loadingName = null;
      });
    }
  }

  /// Refreshes the catalog while preserving asynchronous errors for the dialog.
  void _reloadCatalog() {
    final catalog = widget.service.listFamilies();
    catalog.ignore();
    setState(() {
      _catalog = catalog;
    });
  }

  /// Renders a bounded searchable catalog with a real registered-font preview.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return OperitDialogScaffold(
      title: l10n.settingsAppearanceDeviceFonts,
      maxWidth: 580,
      showCloseButton: true,
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: _selected == null
              ? null
              : () => Navigator.of(context).pop(_selected),
          child: Text(l10n.save),
        ),
      ],
      child: SizedBox(
        height: MediaQuery.sizeOf(context).height * .58,
        child: Column(
          children: [
            TextField(
              controller: _search,
              decoration: InputDecoration(
                prefixIcon: const Icon(Icons.search),
                hintText: l10n.search,
              ),
              onChanged: (value) =>
                  setState(() => _query = value.toLowerCase().trim()),
            ),
            const SizedBox(height: 8),
            Expanded(
              child: FutureBuilder<List<String>>(
                future: _catalog,
                builder: (context, snapshot) {
                  if (snapshot.hasError) {
                    return Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Text(
                          l10n.settingsAppearanceThemeConfigFailed(
                            '${snapshot.error}',
                          ),
                        ),
                        TextButton(
                          onPressed: _reloadCatalog,
                          child: Text(l10n.retry),
                        ),
                      ],
                    );
                  }
                  if (!snapshot.hasData) {
                    return const Center(child: CircularProgressIndicator());
                  }
                  final names = snapshot.data!
                      .where((name) => name.toLowerCase().contains(_query))
                      .toList();
                  if (names.isEmpty) {
                    return Center(
                      child: Text(l10n.settingsAppearanceNoDeviceFonts),
                    );
                  }
                  return ListView.builder(
                    itemCount: names.length,
                    itemBuilder: (context, index) {
                      final name = names[index];
                      return ListTile(
                        dense: true,
                        title: Text(name),
                        selected:
                            (_selected?.name ??
                                _loadingName ??
                                widget.selectedName) ==
                            name,
                        trailing: _loadingName == name
                            ? const SizedBox(
                                width: 18,
                                height: 18,
                                child: CircularProgressIndicator(
                                  strokeWidth: 2,
                                ),
                              )
                            : _selected?.name == name
                            ? const Icon(Icons.check)
                            : null,
                        onTap: () => _select(name),
                      );
                    },
                  );
                },
              ),
            ),
            if (_error != null)
              Text(
                l10n.settingsAppearanceThemeConfigFailed('$_error'),
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            if (_selected case final selected?) ...[
              const Divider(),
              Text(
                selected.name,
                style: Theme.of(context).textTheme.labelMedium,
              ),
              const SizedBox(height: 8),
              Text(
                l10n.settingsAppearanceDeviceFontPreview,
                style: TextStyle(
                  fontFamily: selected.family,
                  fontFamilyFallback: const [],
                  fontSize: 20,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
