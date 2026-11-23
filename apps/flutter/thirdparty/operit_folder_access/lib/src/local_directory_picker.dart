import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// A local folder whose path is interpreted only by its filesystem Host.
class DirectoryLocation {
  /// Keeps the Host's display name separate from its native address.
  const DirectoryLocation({required this.path, required this.name});

  final String path;
  final String name;

  /// Decodes one directory returned by the native adapter.
  factory DirectoryLocation.fromMap(Map<Object?, Object?> value) =>
      DirectoryLocation(
        path: value['path'] as String,
        name: value['name'] as String,
      );
}

/// One directory listing, including its real navigation ancestry and write status.
class DirectoryListing {
  /// Stores the complete result of one Host enumeration.
  const DirectoryListing({
    required this.directory,
    required this.writable,
    required this.breadcrumbs,
    required this.directories,
  });

  final DirectoryLocation directory;
  final bool writable;
  final List<DirectoryLocation> breadcrumbs;
  final List<DirectoryLocation> directories;

  /// Decodes directory capabilities without deriving physical paths in Flutter.
  factory DirectoryListing.fromMap(Map<Object?, Object?> value) =>
      DirectoryListing(
        directory: DirectoryLocation.fromMap(value),
        writable: value['writable'] as bool,
        breadcrumbs: _locations(value['breadcrumbs']),
        directories: _locations(value['directories']),
      );
}

/// Decodes a list of native directory descriptors.
List<DirectoryLocation> _locations(Object? values) => (values as List)
    .map(
      (value) =>
          DirectoryLocation.fromMap(Map<Object?, Object?>.from(value as Map)),
    )
    .toList(growable: false);

/// Uses the Android Host for permission, mounted volumes and native filesystem I/O.
class LocalDirectoryAccess {
  /// Allows a test messenger without changing directory-selection policy.
  const LocalDirectoryAccess({
    this.channel = const MethodChannel('operit/runtime'),
  });

  final MethodChannel channel;

  /// Requests shared-storage authorization and lists available storage volumes.
  Future<List<DirectoryLocation>> roots() async =>
      _locations(await channel.invokeMethod<Object?>('localDirectoryRoots'));

  /// Enumerates directories on a Host worker, including current write access.
  Future<DirectoryListing> list(String path) async => DirectoryListing.fromMap(
    (await channel.invokeMapMethod<Object?, Object?>('localDirectoryList', {
      'path': path,
    }))!,
  );

  /// Creates a single folder and opens the exact directory returned by the Host.
  Future<DirectoryListing> create(String path, String name) async =>
      DirectoryListing.fromMap(
        (await channel.invokeMapMethod<Object?, Object?>(
          'localDirectoryCreate',
          {'path': path, 'name': name},
        ))!,
      );
}

/// Owns the themed picker presenter below the application's Navigator.
class LocalDirectoryPickerHost extends StatefulWidget {
  /// Installs one presenter without adding platform checks to business widgets.
  const LocalDirectoryPickerHost({
    super.key,
    required this.child,
    this.access = const LocalDirectoryAccess(),
  });

  final Widget child;
  final LocalDirectoryAccess access;
  static _LocalDirectoryPickerHostState? _active;

  /// Opens the registered Material picker and returns null on user cancellation.
  static Future<String?> select() {
    final host = _active;
    if (host == null || !host.mounted) {
      throw StateError('LocalDirectoryPickerHost is not attached');
    }
    return host.select();
  }

  /// Creates the application-owned dialog presenter.
  @override
  State<LocalDirectoryPickerHost> createState() =>
      _LocalDirectoryPickerHostState();
}

class _LocalDirectoryPickerHostState extends State<LocalDirectoryPickerHost> {
  bool _opening = false;

  /// Registers this window as the directory-selection owner.
  @override
  void initState() {
    super.initState();
    LocalDirectoryPickerHost._active = this;
  }

  /// Removes only this window's presenter when it leaves the tree.
  @override
  void dispose() {
    if (LocalDirectoryPickerHost._active == this)
      LocalDirectoryPickerHost._active = null;
    super.dispose();
  }

  /// Presents a standard Material 3 surface using the active application colors.
  Future<String?> select() async {
    if (_opening) throw StateError('A directory picker is already open');
    _opening = true;
    try {
      final theme = Theme.of(context);
      return await showDialog<String>(
        context: context,
        barrierDismissible: false,
        builder: (context) => Theme(
          data: ThemeData(
            useMaterial3: true,
            colorScheme: theme.colorScheme,
            textTheme: theme.textTheme,
          ),
          child: _LocalDirectoryPicker(access: widget.access),
        ),
      );
    } finally {
      _opening = false;
    }
  }

