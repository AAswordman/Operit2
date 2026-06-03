// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../main/TopBarController.dart';
import '../../../main/navigation/AppNavigationModels.dart';
import '../../../main/screens/OperitScreens.dart';
import '../../../main/screens/ScreenRouteRegistry.dart';
import '../components/SettingsCategoryList.dart';
import '../components/SettingsDetailView.dart';
import '../models/SettingsModels.dart';

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key, this.initialCategory});

  final SettingsCategory? initialCategory;

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  late SettingsCategory? _phoneSelectedCategory = widget.initialCategory;
  SettingsCategory _wideSelectedCategory = SettingsCategory.model;
  TopBarController? _topBarController;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _topBarController = TopBarScope.of(context);
    _syncTopBarTitle();
  }

  @override
  void didUpdateWidget(covariant SettingsScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.initialCategory != widget.initialCategory) {
      _phoneSelectedCategory = widget.initialCategory;
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
    return LayoutBuilder(
      builder: (context, constraints) {
        final useWideLayout = constraints.maxWidth >= 760;
        if (useWideLayout) {
          return _SettingsWideLayout(
            selectedCategory: _wideSelectedCategory,
            onCategorySelected: (category) {
              setState(() {
                _wideSelectedCategory = category;
              });
            },
          );
        }

        final selectedCategory = _phoneSelectedCategory;
        if (selectedCategory == null) {
          return SettingsCategoryList(
            selectedCategory: null,
            onCategorySelected: _openPhoneCategory,
          );
        }

        return SettingsDetailView(category: selectedCategory);
      },
    );
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
    final category = widget.initialCategory;
    if (category == null) {
      controller.clearTitleContent(owner: this);
      return;
    }
    final spec = SettingsCategorySpec.of(category);
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

class _SettingsWideLayout extends StatelessWidget {
  const _SettingsWideLayout({
    required this.selectedCategory,
    required this.onCategorySelected,
  });

  final SettingsCategory selectedCategory;
  final ValueChanged<SettingsCategory> onCategorySelected;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Row(
      children: <Widget>[
        SizedBox(
          width: 260,
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: colorScheme.surface,
              border: Border(
                right: BorderSide(
                  color: colorScheme.outlineVariant.withValues(alpha: 0.45),
                ),
              ),
            ),
            child: SettingsCategoryList(
              selectedCategory: selectedCategory,
              onCategorySelected: onCategorySelected,
            ),
          ),
        ),
        Expanded(child: SettingsDetailView(category: selectedCategory)),
      ],
    );
  }
}
