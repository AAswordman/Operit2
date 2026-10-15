// ignore_for_file: file_names
part of '../ToolPkgUiLauncherScreen.dart';

class _ComposeNativeTabRow extends StatefulWidget {
  const _ComposeNativeTabRow({
    required this.selectedIndex,
    required this.scrollable,
    required this.tabs,
    required this.onSelected,
    this.color,
    this.contentColor,
  });
  final int selectedIndex;
  final bool scrollable;
  final List<Widget> tabs;
  final ValueChanged<int> onSelected;
  final Color? color;
  final Color? contentColor;
  @override
  State<_ComposeNativeTabRow> createState() => _ComposeNativeTabRowState();
}

class _ComposeNativeTabRowState extends State<_ComposeNativeTabRow>
    with SingleTickerProviderStateMixin {
  late TabController _controller;
  @override
  void initState() {
    super.initState();
    _controller = TabController(
      length: widget.tabs.length,
      initialIndex: widget.selectedIndex,
      vsync: this,
    );
  }

  @override
  void didUpdateWidget(covariant _ComposeNativeTabRow oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.tabs.length != widget.tabs.length) {
      _controller.dispose();
      _controller = TabController(
        length: widget.tabs.length,
        initialIndex: widget.selectedIndex,
        vsync: this,
      );
    } else if (_controller.index != widget.selectedIndex) {
      _controller.animateTo(widget.selectedIndex);
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Material(
    color: widget.color ?? Colors.transparent,
    child: TabBar(
      controller: _controller,
      isScrollable: widget.scrollable,
      labelColor: widget.contentColor,
      tabs: widget.tabs,
      onTap: widget.onSelected,
    ),
  );
}

class _ComposePager extends StatefulWidget {
  const _ComposePager({
    required this.currentPage,
    required this.children,
    required this.onPageChanged,
  });
  final int currentPage;
  final List<Widget> children;
  final ValueChanged<int> onPageChanged;
  @override
  State<_ComposePager> createState() => _ComposePagerState();
}

class _ComposePagerState extends State<_ComposePager> {
  late PageController _controller;
  @override
  void initState() {
    super.initState();
    _controller = PageController(initialPage: widget.currentPage);
  }

  @override
  void didUpdateWidget(covariant _ComposePager oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.currentPage != oldWidget.currentPage) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted &&
            _controller.hasClients &&
            (_controller.page! - widget.currentPage).abs() > 0.01) {
          _controller.animateToPage(
            widget.currentPage,
            duration: const Duration(milliseconds: 300),
            curve: Curves.ease,
          );
        }
      });
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => PageView(
    controller: _controller,
    onPageChanged: widget.onPageChanged,
    children: widget.children,
  );
}
