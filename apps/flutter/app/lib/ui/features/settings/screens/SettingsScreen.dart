// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../../../main/MainLayoutController.dart';
import '../../../main/TopBarController.dart';
import '../../../main/navigation/AppNavigationModels.dart';
import '../../../main/screens/OperitScreens.dart';
import '../../../main/screens/ScreenRouteRegistry.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../../../theme/OperitTheme.dart';
import '../components/SettingsCategoryList.dart';
import '../components/SettingsDetailView.dart';
import '../components/SettingsLayoutMetrics.dart';
import '../models/SettingsModels.dart';

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key, this.initialCategory});

  final SettingsCategory? initialCategory;

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  static const Duration _detailSwitchDuration = Duration(milliseconds: 220);
  static const double _detailSwitchOffset = 0.025;

  late SettingsCategory? _phoneSelectedCategory = widget.initialCategory;
  late SettingsCategory _wideSelectedCategory =
      widget.initialCategory ?? SettingsCategory.model;
  TopBarController? _topBarController;
  bool _isCurrentMainScreen = true;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _topBarController = TopBarScope.of(context);
    _isCurrentMainScreen = MainScreenActivityScope.isCurrentScreenOf(context);
    _syncTopBarTitle();
  }

  @override
  void didUpdateWidget(covariant SettingsScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.initialCategory != widget.initialCategory) {
      _phoneSelectedCategory = widget.initialCategory;
      _wideSelectedCategory = widget.initialCategory ?? _wideSelectedCategory;
      _syncTopBarTitle();
    }
  }

  @override
  void dispose() {
    _topBarController?.clearTitleContent(owner: this);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final useWideLayout = settingsUseWideLayout(context);
    if (useWideLayout) {
      return _buildWideSettingsLayout(context);
    }

    final selectedCategory = _phoneSelectedCategory;
    if (selectedCategory == null) {
      return SettingsCategoryList(
        selectedCategory: null,
        onCategorySelected: _openPhoneCategory,
      );
    }

    return SettingsDetailView(category: selectedCategory);
  }

  Widget _buildWideSettingsLayout(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final themeSnapshot = OperitTheme.of(context).themePreferenceSnapshot;
    final backgroundVisible =
        themeSnapshot.useBackgroundImage &&
        themeSnapshot.backgroundImageUri != null &&
        themeSnapshot.backgroundImageUri!.isNotEmpty;
    final transparentSurface = themeSnapshot.transparentSurfaceEnabled;
    final sidebarColor = backgroundVisible
        ? colorScheme.surface.withValues(alpha: 0.72)
        : transparentSurface
        ? colorScheme.surface.withValues(alpha: 0.04)
        : colorScheme.surface;
    return Row(
      children: <Widget>[
        SizedBox(
          width: 260,
          child: OperitGlassSurface(
            color: sidebarColor,
            layer: OperitGlassSurfaceLayer.panel,
            transparentAlpha: 0.035,
            borderRadius: BorderRadius.zero,
            border: Border(
              right: BorderSide(
                color: colorScheme.outlineVariant.withValues(
                  alpha: transparentSurface ? 0.18 : 0.45,
                ),
              ),
            ),
            child: SettingsCategoryList(
              selectedCategory: _wideSelectedCategory,
              onCategorySelected: _selectWideCategory,
            ),
          ),
        ),
        Expanded(
          child: AnimatedSwitcher(
            duration: _detailSwitchDuration,
            switchInCurve: Curves.easeOutCubic,
            switchOutCurve: Curves.easeInCubic,
            layoutBuilder: (currentChild, previousChildren) {
              return Stack(
                fit: StackFit.expand,
                children: <Widget>[...previousChildren, ?currentChild],
              );
            },
            transitionBuilder: (child, animation) {
              return FadeTransition(
                opacity: animation,
                child: SlideTransition(
                  position: Tween<Offset>(
                    begin: const Offset(_detailSwitchOffset, 0),
                    end: Offset.zero,
                  ).animate(animation),
                  child: child,
                ),
              );
            },
            child: SettingsDetailView(
              key: ValueKey<SettingsCategory>(_wideSelectedCategory),
              category: _wideSelectedCategory,
            ),
          ),
        ),
      ],
    );
  }

  void _selectWideCategory(SettingsCategory category) {
    if (_wideSelectedCategory == category) {
      return;
    }
    setState(() {
      _wideSelectedCategory = category;
    });
    _syncTopBarTitle();
  }

  void _openPhoneCategory(SettingsCategory category) {
    final entry = ScreenRouteRegistry.toEntry(
      screen: SettingsScreenRoute(category: category),
    );
    AppRouterGateway.navigate(
      routeId: entry.routeId,
      args: entry.args,
      source: entry.source,
    );
  }

  void _syncTopBarTitle() {
    final controller = _topBarController;
    if (controller == null) {
      return;
    }
    if (!_isCurrentMainScreen) {
      controller.clearTitleContent(owner: this);
      return;
    }
    final category = settingsUseWideLayout(context)
        ? _wideSelectedCategory
        : _phoneSelectedCategory;
    if (category == null) {
      controller.clearTitleContent(owner: this);
      return;
    }
    final spec = SettingsCategorySpec.of(
      category,
      AppLocalizations.of(context)!,
    );
    controller.setTitleContent(
      TopBarTitleContent(
        (context) => Text(
          spec.title,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: Theme.of(context).textTheme.titleSmall?.copyWith(
            color: Theme.of(context).colorScheme.onSurface,
            fontSize: 14,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
      owner: this,
    );
  }
}
