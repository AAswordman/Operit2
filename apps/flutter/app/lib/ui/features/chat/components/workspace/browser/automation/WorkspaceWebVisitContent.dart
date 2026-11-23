// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit2/core/web_visit/RuntimeWebVisitRegistry.dart';
import 'package:operit2/core/web_visit/RuntimeWebVisitTask.dart';
import 'package:operit2/core/web_visit/WebVisitModels.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserSurfaceRegistry.dart';
import '../WorkspaceBrowserViewStore.dart';
import '../surface/WorkspaceBrowserSurface.dart';

/// Displays the authoritative visit browser without creating or executing a second visit.
class WorkspaceWebVisitContent extends StatefulWidget {
  /// Observes a runtime task whose completion is owned by the workspace presenter.
  const WorkspaceWebVisitContent({super.key, required this.request});

  final WebVisitRequest request;

  /// Creates an observer that retains surface attachment futures across state updates.
  @override
  State<WorkspaceWebVisitContent> createState() =>
      _WorkspaceWebVisitContentState();
}

class _WorkspaceWebVisitContentState extends State<WorkspaceWebVisitContent> {
  final Map<String, Future<WorkspaceBrowserTabState>> _attachments = {};

  /// Builds a real interactive browser surface with task-owned controls.
  @override
  Widget build(BuildContext context) {
    final task = RuntimeWebVisitRegistry.instance.sessionFor(
      widget.request.requestId,
    );
    if (task == null) return const Center(child: Text('Visit completed'));
    final store = WorkspaceBrowserViewStore.instance;
    return AnimatedBuilder(
      animation: Listenable.merge([task, store]),
      builder: (context, _) => Column(
        children: [
          Padding(
            padding: const EdgeInsets.all(8),
            child: Row(
              children: [
                Expanded(
                  child: Text(
                    task.currentUrl.isEmpty
                        ? widget.request.url
                        : task.currentUrl,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                const SizedBox(width: 8),
                Flexible(
                  child: Text(
                    task.statusText,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),
          ),
          Expanded(child: _surface(task, store)),
          Padding(
            padding: const EdgeInsets.all(8),
            child: Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                TextButton(
                  onPressed: task.handleLeftAction,
                  child: Text(task.leftButtonText),
                ),
                FilledButton(
                  onPressed: task.handleRightAction,
                  child: Text(task.rightButtonText),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  /// Selects the actual browser through the platform's presentation contract.
  Widget _surface(RuntimeWebVisitTask task, WorkspaceBrowserViewStore store) {
    final host = task.host;
    if (host != null) return host.buildSurface(task.request.requestId);
    final session = task.browserSession;
    if (session == null) {
      return const Center(child: CircularProgressIndicator());
    }
    if (store.usesNativeSurface) {
      return RuntimeBrowserSurfaceTarget(sessionId: session.sessionId);
    }
    return FutureBuilder<WorkspaceBrowserTabState>(
      future: _attachments.putIfAbsent(
        session.sessionId,
        () => store.attachSession(session),
      ),
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          return Center(
            child: Text('Browser presentation failed: ${snapshot.error}'),
          );
        }
        final tab = snapshot.data;
        if (tab == null) {
          return const Center(child: CircularProgressIndicator());
        }
        return WorkspaceBrowserSurface(tab: tab, store: store);
      },
    );
  }
}
