// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

class _ComposeCanvasPainter extends CustomPainter {
  /// Creates the compose canvas painter instance.
  const _ComposeCanvasPainter({
    required this.commands,
    required this.colorScheme,
    required this.textTheme,
  });

  final List<Map<String, Object?>> commands;
  final ColorScheme colorScheme;
  final TextTheme textTheme;

  /// Resolves paint for the Compose DSL renderer.
  @override
  void paint(Canvas canvas, Size size) {
    for (final command in commands) {
      final type = _normalizeToken(
        _string(command['type'] ?? command['command']),
      );
      switch (type) {
        case 'line':
        case 'drawline':
          _drawLine(canvas, size, command);
          break;
        case 'rect':
        case 'drawrect':
          _drawRect(canvas, size, command);
          break;
        case 'roundrect':
        case 'drawroundrect':
          _drawRoundRect(canvas, size, command);
          break;
        case 'circle':
        case 'drawcircle':
          _drawCircle(canvas, size, command);
          break;
        case 'text':
        case 'drawtext':
          _drawText(canvas, size, command);
          break;
        case 'icon':
        case 'drawicon':
          _drawIcon(canvas, size, command);
          break;
        case 'path':
        case 'drawpath':
          _drawPath(canvas, size, command);
          break;
      }
    }
  }

  /// Resolves draw line for the Compose DSL renderer.
  void _drawLine(Canvas canvas, Size size, Map<String, Object?> command) {
    final paint = _paint(command, stroke: true);
    canvas.drawLine(
      Offset(
        _canvasNumber(command['x1'], size.width),
        _canvasNumber(command['y1'], size.height),
      ),
      Offset(
        _canvasNumber(command['x2'], size.width),
        _canvasNumber(command['y2'], size.height),
      ),
      paint,
    );
  }

  /// Resolves draw rect for the Compose DSL renderer.
  void _drawRect(Canvas canvas, Size size, Map<String, Object?> command) {
    final rect = Rect.fromLTWH(
      _canvasNumber(command['x'], size.width),
      _canvasNumber(command['y'], size.height),
      _canvasNumber(command['width'], size.width),
      _canvasNumber(command['height'], size.height),
    );
    canvas.drawRect(rect, _paint(command, stroke: _isStroke(command)));
  }

  /// Resolves draw round rect for the Compose DSL renderer.
  void _drawRoundRect(Canvas canvas, Size size, Map<String, Object?> command) {
    final rect = Rect.fromLTWH(
      _canvasNumber(command['x'], size.width),
      _canvasNumber(command['y'], size.height),
      _canvasNumber(command['width'], size.width),
      _canvasNumber(command['height'], size.height),
    );
    final radius = _canvasNumber(
      command['radius'] ?? command['cornerRadius'],
      size.shortestSide,
    );
    canvas.drawRRect(
      RRect.fromRectAndRadius(rect, Radius.circular(radius)),
      _paint(command, stroke: _isStroke(command)),
    );
  }

  /// Resolves draw circle for the Compose DSL renderer.
  void _drawCircle(Canvas canvas, Size size, Map<String, Object?> command) {
    canvas.drawCircle(
      Offset(
        _canvasNumber(command['cx'] ?? command['x'], size.width),
        _canvasNumber(command['cy'] ?? command['y'], size.height),
      ),
      _canvasNumber(command['radius'] ?? command['r'], size.shortestSide),
      _paint(command, stroke: _isStroke(command)),
    );
  }

  /// Resolves draw text for the Compose DSL renderer.
  void _drawText(Canvas canvas, Size size, Map<String, Object?> command) {
    final text = _string(command['text']);
    if (text.isEmpty) {
      return;
    }
    final painter =
        TextPainter(
          text: TextSpan(
            text: text,
            style: textTheme.bodyMedium!.copyWith(
              color: _canvasColor(command['color']) ?? colorScheme.onSurface,
              fontWeight: _fontWeight(_string(command['fontWeight'])),
            ),
          ),
          textScaler: TextScaler.linear(
            _canvasNumber(command['fontSize'], size.shortestSide, base: 14) /
                textTheme.bodyMedium!.fontSize!,
          ),
          maxLines: _int(command['maxLines']),
          textDirection: TextDirection.ltr,
        )..layout(
          minWidth: _canvasNumber(command['minWidth'], size.width),
          maxWidth: _canvasNumber(
            command['maxWidth'],
            size.width,
            base: size.width <= 0 ? double.infinity : size.width,
          ),
        );
    painter.paint(
      canvas,
      Offset(
        _canvasNumber(command['x'], size.width),
        _canvasNumber(command['y'], size.height),
      ),
    );
  }

