// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../l10n/generated/app_localizations.dart';

/// Offers application language choices using their recognizable native names.
class AppLanguageControl extends StatefulWidget {
  /// Creates a language selector backed by the ordinary preference store.
  const AppLanguageControl({
    super.key,
    this.preferences = const UserPreferencesManager(),
  });

  final UserPreferencesManager preferences;

  /// Creates state that prevents overlapping preference writes.
  @override
  State<AppLanguageControl> createState() => _AppLanguageControlState();
}

class _AppLanguageControlState extends State<AppLanguageControl> {
  bool _saving = false;

  /// Presents the full language list and applies a committed selection immediately.
  Future<void> _chooseLanguage() async {
    final l10n = AppLocalizations.of(context)!;
    final selected = await showDialog<AppLanguage>(
      context: context,
      builder: (context) => SimpleDialog(
        title: Text(l10n.settingsAppLanguageTitle),
        children: <Widget>[
          for (final language in AppLanguage.values)
            ListTile(
              key: ValueKey('settings.appLanguage.${language.name}'),
              leading: const Icon(Icons.language_outlined),
              title: Text(
                language.nativeName ?? l10n.settingsAppLanguageSystem,
              ),
              trailing: language == UserPreferencesManager.appLanguage.value
                  ? const Icon(Icons.check)
                  : null,
              onTap: () => Navigator.of(context).pop(language),
            ),
        ],
      ),
    );
    if (!mounted ||
        selected == null ||
        selected == UserPreferencesManager.appLanguage.value) {
      return;
    }
    setState(() => _saving = true);
    try {
      await widget.preferences.saveAppLanguage(selected);
    } catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            AppLocalizations.of(
              context,
            )!.settingsAppLanguageSaveError(error.toString()),
          ),
        ),
      );
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  /// Displays the selected mode and an immediate-application hint.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return ValueListenableBuilder<AppLanguage>(
      valueListenable: UserPreferencesManager.appLanguage,
      builder: (context, language, _) => ListTile(
        key: const ValueKey('settings.appLanguage'),
        contentPadding: EdgeInsets.zero,
        leading: const Icon(Icons.language_outlined),
        title: Text(language.nativeName ?? l10n.settingsAppLanguageSystem),
        subtitle: Text(l10n.settingsAppLanguageDescription),
        trailing: _saving
            ? const SizedBox.square(
                dimension: 20,
                child: CircularProgressIndicator(strokeWidth: 2),
              )
            : const Icon(Icons.chevron_right),
        onTap: _saving ? null : _chooseLanguage,
      ),
    );
  }
}
