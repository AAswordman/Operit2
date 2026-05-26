// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../components/DrawerContent.dart';
import '../components/NavigationDrawerAppearance.dart';
import '../navigation/AppNavigationModels.dart';

class PhoneLayout extends StatelessWidget {
  const PhoneLayout({
    super.key,
    required this.content,
    required this.navigationEntries,
    required this.selectedRouteId,
    required this.isNetworkAvailable,
    required this.networkType,
    required this.drawerWidth,
    required this.drawerOpen,
    required this.enableNavigationAnimation,
    required this.onCloseDrawer,
    required this.onNavigationEntrySelected,
  });

  final Widget content;
  final List<NavigationEntrySpec> navigationEntries;
  final String selectedRouteId;
  final bool isNetworkAvailable;
  final String networkType;
  final double drawerWidth;
  final bool drawerOpen;
  final bool enableNavigationAnimation;
  final VoidCallback onCloseDrawer;
  final ValueChanged<NavigationEntrySpec> onNavigationEntrySelected;

  @override
  Widget build(BuildContext context) {
    final appearance = navigationDrawerAppearanceOf(context);
    final drawerProgress = drawerOpen ? 1.0 : 0.0;
    final contentTranslationX = enableNavigationAnimation
        ? drawerWidth * (0.82 * drawerProgress)
        : drawerWidth * drawerProgress;
    final contentTranslationY = enableNavigationAnimation
        ? 12.0 * drawerProgress
        : 0.0;
    final contentScale = enableNavigationAnimation
        ? 1.0 - (0.08 * drawerProgress)
        : 1.0;
    final contentRotationY = enableNavigationAnimation
        ? -7.0 * drawerProgress
        : 0.0;
    final contentCornerRadius = enableNavigationAnimation
        ? 24.0 * drawerProgress
        : 0.0;
    final contentShadowElevation = enableNavigationAnimation
        ? 18.0 * drawerProgress
        : 0.0;
    final drawerOffset = -drawerWidth * (1.0 - drawerProgress);
    final sidebarElevation = enableNavigationAnimation
        ? 16.0 * drawerProgress
        : 3.0 * drawerProgress;
    final drawerScale = enableNavigationAnimation
        ? 0.92 + (0.08 * drawerProgress)
        : 1.0;
    final drawerContentAlpha = enableNavigationAnimation
        ? 0.72 + (0.28 * drawerProgress)
        : 1.0;

    final screenSize = MediaQuery.sizeOf(context);

    return Stack(
      children: <Widget>[
        Positioned.fill(
          child: AnimatedSlide(
            offset: Offset(
              contentTranslationX / screenSize.width,
              contentTranslationY / screenSize.height,
            ),
            duration: const Duration(milliseconds: 280),
            curve: Curves.fastOutSlowIn,
            child: Transform(
              alignment: Alignment.centerLeft,
              transform: Matrix4.identity()
                ..setEntry(3, 2, 0.001)
                ..rotateY(contentRotationY * math.pi / 180),
              child: AnimatedScale(
                duration: const Duration(milliseconds: 280),
                curve: Curves.fastOutSlowIn,
                scale: contentScale,
                alignment: Alignment.centerLeft,
                child: AnimatedContainer(
                  duration: const Duration(milliseconds: 280),
                  curve: Curves.fastOutSlowIn,
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(contentCornerRadius),
                    boxShadow: <BoxShadow>[
                      if (contentShadowElevation > 0)
                        BoxShadow(
                          blurRadius: contentShadowElevation,
                          color: Colors.black.withValues(alpha: 0.16),
                        ),
                    ],
                  ),
                  clipBehavior: Clip.antiAlias,
                  child: content,
                ),
              ),
            ),
          ),
        ),
        if (drawerOpen)
          Positioned.fill(
            left: drawerWidth,
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: onCloseDrawer,
              child: const ColoredBox(color: Colors.transparent),
            ),
          ),
        AnimatedPositioned(
          duration: const Duration(milliseconds: 280),
          curve: Curves.fastOutSlowIn,
          left: drawerOffset,
          top: MediaQuery.paddingOf(context).top,
          bottom: 0,
          width: drawerWidth,
          child: AnimatedOpacity(
            duration: const Duration(milliseconds: 280),
            curve: Curves.fastOutSlowIn,
            opacity: drawerContentAlpha,
            child: Transform.scale(
              alignment: Alignment.centerLeft,
              scale: drawerScale,
              child: Material(
                color: appearance.containerColor,
                elevation: sidebarElevation,
                borderRadius: const BorderRadiusDirectional.only(
                  topEnd: Radius.circular(16),
                  bottomEnd: Radius.circular(16),
                ),
                clipBehavior: Clip.antiAlias,
                child: DrawerContent(
                  navigationEntries: navigationEntries,
                  selectedRouteId: selectedRouteId,
                  isNetworkAvailable: isNetworkAvailable,
                  networkType: networkType,
                  appearance: appearance,
                  onNavigationEntrySelected: onNavigationEntrySelected,
                ),
              ),
            ),
          ),
        ),
      ],
    );
  }
}
