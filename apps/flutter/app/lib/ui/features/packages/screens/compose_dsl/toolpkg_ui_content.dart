// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Composes a plugin session's content into its owning surface.
typedef ToolPkgUiContentBuilder =
    Widget Function(BuildContext context, ToolPkgUiContent content);

/// Offers box and sliver presentations of the same session-owned content.
///
/// Mount one presentation per session. Actions, root loading, and retained nodes
/// remain owned by ToolPkgUiLauncherScreen regardless of the chosen presentation.
class ToolPkgUiContent {
  /// Creates the two presentations of a rendered Compose tree.
  const ToolPkgUiContent._({required this.body, required this.sliver});

  /// Keeps a live renderer mounted while presenting action errors in either layout.
  ToolPkgUiContent._compose(_ComposeProgramExecutor renderer, Widget? errorView)
    : this._(
        body: Stack(
          children: [
            renderer,
            if (errorView != null)
              Positioned(top: 0, left: 0, right: 0, child: errorView),
          ],
        ),
        sliver: SliverMainAxisGroup(
          slivers: [
            _ComposeDslSliverContent(renderer: renderer),
            if (errorView != null) SliverToBoxAdapter(child: errorView),
          ],
        ),
      );

  /// Adapts a loading, error, or empty box to either surface protocol.
  ToolPkgUiContent._box(Widget child)
    : body = child,
      sliver = SliverToBoxAdapter(child: child);

  /// Renders content under ordinary box constraints.
  final Widget body;

  /// Exposes static lazy-column items to the embedding viewport.
  final Widget sliver;
}

/// Uses ordinary box layout for standalone screens and existing embeddings.
Widget _buildToolPkgUiBody(BuildContext context, ToolPkgUiContent content) =>
    content.body;
