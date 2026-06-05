import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import '../../src/renderer/liquid_glass_renderer.dart';
import '../../types/glass_quality.dart';
import 'glass_container.dart';

/// A single step in a [GlassWizard].
///
/// Each step has a [title], optional [subtitle], optional expandable [content],
/// and an optional [isCompleted] flag.
class GlassWizardStep {
  const GlassWizardStep({
    required this.title,
    this.subtitle,
    this.content,
    this.leading,
    this.isCompleted = false,
  });

  /// The primary label of the step (typically a [Text] widget).
  final Widget title;

  /// Optional secondary label displayed under [title].
  final Widget? subtitle;

  /// Optional expanded content shown when this step is active.
  ///
  /// If null, the step header collapses to a label only.
  final Widget? content;

  /// Optional custom widget for the step indicator circle.
  ///
  /// If null, the default numbered circle or checkmark is shown.
  final Widget? leading;

  /// Whether this step has been completed.
  ///
  /// Completed steps show a checkmark and are rendered with reduced opacity.
  final bool isCompleted;
}

/// A glass-aesthetic multi-step wizard for sequential flows.
///
/// [GlassWizard] presents a vertical list of sequential steps, each with an
/// indicator circle, title, optional subtitle, and expandable content.
///
/// > **Note:** This widget is a glass-themed multi-step flow component. For
/// > the iOS 26 equivalent of `UIStepper` (the `−`/`+` numeric incrementer),
/// > use [GlassStepper] instead.
///
/// ```dart
/// GlassWizard(
///   currentStep: _step,
///   onStepTapped: (step) => setState(() => _step = step),
///   steps: [
///     GlassWizardStep(
///       title: Text('Account'),
///       subtitle: Text('Enter your credentials'),
///       content: Column(
///         children: [
///           GlassTextField(label: 'Email'),
///           GlassButton(
///             icon: Icon(CupertinoIcons.arrow_right),
///             label: 'Continue',
///             onTap: () => setState(() => _step = 1),
///           ),
///         ],
///       ),
///     ),
///     GlassWizardStep(title: Text('Profile')),
///     GlassWizardStep(title: Text('Done'), isCompleted: true),
///   ],
/// )
/// ```
class GlassWizard extends StatelessWidget {
  /// Creates a glass wizard with the given steps.
  const GlassWizard({
    super.key,
    required this.steps,
    this.currentStep = 0,
    this.onStepTapped,
    this.physics,
    this.indicatorSize = 28.0,
    this.activeColor,
    this.completedColor,
    this.inactiveColor,
    this.settings,
    this.quality,
    this.padding = const EdgeInsets.all(16),
  });

  // ===========================================================================
  // Required Properties
  // ===========================================================================

  /// The steps of the wizard.
  final List<GlassWizardStep> steps;

  // ===========================================================================
  // State Properties
  // ===========================================================================

  /// Index of the step that is currently active (expanded).
  ///
  /// Defaults to 0 (first step).
  final int currentStep;

  /// Called when the user taps on a step header.
  ///
  /// If null, tapping headers has no effect (controlled wizard).
  final ValueChanged<int>? onStepTapped;

  // ===========================================================================
  // Styling Properties
  // ===========================================================================

  /// Diameter of each step indicator circle in logical pixels.
  ///
  /// Defaults to 28.
  final double indicatorSize;

  /// Colour of the active step indicator.
  ///
  /// Defaults to `Colors.white`.
  final Color? activeColor;

  /// Colour of completed step indicators.
  ///
  /// Defaults to `CupertinoColors.systemGreen`.
  final Color? completedColor;

  /// Colour of inactive step indicators.
  ///
  /// Defaults to `Colors.white.withValues(alpha: 0.3)`.
  final Color? inactiveColor;

  /// Padding inside the glass container.
  final EdgeInsetsGeometry padding;

  // ===========================================================================
  // Glass Layer Properties
  // ===========================================================================

  /// Glass effect settings for the outer container.
  final LiquidGlassSettings? settings;

  /// Rendering quality for the glass effect.
  final GlassQuality? quality;

  /// Scroll physics for the wizard.
  final ScrollPhysics? physics;

