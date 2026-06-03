// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'MarketStatsSupport.dart';

class MarketBrowseControls extends StatelessWidget {
  const MarketBrowseControls({
    super.key,
    required this.sortOption,
    required this.enabled,
    required this.onSortChanged,
  });

  final MarketSortOption sortOption;
  final bool enabled;
  final ValueChanged<MarketSortOption> onSortChanged;

  @override
  Widget build(BuildContext context) {
    if (!enabled) {
      return const SizedBox(height: 8);
    }
    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
      child: SizedBox(
        width: double.infinity,
        child: SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: Row(
            spacing: 8,
            crossAxisAlignment: CrossAxisAlignment.center,
            children: <Widget>[
              Text(
                '排序',
                style: Theme.of(context).textTheme.labelMedium?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                  fontWeight: FontWeight.w500,
                ),
              ),
              _MarketSortChip(
                selected: sortOption == MarketSortOption.downloads,
                label: '下载',
                onSelected: () => onSortChanged(MarketSortOption.downloads),
              ),
              _MarketSortChip(
                selected: sortOption == MarketSortOption.updated,
                label: '更新',
                onSelected: () => onSortChanged(MarketSortOption.updated),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _MarketSortChip extends StatelessWidget {
  const _MarketSortChip({
    required this.selected,
    required this.label,
    required this.onSelected,
  });

  final bool selected;
  final String label;
  final VoidCallback onSelected;

  @override
  Widget build(BuildContext context) {
    return FilterChip(
      selected: selected,
      showCheckmark: false,
      label: Text(label),
      labelStyle: Theme.of(context).textTheme.labelMedium,
      labelPadding: const EdgeInsets.symmetric(horizontal: 8),
      padding: EdgeInsets.zero,
      visualDensity: VisualDensity.compact,
      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
      onSelected: (_) => onSelected(),
    );
  }
}

class MarketTopBarSearchField extends StatefulWidget {
  const MarketTopBarSearchField({
    super.key,
    required this.query,
    required this.onQueryChanged,
    required this.onClose,
  });

  final String query;
  final ValueChanged<String> onQueryChanged;
  final VoidCallback onClose;

  @override
  State<MarketTopBarSearchField> createState() =>
      _MarketTopBarSearchFieldState();
}

class _MarketTopBarSearchFieldState extends State<MarketTopBarSearchField> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.query,
  );
  late final FocusNode _focusNode = FocusNode();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      _focusNode.requestFocus();
    });
  }

  @override
  void didUpdateWidget(covariant MarketTopBarSearchField oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.query != _controller.text) {
      _controller.value = TextEditingValue(
        text: widget.query,
        selection: TextSelection.collapsed(offset: widget.query.length),
      );
    }
  }

  @override
  void dispose() {
    _focusNode.dispose();
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final contentColor = Theme.of(context).colorScheme.onSurface;
    return TextField(
      controller: _controller,
      focusNode: _focusNode,
      onChanged: widget.onQueryChanged,
      maxLines: 1,
      textInputAction: TextInputAction.search,
      cursorColor: contentColor,
      style: Theme.of(
        context,
      ).textTheme.bodyMedium?.copyWith(color: contentColor),
      decoration: InputDecoration(
        hintText: '搜索市场',
        hintStyle: Theme.of(context).textTheme.bodyMedium?.copyWith(
          color: contentColor.withValues(alpha: 0.72),
        ),
        prefixIcon: Icon(
          Icons.search,
          color: contentColor.withValues(alpha: 0.9),
        ),
        suffixIcon: IconButton(
          onPressed: widget.onClose,
          icon: Icon(Icons.close, color: contentColor.withValues(alpha: 0.9)),
          tooltip: '取消',
        ),
        border: InputBorder.none,
        focusedBorder: InputBorder.none,
        enabledBorder: InputBorder.none,
        isDense: true,
        contentPadding: const EdgeInsets.symmetric(vertical: 12),
      ),
    );
  }
}
