// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../../chat/components/style/bubble/BubbleSurface.dart';
import '../components/SettingsControlStyles.dart';

/// Keeps the original image visible while editing crop and nine-slice boundaries.
class BubbleImageAdjustEditor extends StatefulWidget {
  /// Creates a source-first editor with a separate optional result preview.
  const BubbleImageAdjustEditor({
    super.key,
    required this.initialStyle,
    required this.onChanged,
    required this.previewBuilder,
  });

  final BubbleImageStyle initialStyle;
  final ValueChanged<BubbleImageStyle> onChanged;
  final Widget Function(BuildContext, BubbleImageStyle) previewBuilder;

  /// Creates the editor's draft and image zoom state.
  @override
  State<BubbleImageAdjustEditor> createState() =>
      _BubbleImageAdjustEditorState();
}

class _BubbleImageAdjustEditorState extends State<BubbleImageAdjustEditor> {
  late BubbleImageStyle _style;
  final _transform = TransformationController();

  /// Starts from the stored image slicing parameters.
  @override
  void initState() {
    super.initState();
    _style = widget.initialStyle;
  }

  /// Releases the original-image viewport controller.
  @override
  void dispose() {
    _transform.dispose();
    super.dispose();
  }

  /// Updates the draft without changing its image, render mode, or saved values.
  void _change({
    double? cropLeft,
    double? cropTop,
    double? cropRight,
    double? cropBottom,
    double? repeatStart,
    double? repeatEnd,
    double? repeatYStart,
    double? repeatYEnd,
    double? scale,
  }) {
    setState(() {
      _style = BubbleImageStyle(
        imagePath: _style.imagePath,
        renderMode: _style.renderMode,
        cropLeftRatio: cropLeft ?? _style.cropLeftRatio,
        cropTopRatio: cropTop ?? _style.cropTopRatio,
        cropRightRatio: cropRight ?? _style.cropRightRatio,
        cropBottomRatio: cropBottom ?? _style.cropBottomRatio,
        repeatXStartRatio: repeatStart ?? _style.repeatXStartRatio,
        repeatXEndRatio: repeatEnd ?? _style.repeatXEndRatio,
        repeatYStartRatio: repeatYStart ?? _style.repeatYStartRatio,
        repeatYEndRatio: repeatYEnd ?? _style.repeatYEndRatio,
        imageScale: scale ?? _style.imageScale,
      );
    });
    widget.onChanged(_style);
  }

  /// Builds a compact percentage control for one source-image boundary.
  Widget _slider(
    String label,
    double value,
    ValueChanged<double> onChanged, {
    double min = 0,
    double max = 0.45,
  }) {
    return SettingsSliderRow(
      compact: true,
      label: label,
      value: value,
      min: min,
      max: max,
      divisions: ((max - min) * 100).round().clamp(1, 100),
      valueText: '${(value * 100).round()}%',
      onChanged: onChanged,
    );
  }