  @override
  Widget build(BuildContext context) {
    return GlassContainer(
      shape: const LiquidRoundedSuperellipse(borderRadius: 16),
      settings: settings,
      quality: quality,
      padding: EdgeInsets.zero,
      child: Padding(
        padding: padding,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            for (int i = 0; i < steps.length; i++) ...[
              _WizardStepRow(
                step: steps[i],
                index: i,
                currentStep: currentStep,
                totalSteps: steps.length,
                indicatorSize: indicatorSize,
                activeColor: activeColor ?? Colors.white,
                completedColor: completedColor ?? CupertinoColors.systemGreen,
                inactiveColor:
                    inactiveColor ?? Colors.white.withValues(alpha: 0.3),
                onTap: onStepTapped != null ? () => onStepTapped!(i) : null,
              ),
              if (i < steps.length - 1)
                _WizardStepConnector(
                  indicatorSize: indicatorSize,
                  isCompleted: i < currentStep || steps[i].isCompleted,
                ),
            ],
          ],
        ),
      ),
    );
  }
}

// =============================================================================
// Private: Step Row
// =============================================================================

class _WizardStepRow extends StatelessWidget {
  const _WizardStepRow({
    required this.step,
    required this.index,
    required this.currentStep,
    required this.totalSteps,
    required this.indicatorSize,
    required this.activeColor,
    required this.completedColor,
    required this.inactiveColor,
    this.onTap,
  });

  final GlassWizardStep step;
  final int index;
  final int currentStep;
  final int totalSteps;
  final double indicatorSize;
  final Color activeColor;
  final Color completedColor;
  final Color inactiveColor;
  final VoidCallback? onTap;

  bool get _isActive => index == currentStep;
  bool get _isCompleted => step.isCompleted || index < currentStep;

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        GestureDetector(
          onTap: onTap,
          behavior: HitTestBehavior.opaque,
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              _buildIndicator(),
              const SizedBox(width: 12),
              Expanded(child: _buildLabel()),
            ],
          ),
        ),
        if (_isActive && step.content != null)
          Padding(
            padding: EdgeInsets.only(
              left: indicatorSize + 12,
              top: 12,
              bottom: 4,
            ),
            child: step.content,
          ),
      ],
    );
  }

  Widget _buildIndicator() {
    final Color fill;
    Widget child;

    if (_isCompleted) {
      fill = completedColor;
      child =
          Icon(Icons.check, size: indicatorSize * 0.55, color: Colors.white);
    } else if (_isActive) {
      fill = activeColor;
      child = Text(
        '${index + 1}',
        style: TextStyle(
          color: Colors.black87,
          fontSize: indicatorSize * 0.45,
          fontWeight: FontWeight.bold,
        ),
      );
    } else {
      fill = Colors.transparent;
      child = Text(
        '${index + 1}',
        style: TextStyle(
          color: inactiveColor,
          fontSize: indicatorSize * 0.45,
          fontWeight: FontWeight.w500,
        ),
      );
    }

    return AnimatedContainer(
      duration: const Duration(milliseconds: 250),
      curve: Curves.easeInOut,
      width: indicatorSize,
      height: indicatorSize,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        color: fill,
        border: Border.all(
          color: _isCompleted
              ? completedColor
              : _isActive
                  ? activeColor
                  : inactiveColor,
          width: 1.5,
        ),
      ),
      child: Center(child: child),
    );
  }

  Widget _buildLabel() {
    final double opacity = _isCompleted ? 0.55 : (_isActive ? 1.0 : 0.6);

    return AnimatedOpacity(
      duration: const Duration(milliseconds: 200),
      opacity: opacity,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          DefaultTextStyle(
            style: TextStyle(
              color: Colors.white,
              fontSize: 15,
              fontWeight: _isActive ? FontWeight.w600 : FontWeight.w400,
            ),
            child: step.title,
          ),
          if (step.subtitle != null) ...[
            const SizedBox(height: 2),
            DefaultTextStyle(
              style: TextStyle(
                color: Colors.white.withValues(alpha: 0.65),
                fontSize: 12,
              ),
              child: step.subtitle!,
            ),
          ],
        ],
      ),
    );
  }
}

// =============================================================================
// Private: Connector Line
// =============================================================================

class _WizardStepConnector extends StatelessWidget {
  const _WizardStepConnector({
    required this.indicatorSize,
    required this.isCompleted,
  });

  final double indicatorSize;
  final bool isCompleted;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.only(
        left: indicatorSize / 2 - 0.5,
        top: 4,
        bottom: 4,
      ),
      child: SizedBox(
        width: 1,
        height: 16,
        child: ColoredBox(
          color: isCompleted
              ? CupertinoColors.systemGreen.withValues(alpha: 0.7)
              : Colors.white.withValues(alpha: 0.2),
        ),
      ),
    );
  }
}
