import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

/// Paints a native-view layer without depending on a device WebView plugin.
class TestPlatformView extends LeafRenderObjectWidget {
  const TestPlatformView({super.key});

  @override
  RenderObject createRenderObject(BuildContext context) =>
      _RenderTestPlatformView();
}

class _RenderTestPlatformView extends RenderBox {
  @override
  void performLayout() {
    size = constraints.biggest;
  }

  @override
  void paint(PaintingContext context, Offset offset) {
    context.addLayer(PlatformViewLayer(rect: offset & size, viewId: 1));
  }
}