  /// Keeps the presenter separate from the application's navigation content.
  @override
  Widget build(BuildContext context) => widget.child;
}

class _LocalDirectoryPicker extends StatefulWidget {
  /// Binds the reusable browser to its native directory adapter.
  const _LocalDirectoryPicker({required this.access});
  final LocalDirectoryAccess access;

  /// Creates the current directory, filter and asynchronous operation state.
  @override
  State<_LocalDirectoryPicker> createState() => _LocalDirectoryPickerState();
}

class _LocalDirectoryPickerState extends State<_LocalDirectoryPicker> {
  final _search = TextEditingController();
  List<DirectoryLocation> _roots = [];
  DirectoryListing? _listing;
  bool _busy = true;
  String? _error;
  String? _creationError;
  String? _requestedPath;

  /// Selects interface strings by language, independently of the platform adapter.
  bool get _zh => Localizations.localeOf(context).languageCode == 'zh';

  /// Starts authorization only after this user-initiated picker is visible.
  @override
  void initState() {
    super.initState();
    _initialize();
  }

  /// Releases search input when the dialog closes.
  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  /// Obtains storage roots and opens the first volume without invoking a system file picker.
  Future<void> _initialize() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final roots = await widget.access.roots();
      if (roots.isEmpty)
        throw StateError('No shared storage volume is available');
      if (!mounted) return;
      _roots = roots;
      await _open(roots.first.path);
    } catch (error) {
      if (mounted)
        setState(() {
          _error = _message(error);
          _busy = false;
        });
    }
  }

  /// Loads an exact Host address and keeps failed enumeration unselectable.
  Future<void> _open(String path) async {
    _requestedPath = path;
    setState(() {
      _busy = true;
      _error = null;
      _creationError = null;
    });
    try {
      final listing = await widget.access.list(path);
      if (!mounted) return;
      _search.clear();
      setState(() {
        _listing = listing;
        _busy = false;
      });
    } catch (error) {
      if (mounted)
        setState(() {
          _error = _message(error);
          _busy = false;
        });
    }
  }

  /// Displays the provider's actual failure instead of treating denial as cancellation.
  String _message(Object error) => error is PlatformException
      ? error.message ?? error.code
      : error.toString();

  /// Shows mounted volumes without constructing paths in shared Flutter code.
  void _showRoots() {
    _search.clear();
    setState(() {
      _listing = null;
      _error = null;
      _creationError = null;
      _requestedPath = null;
    });
  }

  /// Navigates one real breadcrumb upward, or returns to the volume list.
  void _back() {
    if (_busy) return;
    final breadcrumbs = _listing?.breadcrumbs;
    if (breadcrumbs != null && breadcrumbs.length > 1) {
      _open(breadcrumbs[breadcrumbs.length - 2].path);
    } else {
      _showRoots();
    }
  }

  /// Asks for a folder name and keeps creation failures visible and retryable.
  Future<void> _create() async {
    final parent = _listing!.directory.path;
    final name = await showDialog<String>(
      context: context,
      builder: (context) => _NewFolderDialog(zh: _zh),
    );
    if (!mounted || name == null) return;
    setState(() {
      _busy = true;
      _error = null;
      _creationError = null;
    });
    try {
      final listing = await widget.access.create(parent, name);
      if (!mounted) return;
      _search.clear();
      _requestedPath = listing.directory.path;
      setState(() {
        _listing = listing;
        _busy = false;
      });
    } catch (error) {
      if (mounted) {
        setState(() {
          _busy = false;
          _creationError = _message(error);
        });
      }
    }
  }

  /// Builds a compact, adaptive Material 3 folder browser with an anchored selection action.
  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final listing = _listing;
    final writable =
        listing != null && listing.writable && !_busy && _error == null;
    final size = MediaQuery.sizeOf(context);
    final compact = size.height - MediaQuery.viewInsetsOf(context).bottom < 560;
    return PopScope(
      canPop: listing == null && !_busy,
      onPopInvokedWithResult: (didPop, result) {
        if (!didPop) _back();
      },
      child: Dialog(
        insetPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 24),
        backgroundColor: colors.surfaceContainerHigh,
        clipBehavior: Clip.antiAlias,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(28)),
        child: SizedBox(
          width: 560,
          height: (size.height - 80).clamp(0, 720),
          child: Column(
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(24, 20, 12, 12),
                child: Row(
                  children: [
                    Icon(
                      Icons.folder_open_rounded,
                      color: colors.primary,
                      size: 28,
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        _zh ? '选择文件夹' : 'Choose a folder',
                        style: Theme.of(context).textTheme.headlineSmall,
                      ),
                    ),
                    IconButton(
                      tooltip: MaterialLocalizations.of(
                        context,
                      ).closeButtonTooltip,
                      onPressed: () => Navigator.pop(context),
                      icon: const Icon(Icons.close_rounded),
                    ),
                  ],
                ),
              ),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: Row(
                  children: [
                    IconButton(
                      tooltip: MaterialLocalizations.of(
                        context,
                      ).backButtonTooltip,
                      onPressed: listing == null || _busy ? null : _back,
                      icon: const Icon(Icons.arrow_back_rounded),
                    ),
                    Expanded(
                      child: SingleChildScrollView(
                        scrollDirection: Axis.horizontal,
                        reverse: true,
                        child: Row(
                          children: [
                            IconButton(
                              tooltip: _zh ? '存储位置' : 'Storage locations',
                              onPressed: _busy ? null : _showRoots,
                              icon: const Icon(Icons.storage_rounded, size: 20),
                            ),
                            for (final (index, crumb)
                                in (listing?.breadcrumbs ??
                                        <DirectoryLocation>[])
                                    .indexed) ...[
                              if (index > 0)
                                Icon(
                                  Icons.chevron_right_rounded,
                                  size: 18,
                                  color: colors.outline,
                                ),
                              TextButton(
                                onPressed:
                                    _busy ||
                                        index == listing!.breadcrumbs.length - 1
                                    ? null
                                    : () => _open(crumb.path),
                                child: Text(
                                  crumb.name,
                                  style:
                                      index == listing!.breadcrumbs.length - 1
                                      ? TextStyle(
                                          color: colors.onSurface,
                                          fontWeight: FontWeight.w600,
                                        )
                                      : null,
                                ),
                              ),
                            ],
                          ],
                        ),
                      ),
                    ),
                    IconButton.filledTonal(
                      tooltip: _zh ? '新建文件夹' : 'New folder',
                      onPressed: writable ? _create : null,
                      icon: const Icon(Icons.create_new_folder_outlined),
                    ),
                  ],
                ),
              ),
              Padding(
                padding: const EdgeInsets.fromLTRB(24, 12, 24, 16),
                child: TextField(
                  controller: _search,
                  enabled: !_busy && _error == null,
                  onChanged: (_) => setState(() {}),
                  decoration: InputDecoration(
                    hintText: _zh ? '搜索当前目录的文件夹' : 'Search folders here',
                    prefixIcon: const Icon(Icons.search_rounded),
                    filled: true,
                    fillColor: colors.surfaceContainerHighest,
                    border: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(28),
                      borderSide: BorderSide.none,
                    ),
                    contentPadding: const EdgeInsets.symmetric(
                      horizontal: 20,
                      vertical: 14,
                    ),
                  ),
                ),
              ),
              Expanded(
                child: Material(color: colors.surface, child: _body(context)),
              ),
              _footer(context, writable, showLocation: !compact),
            ],
          ),
        ),
      ),
    );
  }

  /// Shows loading, actionable failures, empty folders or a filtered native directory list.
  Widget _body(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    if (_busy) return const Center(child: CircularProgressIndicator());
    if (_error != null) {
      return Column(
        children: [
          Expanded(
            child: Center(
              child: SingleChildScrollView(
                padding: const EdgeInsets.fromLTRB(24, 16, 24, 8),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.folder_off_outlined,
                      size: 32,
                      color: colors.error,
                    ),
                    const SizedBox(height: 8),
                    Text(
                      _zh ? '无法访问此位置' : 'Cannot access this location',
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    const SizedBox(height: 8),
                    SelectableText(_error!, textAlign: TextAlign.center),
                  ],
                ),
              ),
            ),
          ),
          Padding(
            padding: const EdgeInsets.only(bottom: 12),
            child: OutlinedButton.icon(
              onPressed: _roots.isEmpty
                  ? _initialize
                  : () => _open(_requestedPath!),
              icon: const Icon(Icons.refresh_rounded),
              label: Text(_zh ? '重试' : 'Try again'),
            ),
          ),
        ],
      );
    }
    final entries = (_listing?.directories ?? _roots)
        .where(
          (entry) => entry.name.toLowerCase().contains(
            _search.text.trim().toLowerCase(),
          ),
        )
        .toList(growable: false);
    if (entries.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              _search.text.isEmpty
                  ? Icons.folder_open_rounded
                  : Icons.search_off_rounded,
              size: 56,
              color: colors.outlineVariant,
            ),
            const SizedBox(height: 16),
            Text(
              _search.text.isEmpty
                  ? (_zh ? '此文件夹为空' : 'This folder is empty')
                  : (_zh ? '没有匹配的文件夹' : 'No matching folders'),
              style: TextStyle(color: colors.onSurfaceVariant),
            ),
          ],
        ),
      );
    }
    return ListView.separated(
      padding: const EdgeInsets.symmetric(vertical: 8, horizontal: 12),
      itemCount: entries.length,
      separatorBuilder: (_, _) => const SizedBox(height: 2),
      itemBuilder: (context, index) {
        final entry = entries[index];
        return ListTile(
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(16),
          ),
          contentPadding: const EdgeInsets.symmetric(
            horizontal: 12,
            vertical: 4,
          ),
          leading: Container(
            width: 44,
            height: 44,
            decoration: BoxDecoration(
              color: colors.secondaryContainer,
              borderRadius: BorderRadius.circular(14),
            ),
            child: Icon(
              _listing == null ? Icons.storage_rounded : Icons.folder_rounded,
              color: colors.onSecondaryContainer,
            ),
          ),
          title: Text(entry.name, maxLines: 2, overflow: TextOverflow.ellipsis),
          subtitle: _listing == null
              ? Text(entry.path, maxLines: 1, overflow: TextOverflow.ellipsis)
              : null,
          trailing: Icon(Icons.chevron_right_rounded, color: colors.outline),
          onTap: () => _open(entry.path),
        );
      },
    );
  }

  /// Anchors the actual destination and confirmation below the scrollable list.
  Widget _footer(
    BuildContext context,
    bool writable, {
    required bool showLocation,
  }) {
    final colors = Theme.of(context).colorScheme;
    final listing = _listing;
    return Padding(
      padding: const EdgeInsets.fromLTRB(24, 16, 24, 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (_creationError != null) ...[
            Text(_creationError!, style: TextStyle(color: colors.error)),
            const SizedBox(height: 12),
          ],
          if (listing != null && showLocation) ...[
            Row(
              children: [
                Icon(
                  listing.writable
                      ? Icons.folder_outlined
                      : Icons.lock_outline_rounded,
                  size: 20,
                  color: colors.onSurfaceVariant,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        listing.writable
                            ? (_zh ? '所选位置' : 'Selected location')
                            : (_zh ? '此位置为只读' : 'This location is read-only'),
                        style: Theme.of(context).textTheme.labelMedium
                            ?.copyWith(color: colors.onSurfaceVariant),
                      ),
                      Text(
                        listing.directory.path,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),
          ],
          FilledButton.icon(
            onPressed: writable
                ? () => Navigator.pop(context, listing!.directory.path)
                : null,
            style: FilledButton.styleFrom(
              minimumSize: const Size.fromHeight(48),
            ),
            icon: const Icon(Icons.check_rounded),
            label: Text(_zh ? '使用此文件夹' : 'Use this folder'),
          ),
        ],
      ),
    );
  }
}

