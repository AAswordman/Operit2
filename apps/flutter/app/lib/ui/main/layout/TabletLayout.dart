// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../components/DrawerContent.dart';
import '../components/NavigationDrawerAppearance.dart';
import '../navigation/AppNavigationModels.dart';

class TabletLayout extends StatelessWidget {
  const TabletLayout({
    super.key,
    required this.content,
    required this.navigationEntries,
    required this.selectedRouteId,
    required this.isNetworkAvailable,
    required this.networkType,
    required this.isTabletSidebarExpanded,
    required this.tabletSidebarWidth,
    required this.collapsedTabletSidebarWidth,
    required this.onNavigationEntrySelected,
  });

  final Widget content;
  final List<NavigationEntrySpec> navigationEntries;
  final String selectedRouteId;
  final bool isNetworkAvailable;
  final String networkType;
  final bool isTabletSidebarExpanded;
  final double tabletSidebarWidth;
  final double collapsedTabletSidebarWidth;
  final ValueChanged<NavigationEntrySpec> onNavigationEntrySelected;

  @override
  Widget build(BuildContext context) {
    final appearance = navigationDrawerAppearanceOf(context);
    final targetSidebarWidth = isTabletSidebarExpanded
        ? tabletSidebarWidth
        : collapsedTabletSidebarWidth;

    return Row(
      children: <Widget>[
        AnimatedContainer(
          duration: const Duration(milliseconds: 280),
          curve: Curves.easeInOut,
          width: targetSidebarWidth,
          height: double.infinity,
          decoration: BoxDecoration(
            color: appearance.containerColor,
            boxShadow: <BoxShadow>[
              BoxShadow(
                blurRadius: 4,
                color: Colors.black.withValues(alpha: 0.12),
              ),
            ],
          ),
          clipBehavior: Clip.antiAlias,
          child: isTabletSidebarExpanded
              ? DrawerContent(
                  navigationEntries: navigationEntries,
                  selectedRouteId: selectedRouteId,
                  isNetworkAvailable: isNetworkAvailable,
                  networkType: networkType,
                  appearance: appearance,
                  onNavigationEntrySelected: onNavigationEntrySelected,
                )
              : CollapsedDrawerContent(
                  navigationEntries: navigationEntries,
                  selectedRouteId: selectedRouteId,
                  isNetworkAvailable: isNetworkAvailable,
                  appearance: appearance,
                  onNavigationEntrySelected: onNavigationEntrySelected,
                ),
        ),
        Expanded(child: content),
      ],
    );
  }
}
