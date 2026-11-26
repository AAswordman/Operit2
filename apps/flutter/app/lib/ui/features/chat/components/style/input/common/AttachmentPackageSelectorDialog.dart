// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';
import '../../../../viewmodel/ChatViewModel.dart';
import 'MentionPackageOptions.dart';

/// Opens the shared attachment selector listing only enabled packages, skills, and MCP servers.
Future<String?> showAttachmentPackageSelectorDialog({
  required BuildContext context,
  required ChatViewModel viewModel,
}) {
  return showDialog<String>(
    context: context,
    builder: (context) => _AttachmentPackageSelectorDialog(viewModel: viewModel),
  );
}

class _AttachmentPackageSelectorDialog extends StatefulWidget {
  const _AttachmentPackageSelectorDialog({required this.viewModel});

  final ChatViewModel viewModel;

  @override
  State<_AttachmentPackageSelectorDialog> createState() =>
      _AttachmentPackageSelectorDialogState();
}

class _AttachmentPackageSelectorDialogState
    extends State<_AttachmentPackageSelectorDialog> {
  final TextEditingController _searchController = TextEditingController();
  late final Future<List<MentionPackageOption>> _future =
      loadMentionPackageOptions(widget.viewModel);

  @override
  void initState() {
    super.initState();
    _searchController.addListener(_onSearchChanged);
  }

  @override
  void dispose() {
    _searchController.removeListener(_onSearchChanged);
    _searchController.dispose();
    super.dispose();
  }

  void _onSearchChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return AlertDialog(
      title: Text(
        l10n.attachmentPackageSelectTitle,
        style: Theme.of(context).textTheme.titleMedium,
      ),
      content: SizedBox(
        width: 420,
        child: FutureBuilder<List<MentionPackageOption>>(
          future: _future,
          builder: (context, snapshot) {
            if (snapshot.hasError) {
              Error.throwWithStackTrace(snapshot.error!, snapshot.stackTrace!);
            }
            if (snapshot.connectionState != ConnectionState.done) {
              return const SizedBox(
                height: 120,
                child: Center(child: CircularProgressIndicator()),
              );
            }
            final options = snapshot.requireData;
            if (options.isEmpty) {
              return Padding(
                padding: const EdgeInsets.symmetric(vertical: 24),
                child: Text(
                  l10n.attachmentPackageEmpty,
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              );
            }

            final filteredOptions = filterMentionPackageOptions(
              options,
              _searchController.text,
            );
            return ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 400),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  TextField(
                    controller: _searchController,
                    maxLines: 1,
                    decoration: InputDecoration(
                      hintText: l10n.attachmentPackageSearchPlaceholder,
                      prefixIcon: Icon(
                        Icons.search,
                        color: colorScheme.onSurfaceVariant,
                      ),
                      suffixIcon: _searchController.text.isEmpty
                          ? null
                          : IconButton(
                              onPressed: _searchController.clear,
                              icon: const Icon(Icons.clear),
                              tooltip: l10n.clearSearch,
                            ),
                    ),
                  ),
                  const SizedBox(height: 12),
                  if (filteredOptions.isEmpty)
                    Padding(
                      padding: const EdgeInsets.symmetric(vertical: 24),
                      child: Text(
                        l10n.attachmentPackageSearchEmpty,
                        textAlign: TextAlign.center,
                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                    )
                  else
                    Flexible(
                      child: ListView.builder(
                        shrinkWrap: true,
                        itemCount: filteredOptions.length,
                        itemBuilder: (context, index) {
                          final option = filteredOptions[index];
                          return _AttachmentPackageOptionTile(
                            option: option,
                            onTap: () =>
                                Navigator.of(context).pop(option.packageName),
                          );
                        },
                      ),
                    ),
                ],
              ),
            );
          },
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
      ],
    );
  }
}

class _AttachmentPackageOptionTile extends StatelessWidget {
  const _AttachmentPackageOptionTile({
    required this.option,
    required this.onTap,
  });

  final MentionPackageOption option;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final iconColor = switch (option.kind) {
      MentionPackageKind.package => colorScheme.primary,
      MentionPackageKind.skill => colorScheme.secondary,
      MentionPackageKind.mcp => colorScheme.tertiary,
    };
    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(8),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 10),
          child: Row(
            children: <Widget>[
              Icon(Icons.auto_awesome, size: 20, color: iconColor),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      option.title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodyMedium?.copyWith(
                        color: colorScheme.onSurface,
                      ),
                    ),
                    Text(
                      buildMentionPackageSubtitle(
                        AppLocalizations.of(context)!,
                        option,
                      ),
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