class _NewFolderDialog extends StatefulWidget {
  /// Keeps input ownership inside the dialog route, including its exit animation.
  const _NewFolderDialog({required this.zh});
  final bool zh;

  /// Creates the folder-name controller and validation state.
  @override
  State<_NewFolderDialog> createState() => _NewFolderDialogState();
}

class _NewFolderDialogState extends State<_NewFolderDialog> {
  final _name = TextEditingController();

  /// Releases input only after the dialog's TextField is unmounted.
  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  /// Checks one direct filename before submitting it to the authoritative Host.
  bool get _valid {
    final name = _name.text.trim();
    return name.isNotEmpty &&
        name != '.' &&
        name != '..' &&
        !name.contains(RegExp(r'[/\\\x00]'));
  }

  /// Submits a valid display name without constructing a physical path.
  void _submit() {
    if (_valid) Navigator.pop(context, _name.text.trim());
  }

  /// Builds a Material 3 naming dialog with immediate validation.
  @override
  Widget build(BuildContext context) => AlertDialog(
    icon: const Icon(Icons.create_new_folder_outlined),
    title: Text(widget.zh ? '新建文件夹' : 'New folder'),
    content: TextField(
      controller: _name,
      autofocus: true,
      maxLength: 100,
      onChanged: (_) => setState(() {}),
      onSubmitted: (_) => _submit(),
      decoration: InputDecoration(
        labelText: widget.zh ? '文件夹名称' : 'Folder name',
        errorText: _name.text.isNotEmpty && !_valid
            ? (widget.zh ? '请输入有效的文件夹名称' : 'Enter a valid folder name')
            : null,
        border: const OutlineInputBorder(),
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: Text(MaterialLocalizations.of(context).cancelButtonLabel),
      ),
      FilledButton(
        onPressed: _valid ? _submit : null,
        child: Text(widget.zh ? '创建' : 'Create'),
      ),
    ],
  );
}
