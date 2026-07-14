// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserOwner.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserOwnerSurfaceHost.dart';

class WorkspaceBrowserNativeSurface extends StatefulWidget {
  /// Creates a workspace-mounted native owner WebView surface.
  const WorkspaceBrowserNativeSurface({super.key, required this.sessionId});

  final String sessionId;

  /// Creates the mutable native surface state.
  @override
  State<WorkspaceBrowserNativeSurface> createState() =>
      _WorkspaceBrowserNativeSurfaceState();
}

class _WorkspaceBrowserNativeSurfaceState
    extends State<WorkspaceBrowserNativeSurface> {
  bool _ready = false;
  int _claimGeneration = 0;

  /// Requests ownership transfer after the first placeholder frame.
  @override
  void initState() {
    super.initState();
    _claimSession();
  }

  /// Moves the native surface claim when the active tab changes.
  @override
  void didUpdateWidget(covariant WorkspaceBrowserNativeSurface oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.sessionId == widget.sessionId) {
      return;
    }
    _releaseSession(oldWidget.sessionId);
    setState(() {
      _ready = false;
    });
    _claimSession();
  }

  /// Releases the workspace claim for this WebView session.
  @override
  void dispose() {
    _claimGeneration += 1;
    _releaseSession(widget.sessionId);
    super.dispose();
  }

  /// Builds the native WebView once the owner host has released its mount.
  @override
  Widget build(BuildContext context) {
    if (!_ready) {
      return const Center(child: CircularProgressIndicator());
    }
    return AnimatedBuilder(
      animation: RuntimeBrowserOwner.instance,
      builder: (context, child) {
        final tab = RuntimeBrowserOwner.instance.tabForSession(
          widget.sessionId,
        );
        if (tab == null) {
          return const SizedBox.expand();
        }
        return RuntimeBrowserOwnerWebView(
          tab: tab,
          layoutControlsSurfaceSize: true,
        );
      },
    );
  }

  /// Claims the owner WebView for direct workspace mounting.
  void _claimSession() {
    final generation = ++_claimGeneration;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || generation != _claimGeneration) {
        return;
      }
      RuntimeBrowserOwner.instance.attachWorkspaceSurfaceSession(
        widget.sessionId,
      );
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted || generation != _claimGeneration) {
          return;
        }
        setState(() {
          _ready = true;
        });
      });
    });
  }

  /// Releases one owner WebView from workspace mounting.
  void _releaseSession(String sessionId) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      RuntimeBrowserOwner.instance.detachWorkspaceSurfaceSession(sessionId);
    });
  }
}
