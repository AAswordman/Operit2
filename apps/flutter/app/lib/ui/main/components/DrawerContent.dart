// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../navigation/AppNavigationModels.dart';
import 'NavigationDrawerAppearance.dart';

class DrawerContent extends StatelessWidget {
  const DrawerContent({
    super.key,
    required this.navigationEntries,
    required this.selectedRouteId,
    required this.isNetworkAvailable,
    required this.networkType,
    required this.appearance,
    required this.onNavigationEntrySelected,
  });

  final List<NavigationEntrySpec> navigationEntries;
  final String selectedRouteId;
  final bool isNetworkAvailable;
  final String networkType;
  final NavigationDrawerAppearance appearance;
  final ValueChanged<NavigationEntrySpec> onNavigationEntrySelected;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: <Widget>[
        Expanded(
          child: ListView(
            padding: const EdgeInsets.fromLTRB(0, 30, 8, 16),
            children: <Widget>[
              _SidebarInfoCard(
                brandName: 'Operit',
                isNetworkAvailable: isNetworkAvailable,
                networkType: networkType,
                appearance: appearance,
              ),
              const SizedBox(height: 14),
              Padding(
                padding: const EdgeInsetsDirectional.only(
                  start: 28,
                  end: 20,
                  bottom: 2,
                ),
                child: Text(
                  'AI Features',
                  style: Theme.of(context).textTheme.titleSmall?.copyWith(
                    color: appearance.titleColor.withValues(alpha: 0.82),
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              const SizedBox(height: 6),
              for (final entry in navigationEntries)
                CompactNavigationDrawerItem(
                  icon: entry.icon,
                  label: entry.title,
                  selected: selectedRouteId == entry.routeId,
                  appearance: appearance,
                  onClick: () => onNavigationEntrySelected(entry),
                ),
            ],
          ),
        ),
        Divider(
          height: 1,
          indent: 20,
          endIndent: 20,
          color: appearance.dividerColor.withValues(alpha: 0.5),
        ),
        const SizedBox(height: 8),
      ],
    );
  }
}

class CollapsedDrawerContent extends StatelessWidget {
  const CollapsedDrawerContent({
    super.key,
    required this.navigationEntries,
    required this.selectedRouteId,
    required this.isNetworkAvailable,
    required this.appearance,
    required this.onNavigationEntrySelected,
  });

  final List<NavigationEntrySpec> navigationEntries;
  final String selectedRouteId;
  final bool isNetworkAvailable;
  final NavigationDrawerAppearance appearance;
  final ValueChanged<NavigationEntrySpec> onNavigationEntrySelected;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.symmetric(vertical: 24),
      children: <Widget>[
        Center(
          child: _RoundDrawerButton(
            selected: false,
            appearance: appearance,
            icon: isNetworkAvailable ? Icons.wifi : Icons.wifi_off,
            onClick: () {},
          ),
        ),
        const SizedBox(height: 16),
        Divider(indent: 20, endIndent: 20, color: appearance.dividerColor),
        const SizedBox(height: 16),
        for (final entry in navigationEntries)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: Center(
              child: _RoundDrawerButton(
                selected: selectedRouteId == entry.routeId,
                appearance: appearance,
                icon: entry.icon,
                onClick: () => onNavigationEntrySelected(entry),
              ),
            ),
          ),
      ],
    );
  }
}

class _SidebarInfoCard extends StatelessWidget {
  const _SidebarInfoCard({
    required this.brandName,
    required this.isNetworkAvailable,
    required this.networkType,
    required this.appearance,
  });

  final String brandName;
  final bool isNetworkAvailable;
  final String networkType;
  final NavigationDrawerAppearance appearance;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 6),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            brandName,
            style: Theme.of(context).textTheme.titleLarge?.copyWith(
              letterSpacing: 0.5,
              color: appearance.titleColor,
              fontWeight: FontWeight.bold,
            ),
          ),
          const SizedBox(height: 10),
          DecoratedBox(
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(50),
              color: appearance.statusAvailableColor.withValues(alpha: 0.12),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Container(
                    width: 6,
                    height: 6,
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      color: isNetworkAvailable
                          ? const Color(0xFF4CAF50)
                          : const Color(0xFFEF5350),
                    ),
                  ),
                  const SizedBox(width: 6),
                  Icon(
                    isNetworkAvailable ? Icons.wifi : Icons.wifi_off,
                    size: 14,
                    color: appearance.statusAvailableColor,
                  ),
                  const SizedBox(width: 4),
                  Text(
                    networkType,
                    style: Theme.of(context).textTheme.labelMedium?.copyWith(
                      color: appearance.statusAvailableColor,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class CompactNavigationDrawerItem extends StatelessWidget {
  const CompactNavigationDrawerItem({
    super.key,
    required this.icon,
    required this.label,
    required this.selected,
    required this.appearance,
    required this.onClick,
  });

  final IconData icon;
  final String label;
  final bool selected;
  final NavigationDrawerAppearance appearance;
  final VoidCallback onClick;

  @override
  Widget build(BuildContext context) {
    final itemShape = BorderRadius.circular(16);
    return Padding(
      padding: const EdgeInsetsDirectional.only(start: 12, end: 0, bottom: 6),
      child: Material(
        color: selected
            ? appearance.selectedContainerColor
            : Colors.transparent,
        borderRadius: itemShape,
        child: InkWell(
          borderRadius: itemShape,
          onTap: onClick,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
            child: Row(
              children: <Widget>[
                Icon(
                  icon,
                  size: 22,
                  color: selected
                      ? appearance.selectedContentColor
                      : appearance.itemColor,
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: selected
                          ? appearance.selectedContentColor
                          : appearance.itemColor,
                      fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _RoundDrawerButton extends StatelessWidget {
  const _RoundDrawerButton({
    required this.selected,
    required this.appearance,
    required this.icon,
    required this.onClick,
  });

  final bool selected;
  final NavigationDrawerAppearance appearance;
  final IconData icon;
  final VoidCallback onClick;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: selected ? appearance.selectedContainerColor : Colors.transparent,
      shape: const CircleBorder(),
      child: IconButton(
        onPressed: onClick,
        icon: Icon(
          icon,
          color: selected
              ? appearance.selectedContentColor
              : appearance.itemColor,
        ),
      ),
    );
  }
}
