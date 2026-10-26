// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit2/core/web_visit/WebVisitModels.dart';
import 'package:operit2/core/web_visit/RuntimeWebVisitSession.dart';
import '../../../../../../theme/OperitGlassSurface.dart';
import '../WorkspaceBrowserViewStore.dart';
import '../surface/WorkspaceBrowserSurface.dart';
import '../tabs/WorkspaceBrowserTabModels.dart';
import 'package:operit2/core/web_visit/RuntimeWebVisitRegistry.dart';

class WorkspaceWebVisitContent extends StatefulWidget {
  const WorkspaceWebVisitContent({super.key, required this.request});

  final WebVisitRequest request;

  @override
  State<WorkspaceWebVisitContent> createState() =>
      _WorkspaceWebVisitContentState();
}

class _WorkspaceWebVisitContentState extends State<WorkspaceWebVisitContent> {
  String? _surfaceSessionId;
  Future<WorkspaceBrowserTabState>? _surface;

  @override
  Widget build(BuildContext context) {
    final session = RuntimeWebVisitRegistry.instance.sessionFor(
      widget.request.requestId,
    );
    if (session == null) return const SizedBox.expand();
    return AnimatedBuilder(
      animation: session,
      builder: (context, _) => _buildVisit(context, session),
    );
  }

  Widget _buildVisit(BuildContext context, RuntimeWebVisitSession session) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 10, 12, 8),
      child: Column(
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: Text(
                  'Visit Web',
                  style: theme.textTheme.titleMedium!.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              DecoratedBox(
                decoration: BoxDecoration(
                  color: theme.colorScheme.primaryContainer,
                  borderRadius: BorderRadius.circular(999),
                ),
                child: Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 6,
                  ),
                  child: Text(
                    session.statusText,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          OperitGlassSurface(
            color: theme.colorScheme.surfaceContainerHighest.withValues(
              alpha: 0.34,
            ),
            layer: OperitGlassSurfaceLayer.card,
            borderRadius: BorderRadius.circular(10),
            border: Border.all(
              color: theme.colorScheme.outlineVariant.withValues(alpha: 0.2),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              child: Column(
                children: <Widget>[
                  Row(
                    children: <Widget>[
                      Expanded(
                        child: Text(
                          session.currentUrl,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: theme.textTheme.bodySmall!.copyWith(
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ),
                    ],
                  ),
                  if (session.pageTitle.isNotEmpty) ...<Widget>[
                    const SizedBox(height: 4),
                    Align(
                      alignment: Alignment.centerLeft,
                      child: Text(
                        session.pageTitle,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.bodySmall!.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
          const SizedBox(height: 10),
          Expanded(
            child: ClipRRect(
              borderRadius: BorderRadius.circular(12),
              child: ColoredBox(
                color: Colors.white,
                child: _buildSurface(session),
              ),
            ),
          ),
          const SizedBox(height: 10),
          Row(
            children: <Widget>[
              Expanded(
                child: FilledButton.tonal(
                  onPressed: session.handleLeftAction,
                  child: Text(
                    session.leftButtonText,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: FilledButton(
                  onPressed: session.pageLoaded
                      ? session.handleRightAction
                      : null,
                  child: Text(
                    session.rightButtonText,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
            ],
          ),
          if (session.helperText.isNotEmpty) ...<Widget>[
            const SizedBox(height: 4),
            Text(
              session.helperText,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              textAlign: TextAlign.center,
              style: theme.textTheme.labelSmall!.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ],
      ),
    );
  }

  Widget _buildSurface(RuntimeWebVisitSession session) {
    final info = session.browserSession;
    if (info == null) return const Center(child: CircularProgressIndicator());
    if (_surfaceSessionId != info.sessionId) {
      _surfaceSessionId = info.sessionId;
      _surface = WorkspaceBrowserViewStore.instance.attachSession(info);
    }
    return FutureBuilder<WorkspaceBrowserTabState>(
      future: _surface,
      builder: (context, snapshot) {
        if (snapshot.hasError) return Center(child: Text('${snapshot.error}'));
        final tab = snapshot.data;
        if (tab == null) {
          return const Center(child: CircularProgressIndicator());
        }
        return WorkspaceBrowserSurface(
          tab: tab,
          store: WorkspaceBrowserViewStore.instance,
        );
      },
    );
  }
}