  /// Resolves draw icon for the Compose DSL renderer.
  void _drawIcon(Canvas canvas, Size size, Map<String, Object?> command) {
    final icon = _iconData(_string(command['name'] ?? command['icon']));
    final fontSize = _canvasNumber(
      command['size'],
      size.shortestSide,
      base: 24,
    );
    final painter = TextPainter(
      text: TextSpan(
        text: String.fromCharCode(icon.codePoint),
        style: TextStyle(
          fontFamily: icon.fontFamily,
          color:
              _canvasColor(command['color'] ?? command['tint']) ??
              colorScheme.onSurface,
        ),
      ),
      textScaler: TextScaler.linear(fontSize / textTheme.bodyMedium!.fontSize!),
      textDirection: TextDirection.ltr,
    )..layout();
    painter.paint(
      canvas,
      Offset(
        _canvasNumber(command['x'], size.width),
        _canvasNumber(command['y'], size.height),
      ),
    );
  }

  /// Resolves draw path for the Compose DSL renderer.
  void _drawPath(Canvas canvas, Size size, Map<String, Object?> command) {
    final ops = command['path'];
    if (ops is! List<Object?>) {
      return;
    }
    final path = Path();
    for (final rawOp in ops.whereType<Map<Object?, Object?>>()) {
      final op = _string(rawOp['op'] ?? rawOp['type'] ?? rawOp['command']);
      switch (_normalizeToken(op)) {
        case 'moveto':
          path.moveTo(
            _canvasNumber(rawOp['x'], size.width),
            _canvasNumber(rawOp['y'], size.height),
          );
          break;
        case 'lineto':
          path.lineTo(
            _canvasNumber(rawOp['x'], size.width),
            _canvasNumber(rawOp['y'], size.height),
          );
          break;
        case 'quadto':
          path.quadraticBezierTo(
            _canvasNumber(rawOp['x1'], size.width),
            _canvasNumber(rawOp['y1'], size.height),
            _canvasNumber(rawOp['x2'], size.width),
            _canvasNumber(rawOp['y2'], size.height),
          );
          break;
        case 'cubicto':
          path.cubicTo(
            _canvasNumber(rawOp['x1'], size.width),
            _canvasNumber(rawOp['y1'], size.height),
            _canvasNumber(rawOp['x2'], size.width),
            _canvasNumber(rawOp['y2'], size.height),
            _canvasNumber(rawOp['x3'], size.width),
            _canvasNumber(rawOp['y3'], size.height),
          );
          break;
        case 'close':
          path.close();
          break;
      }
    }
    canvas.drawPath(path, _paint(command, stroke: _isStroke(command)));
  }

  Paint _paint(Map<String, Object?> command, {required bool stroke}) {
    return Paint()
      ..isAntiAlias = true
      ..style = stroke ? PaintingStyle.stroke : PaintingStyle.fill
      ..strokeWidth = _number(command['strokeWidth']) ?? 1
      ..color =
          _canvasColor(command['color'] ?? command['brush']) ??
          colorScheme.primary;
  }

  /// Evaluates is stroke for the Compose DSL renderer.
  bool _isStroke(Map<String, Object?> command) =>
      _normalizeToken(_string(command['style'])) == 'stroke' ||
      _number(command['strokeWidth']) != null;

  /// Resolves canvas color for the Compose DSL renderer.
  Color? _canvasColor(Object? raw) {
    if (raw is Map<Object?, Object?>) {
      final colors = raw['colors'];
      if (colors is List<Object?> && colors.isNotEmpty) {
        return _canvasColor(colors.first);
      }
      return _colorForScheme(colorScheme, raw);
    }
    if (raw is String && raw.trim().startsWith('#')) {
      final hex = raw.trim().substring(1);
      final parsed = int.tryParse(hex.length == 6 ? 'ff$hex' : hex, radix: 16);
      return parsed == null ? null : Color(parsed);
    }
    return _colorToken(colorScheme, _string(raw));
  }

  /// Resolves canvas number for the Compose DSL renderer.
  double _canvasNumber(Object? raw, double axis, {double base = 0}) {
    if (raw is Map<Object?, Object?>) {
      final value = _number(raw['value']) ?? base;
      final unit = _string(raw['unit']).toLowerCase();
      return unit == 'fraction' ? value * axis : value;
    }
    return _number(raw) ?? base;
  }

  /// Evaluates should repaint for the Compose DSL renderer.
  @override
  bool shouldRepaint(covariant _ComposeCanvasPainter oldDelegate) =>
      oldDelegate.commands != commands ||
      oldDelegate.colorScheme != colorScheme ||
      oldDelegate.textTheme != textTheme;
}
