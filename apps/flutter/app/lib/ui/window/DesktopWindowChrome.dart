// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:operit_ui/operit_ui.dart';

/// Desktop windows share one navigation surface around their content panel.
bool get usesDesktopWindowChrome =>
    !kIsWeb &&
    (defaultTargetPlatform == TargetPlatform.macOS ||
        defaultTargetPlatform == TargetPlatform.windows ||
        defaultTargetPlatform == TargetPlatform.linux);

/// Owns the native caption appearance and its unscaled content inset per engine.
class DesktopWindowChrome extends StatefulWidget {
  const DesktopWindowChrome({
    super.key,
    required this.themeMode,
    required this.lightColorScheme,
    required this.darkColorScheme,
    required this.child,
  });

  final ThemeMode themeMode;
  final ColorScheme lightColorScheme;
  final ColorScheme darkColorScheme;
  final Widget child;

  @override
  State<DesktopWindowChrome> createState() => _DesktopWindowChromeState();
}

class _DesktopWindowChromeState extends State<DesktopWindowChrome>
    with WidgetsBindingObserver {
  static const _channel = MethodChannel('operit/window_chrome');
  double _topInset = 0;

  @override
  void initState() {
    super.initState();
    if (usesDesktopWindowChrome) {
      WidgetsBinding.instance.addObserver(this);
      _channel.setMethodCallHandler(_handleNativeCall);
      unawaited(_configure());
    }
  }

  @override
  void didUpdateWidget(DesktopWindowChrome oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (usesDesktopWindowChrome &&
        (widget.themeMode != oldWidget.themeMode ||
            widget.lightColorScheme != oldWidget.lightColorScheme ||
            widget.darkColorScheme != oldWidget.darkColorScheme)) {
      unawaited(_configure());
    }
  }

  @override
  void didChangePlatformBrightness() => unawaited(_configure());

  Future<void> _configure() async {
    final dark = switch (widget.themeMode) {
      ThemeMode.dark => true,
      ThemeMode.light => false,
      ThemeMode.system =>
        WidgetsBinding.instance.platformDispatcher.platformBrightness ==
            Brightness.dark,
    };
    final scheme = dark ? widget.darkColorScheme : widget.lightColorScheme;
    try {
      final inset = await _channel.invokeMethod<num>('configure', {
        'dark': dark,
        'backgroundColor': scheme.surfaceContainerLow.toARGB32(),
        'foregroundColor': scheme.onSurface.toARGB32(),
      });
      if (mounted && inset != null) _updateInset(inset);
    } on MissingPluginException {
      // Widget previews and an older native runner keep their normal caption.
    }
  }

  Future<void> _handleNativeCall(MethodCall call) async {
    if (call.method == 'topInsetChanged' && call.arguments is num) {
      _updateInset(call.arguments as num);
    }
  }

  void _updateInset(num value) {
    final inset = value.toDouble();
    if (mounted && inset.isFinite && inset >= 0 && inset != _topInset) {
      setState(() => _topInset = inset);
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    if (usesDesktopWindowChrome) _channel.setMethodCallHandler(null);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) =>
      _DesktopCaptionInset(topInset: _topInset, child: widget.child);
}

class _DesktopCaptionInset extends InheritedWidget {
  const _DesktopCaptionInset({required this.topInset, required super.child});

  final double topInset;

  @override
  bool updateShouldNotify(_DesktopCaptionInset oldWidget) =>
      topInset != oldWidget.topInset;
}

/// Extends the sidebar material under the caption, outside the rounded content.
class DesktopWindowFrame extends StatelessWidget {
  const DesktopWindowFrame({
    super.key,
    required this.child,
    required this.transparentSurface,
    this.zoom = 1,
  });

  final Widget child;
  final bool transparentSurface;
  final double zoom;

  @override
  Widget build(BuildContext context) {
    if (!usesDesktopWindowChrome) return child;
    final topInset =
        (context
                .dependOnInheritedWidgetOfExactType<_DesktopCaptionInset>()
                ?.topInset ??
            0) /
        zoom;
    return ThemeDesktopSidebarSurface(
      color: Theme.of(context).colorScheme.surfaceContainerLow,
      transparentSurface: transparentSurface,
      child: Column(
        children: [
          SizedBox(height: topInset),
          Expanded(
            child: topInset > 0
                ? MediaQuery.removePadding(
                    context: context,
                    removeTop: true,
                    child: child,
                  )
                : child,
          ),
        ],
      ),
    );
  }
}

/// Keeps all four content corners visible against the navigation background.
class DesktopContentPanel extends StatelessWidget {
  const DesktopContentPanel({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    if (!usesDesktopWindowChrome) return child;
    const radius = BorderRadius.all(Radius.circular(14));
    return Padding(
      padding: const EdgeInsets.fromLTRB(0, 6, 8, 8),
      child: DecoratedBox(
        decoration: BoxDecoration(
          borderRadius: radius,
          border: Border.all(
            color: Theme.of(
              context,
            ).colorScheme.outlineVariant.withValues(alpha: 0.45),
          ),
        ),
        position: DecorationPosition.foreground,
        child: ClipRRect(borderRadius: radius, child: child),
      ),
    );
  }
}
