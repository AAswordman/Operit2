// ignore_for_file: file_names

import 'package:flutter/material.dart';

abstract final class SettingsControlStyles {
  static const Size activePillSize = Size(78, 28);
  static const Size sectionIconButtonSize = Size(32, 32);
  static const Size entityIconButtonSize = Size(32, 32);

  static TextStyle sectionTitleTextStyle(BuildContext context) {
    return Theme.of(
      context,
    ).textTheme.titleMedium!.copyWith(fontWeight: FontWeight.w700);
  }

  static ButtonStyle sectionTextButton() {
    return TextButton.styleFrom(
      visualDensity: VisualDensity.compact,
      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
      minimumSize: const Size(0, 32),
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
    );
  }

  static ButtonStyle sectionFilledButton() {
    return FilledButton.styleFrom(
      visualDensity: VisualDensity.compact,
      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
      minimumSize: const Size(0, 38),
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 9),
    );
  }

  static ButtonStyle activeTextButton() {
    return TextButton.styleFrom(
      visualDensity: VisualDensity.compact,
      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
      minimumSize: activePillSize,
      fixedSize: activePillSize,
      padding: EdgeInsets.zero,
    );
  }

  static TextStyle activeTextStyle(BuildContext context) {
    return Theme.of(
      context,
    ).textTheme.labelSmall!.copyWith(fontWeight: FontWeight.w700);
  }

  static ButtonStyle entityIconButton() {
    return IconButton.styleFrom(
      visualDensity: VisualDensity.compact,
      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
      minimumSize: entityIconButtonSize,
      fixedSize: entityIconButtonSize,
      padding: EdgeInsets.zero,
      iconSize: 20,
    );
  }
}

class SettingsActivePill extends StatelessWidget {
  const SettingsActivePill({super.key, required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return SizedBox(
      width: SettingsControlStyles.activePillSize.width,
      height: SettingsControlStyles.activePillSize.height,
      child: Chip(
        label: Center(
          child: Text(
            label,
            style: SettingsControlStyles.activeTextStyle(
              context,
            ).copyWith(color: colorScheme.onPrimaryContainer),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
        padding: EdgeInsets.zero,
        labelPadding: EdgeInsets.zero,
        materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
        visualDensity: VisualDensity.compact,
        backgroundColor: colorScheme.primaryContainer.withValues(alpha: 0.7),
        side: BorderSide.none,
        shape: const StadiumBorder(),
      ),
    );
  }
}

class SettingsSetActiveButton extends StatelessWidget {
  const SettingsSetActiveButton({
    super.key,
    required this.label,
    required this.onPressed,
  });

  final String label;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: SettingsControlStyles.activePillSize.width,
      height: SettingsControlStyles.activePillSize.height,
      child: TextButton(
        onPressed: onPressed,
        style: SettingsControlStyles.activeTextButton(),
        child: Text(
          label,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: SettingsControlStyles.activeTextStyle(context),
        ),
      ),
    );
  }
}

class SettingsEntityIconButton extends StatelessWidget {
  const SettingsEntityIconButton({
    super.key,
    required this.tooltip,
    required this.icon,
    required this.onPressed,
  });

  final String tooltip;
  final IconData icon;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    return IconButton(
      tooltip: tooltip,
      onPressed: onPressed,
      visualDensity: VisualDensity.compact,
      constraints: BoxConstraints.tight(
        SettingsControlStyles.entityIconButtonSize,
      ),
      padding: EdgeInsets.zero,
      iconSize: 20,
      style: SettingsControlStyles.entityIconButton(),
      icon: Icon(icon),
    );
  }
}

class SettingsEntityPopupIconButton<T> extends StatelessWidget {
  const SettingsEntityPopupIconButton({
    super.key,
    required this.tooltip,
    required this.icon,
    required this.onSelected,
    required this.itemBuilder,
  });

  final String tooltip;
  final IconData icon;
  final PopupMenuItemSelected<T> onSelected;
  final PopupMenuItemBuilder<T> itemBuilder;

  @override
  Widget build(BuildContext context) {
    return PopupMenuButton<T>(
      tooltip: tooltip,
      itemBuilder: itemBuilder,
      onSelected: onSelected,
      icon: Icon(icon),
      iconSize: 20,
      padding: EdgeInsets.zero,
      constraints: BoxConstraints.tight(
        SettingsControlStyles.entityIconButtonSize,
      ),
      style: SettingsControlStyles.entityIconButton(),
    );
  }
}
