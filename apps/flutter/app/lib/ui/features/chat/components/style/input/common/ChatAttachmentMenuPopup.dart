// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../../../../core/application/PluginHotReload.dart';
import '../../../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../../../l10n/generated/app_localizations.dart';
import '../../../../../../common/contributions/ChatUiContributionModels.dart';
import '../../../../../../common/contributions/ContributionPresentationResult.dart';
import '../../../../../../common/icons/MaterialIconNameResolver.dart';
import '../../../../../../main/navigation/ToolPkgCatalogChangeBus.dart';

/// Delivers a generic attachment completion to the chat that requested it.
typedef ChatAttachmentCompletion =
    Future<void> Function(Object? value, String? expectedChatId);

/// Renders native and registered attachment actions for every composer style.
class ChatAttachmentMenuPopup extends StatefulWidget {
  /// Creates the shared plus-menu without retaining any plugin business fields.
  const ChatAttachmentMenuPopup({
    super.key,
    required this.clients,
    required this.chatId,
    required this.onDismiss,
    required this.onAttachmentResult,
    required this.onAttachImage,
    required this.onTakePhoto,
    required this.onAttachFile,
    required this.onAttachScreenContent,
    required this.onAttachNotifications,
    required this.onAttachLocation,
    required this.onAttachPackage,
  });

  final GeneratedCoreProxyClients clients;
  final String? chatId;
  final VoidCallback onDismiss;
  final ChatAttachmentCompletion? onAttachmentResult;
  final VoidCallback? onAttachImage;
  final VoidCallback? onTakePhoto;
  final VoidCallback? onAttachFile;
  final VoidCallback? onAttachScreenContent;
  final VoidCallback? onAttachNotifications;
  final VoidCallback? onAttachLocation;
  final VoidCallback onAttachPackage;

  /// Creates the state that discovers registered navigation contributions.
  @override
  State<ChatAttachmentMenuPopup> createState() =>
      _ChatAttachmentMenuPopupState();
}

class _ChatAttachmentMenuPopupState extends State<ChatAttachmentMenuPopup> {
  Future<List<ChatUiChoice>>? _entriesFuture;
  String? _languageCode;
  StreamSubscription<void>? _catalogSubscription;
  bool _opening = false;

  /// Loads actual chat-attachments entries and observes catalog mutations.
  @override
  void initState() {
    super.initState();
    _catalogSubscription = ToolPkgCatalogChangeBus.listen(_reloadEntries);
    PluginHotReload.revision.addListener(_reloadEntries);
  }

