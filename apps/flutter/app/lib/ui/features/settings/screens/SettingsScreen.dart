// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../components/SettingsCategoryList.dart';
import '../components/SettingsDetailView.dart';
import '../models/SettingsModels.dart';

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key});

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  SettingsCategory? _phoneSelectedCategory;
  SettingsCategory _wideSelectedCategory = SettingsCategory.model;

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
            onCategorySelected: (category) {
              setState(() {
                _phoneSelectedCategory = category;
              });
            },
          );
        }

        return _SettingsPhoneDetail(
          category: selectedCategory,
          onBack: () {
            setState(() {
              _phoneSelectedCategory = null;
            });
          },
        );
      },
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
              color: colorScheme.surfaceContainerLowest,
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

class _SettingsPhoneDetail extends StatelessWidget {
  const _SettingsPhoneDetail({required this.category, required this.onBack});

  final SettingsCategory category;
  final VoidCallback onBack;

  @override
  Widget build(BuildContext context) {
    final spec = SettingsCategorySpec.of(category);
    return Column(
      children: <Widget>[
        Material(
          color: Theme.of(context).colorScheme.surface,
          child: SafeArea(
            bottom: false,
            child: SizedBox(
              height: 52,
              child: Row(
                children: <Widget>[
                  IconButton(
                    onPressed: onBack,
                    icon: const Icon(Icons.arrow_back),
                    tooltip: '返回设置',
                  ),
                  Icon(spec.icon, size: 21),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Text(
                      spec.title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ),
                  const SizedBox(width: 12),
                ],
              ),
            ),
          ),
        ),
        Expanded(
          child: SettingsDetailView(category: category, showHeader: false),
        ),
      ],
    );
  }
}