  /// Pairs controls even on phones, with scrolling reserved for the control pane.
  Widget _controls(List<Widget> children, {String? hint}) {
    return ListView(
      padding: const EdgeInsets.fromLTRB(12, 8, 12, 12),
      children: <Widget>[
        for (var index = 0; index < children.length; index += 2)
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Expanded(child: children[index]),
              const SizedBox(width: 12),
              Expanded(child: children[index + 1]),
            ],
          ),
        if (hint != null)
          Text(hint, style: Theme.of(context).textTheme.bodySmall),
      ],
    );
  }

  /// Builds the pinned, zoomable source and the compact parameter tabs.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final source = Column(
      children: <Widget>[
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          child: Row(
            children: <Widget>[
              Expanded(
                child: Text(
                  l10n.settingsAppearanceBubbleImageSourceHint,
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
              IconButton(
                tooltip: l10n.settingsAppearanceBubbleImageFitSource,
                onPressed: () => _transform.value = Matrix4.identity(),
                icon: const Icon(Icons.fit_screen),
              ),
            ],
          ),
        ),
        Expanded(
          child: ClipRect(
            child: ColoredBox(
              color: Theme.of(context).colorScheme.surfaceContainerHighest,
              child: InteractiveViewer(
                transformationController: _transform,
                minScale: 1,
                maxScale: 8,
                child: SizedBox.expand(
                  child: BubbleSliceSourceGuide(imageStyle: _style),
                ),
              ),
            ),
          ),
        ),
      ],
    );
    final controls = Column(
      children: <Widget>[
        TabBar(
          labelPadding: const EdgeInsets.symmetric(horizontal: 4),
          tabs: <Widget>[
            Tab(text: l10n.settingsAppearanceBubbleImageCrop),
            Tab(text: l10n.settingsAppearanceBubbleImageGrid),
            Tab(text: l10n.settingsAppearanceBubbleImageScale),
            Tab(text: l10n.settingsAppearanceBubbleImagePreview),
          ],
        ),
        Expanded(
          child: TabBarView(
            children: <Widget>[
              _controls(<Widget>[
                _slider(
                  l10n.settingsAppearanceBubbleImageCropLeft,
                  _style.cropLeftRatio,
                  (value) => _change(cropLeft: value),
                ),
                _slider(
                  l10n.settingsAppearanceBubbleImageCropRight,
                  _style.cropRightRatio,
                  (value) => _change(cropRight: value),
                ),
                _slider(
                  l10n.settingsAppearanceBubbleImageCropTop,
                  _style.cropTopRatio,
                  (value) => _change(cropTop: value),
                ),
                _slider(
                  l10n.settingsAppearanceBubbleImageCropBottom,
                  _style.cropBottomRatio,
                  (value) => _change(cropBottom: value),
                ),
              ]),
              _controls(<Widget>[
                _slider(
                  l10n.settingsAppearanceBubbleImageRepeatStart,
                  _style.repeatXStartRatio,
                  (value) => _change(
                    repeatStart: value,
                    repeatEnd: _style.repeatXEndRatio.clamp(value + 0.01, 0.95),
                  ),
                  min: 0.05,
                  max: 0.9,
                ),
                _slider(
                  l10n.settingsAppearanceBubbleImageRepeatEnd,
                  _style.repeatXEndRatio,
                  (value) => _change(repeatEnd: value),
                  min: _style.repeatXStartRatio + 0.01,
                  max: 0.95,
                ),
                _slider(
                  l10n.settingsAppearanceBubbleImageRepeatYStart,
                  _style.repeatYStartRatio,
                  (value) => _change(
                    repeatYStart: value,
                    repeatYEnd: _style.repeatYEndRatio.clamp(
                      value + 0.01,
                      0.95,
                    ),
                  ),
                  min: 0.05,
                  max: 0.9,
                ),
                _slider(
                  l10n.settingsAppearanceBubbleImageRepeatYEnd,
                  _style.repeatYEndRatio,
                  (value) => _change(repeatYEnd: value),
                  min: _style.repeatYStartRatio + 0.01,
                  max: 0.95,
                ),
              ], hint: l10n.settingsAppearanceBubbleImageGridHint),
              ListView(
                padding: const EdgeInsets.all(12),
                children: <Widget>[
                  SettingsSliderRow(
                    compact: true,
                    label: l10n.settingsAppearanceBubbleImageScale,
                    value: _style.imageScale,
                    min: 0.2,
                    max: 3,
                    divisions: 28,
                    valueText: '${_style.imageScale.toStringAsFixed(1)}x',
                    onChanged: (value) => _change(scale: value),
                  ),
                ],
              ),
              SingleChildScrollView(
                padding: const EdgeInsets.all(12),
                child: widget.previewBuilder(context, _style),
              ),
            ],
          ),
        ),
      ],
    );
    return DefaultTabController(
      length: 4,
      child: LayoutBuilder(
        builder: (context, constraints) {
          if (constraints.maxWidth >= 720) {
            return Row(
              children: <Widget>[
                Expanded(child: source),
                const VerticalDivider(width: 1),
                SizedBox(width: 360, child: controls),
              ],
            );
          }
          return Column(
            children: <Widget>[
              Expanded(child: source),
              SizedBox(
                height: (constraints.maxHeight * 0.44).clamp(0.0, 280.0),
                child: controls,
              ),
            ],
          );
        },
      ),
    );
  }
}