  /// Loads localized registry entries when the host locale becomes available.
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final languageCode = Localizations.localeOf(context).languageCode;
    if (_languageCode != languageCode) {
      _languageCode = languageCode;
      _entriesFuture = _loadEntries(languageCode);
    }
  }

  /// Keeps discovery scoped to the receiving runtime rather than a package ID.
  @override
  void didUpdateWidget(covariant ChatAttachmentMenuPopup oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.clients != widget.clients) _reloadEntries();
  }

  /// Releases registry observers without cancelling a presented plugin route.
  @override
  void dispose() {
    unawaited(_catalogSubscription?.cancel());
    PluginHotReload.revision.removeListener(_reloadEntries);
    super.dispose();
  }

  /// Reads only the navigation registry's chat_attachments surface.
  Future<List<ChatUiChoice>> _loadEntries(String languageCode) async {
    final manager = widget.clients.application.packageManager();
    final results = await Future.wait<Object>(<Future<Object>>[
      manager.getToolPkgNavigationEntries(useEnglish: languageCode == 'en'),
      manager.getToolPkgUiRoutes(
        runtime: 'compose_dsl',
        useEnglish: languageCode == 'en',
      ),
    ]);
    final entries = results[0] as List<core_proxy.ToolPkgNavigationEntry>;
    final routes = results[1] as List<core_proxy.ToolPkgUiRoute>;
    return List<ChatUiChoice>.unmodifiable(<ChatUiChoice>[
      for (final entry in entries)
        if (entry.surface == 'chat_attachments')
          _registeredChoice(entry, routes),
    ]);
  }

  /// Resolves an entry only against the exact owning package's registered route.
  ChatUiChoice _registeredChoice(
    core_proxy.ToolPkgNavigationEntry entry,
    List<core_proxy.ToolPkgUiRoute> routes,
  ) {
    final matches = routes.where(
      (route) =>
          route.containerPackageName == entry.containerPackageName &&
          route.routeId == entry.routeId,
    );
    if (matches.length != 1 || entry.action != null) {
      throw StateError(
        'An attachment entry requires exactly one registered UI route: '
        '${entry.containerPackageName}/${entry.entryId}',
      );
    }
    return ChatUiChoice(
      id: '${entry.containerPackageName}::${entry.entryId}',
      title: entry.title,
      icon: entry.icon,
      action: ChatUiAction(route: matches.single, input: entry.params),
    );
  }

  /// Re-reads registration metadata after an explicit catalog change.
  void _reloadEntries() {
    if (!mounted) return;
    final languageCode = _languageCode;
    if (languageCode == null) return;
    setState(() => _entriesFuture = _loadEntries(languageCode));
  }

  /// Opens the exact registered route and forwards only explicit V1 completion.
  Future<void> _presentEntry(ChatUiChoice entry) async {
    final receiver = widget.onAttachmentResult;
    if (_opening || receiver == null) return;
    final chatId = widget.chatId;
    final clients = widget.clients;
    final navigator = Navigator.of(context, rootNavigator: true);
    final presentationContext = navigator.overlay!.context;
    final messenger = ScaffoldMessenger.of(context);
    setState(() => _opening = true);
    widget.onDismiss();
    try {
      final result = await entry.action.present(
        context: presentationContext,
        clients: clients,
        hostState: <String, Object?>{'chatId': chatId},
      );
      if (result?.status == ContributionPresentationStatus.completed) {
        await receiver(result!.value, chatId);
      }
    } catch (error) {
      if (messenger.mounted) {
        messenger.showSnackBar(SnackBar(content: Text('附件操作失败：$error')));
      }
    } finally {
      if (mounted) setState(() => _opening = false);
    }
  }

  /// Draws a compact native attachment row using an explicitly supplied icon.
  Widget _nativeItem(IconData icon, String title, VoidCallback? action) =>
      _menuRow(icon: icon, title: title, action: action);

  /// Draws registered display metadata without interpreting its opaque input.
  Widget _menuRow({
    required IconData? icon,
    required String title,
    required VoidCallback? action,
    Key? key,
  }) {
    final theme = Theme.of(context);
    final enabled = action != null;
    final color = enabled ? theme.colorScheme.onSurface : theme.disabledColor;
    return InkWell(
      key: key,
      onTap: action,
      child: SizedBox(
        height: 36,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          child: Row(
            children: <Widget>[
              if (icon != null) Icon(icon, size: 18, color: color),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  title,
                  style: theme.textTheme.bodySmall?.copyWith(color: color),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  /// Builds the real shared plus-menu with no domain-specific attachment item.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Material(
      color: Theme.of(context).colorScheme.surfaceContainer,
      elevation: 4,
      borderRadius: BorderRadius.circular(8),
      clipBehavior: Clip.antiAlias,
      child: SingleChildScrollView(
        padding: const EdgeInsets.symmetric(vertical: 4),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            _nativeItem(
              Icons.image,
              l10n.attachmentPhoto,
              widget.onAttachImage,
            ),
            _nativeItem(
              Icons.photo_camera,
              l10n.attachmentCamera,
              widget.onTakePhoto,
            ),
            _nativeItem(
              Icons.description,
              l10n.attachmentFile,
              widget.onAttachFile,
            ),
            _nativeItem(
              Icons.screenshot_monitor,
              l10n.attachmentScreenContent,
              widget.onAttachScreenContent,
            ),
            _nativeItem(
              Icons.notifications,
              l10n.attachmentNotifications,
              widget.onAttachNotifications,
            ),
            _nativeItem(
              Icons.location_on,
              l10n.attachmentLocation,
              widget.onAttachLocation,
            ),
            _nativeItem(
              Icons.auto_awesome,
              l10n.attachmentPackage,
              widget.onAttachPackage,
            ),
            FutureBuilder<List<ChatUiChoice>>(
              future: _entriesFuture,
              builder: (context, snapshot) {
                if (snapshot.hasError) {
                  return Padding(
                    padding: const EdgeInsets.all(12),
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        Text('插件附件入口加载失败：${snapshot.error}'),
                        TextButton(
                          onPressed: _reloadEntries,
                          child: const Text('重试'),
                        ),
                      ],
                    ),
                  );
                }
                final entries = snapshot.data;
                if (entries == null) {
                  return const Padding(
                    padding: EdgeInsets.all(12),
                    child: LinearProgressIndicator(),
                  );
                }
                return Column(
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    for (final entry in entries)
                      _menuRow(
                        key: ValueKey<String>(entry.id),
                        icon: entry.icon == null
                            ? null
                            : MaterialIconNameResolver.resolve(entry.icon!),
                        title: entry.title,
                        action: _opening || widget.onAttachmentResult == null
                            ? null
                            : () => _presentEntry(entry),
                      ),
                  ],
                );
              },
            ),
          ],
        ),
      ),
    );
  }
}
