// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/widgets.dart';

/// Motion tokens for the extension center.
///
/// Durations follow the app's 140-320ms `easeOutCubic` family. When the
/// platform asks for reduced motion every duration collapses to zero and
/// looping decoration stops.
class ExtensionMotion {
  const ExtensionMotion._(this.reduced);

  factory ExtensionMotion.of(BuildContext context) =>
      ExtensionMotion._(MediaQuery.disableAnimationsOf(context));

  /// Whether the platform asked to reduce motion.
  final bool reduced;

  /// Items of a first load that enter one after another; later ones enter together.
  static const int staggerLimit = 8;

  static const Curve enter = Curves.easeOutCubic;
  static const Curve exit = Curves.easeInCubic;

  /// Slightly underdamped spring for sliding selection indicators.
  static const Curve spring = ExtensionSpringCurve();

  /// Hover and press feedback.
  Duration get fast => _duration(140);

  /// Toggles, filters and state changes.
  Duration get standard => _duration(220);

  /// Expanding, collapsing and regrouping content.
  Duration get emphasized => _duration(320);

  /// Delay before the [index]th item of a first load enters.
  Duration stagger(int index) =>
      _duration(24 * math.min(index, staggerLimit));

  /// Whether looping decoration such as flowing links and shimmer may run.
  bool get ambient => !reduced;

  Duration _duration(int milliseconds) =>
      reduced ? Duration.zero : Duration(milliseconds: milliseconds);
}

/// Normalized response of a spring released from rest, settled by `t = 1`.
class ExtensionSpringCurve extends Curve {
  const ExtensionSpringCurve({this.dampingRatio = 0.78});

  /// Below 1 the curve overshoots slightly before settling.
  final double dampingRatio;

  @override
  double transformInternal(double t) {
    // The envelope e^(-ζωt) decays to 0.2% by t = 1.
    final omega = 6.2 / dampingRatio;
    final root = math.sqrt(1 - dampingRatio * dampingRatio);
    final damped = omega * root;
    final envelope = math.exp(-dampingRatio * omega * t);
    return 1 -
        envelope *
            (math.cos(damped * t) +
                dampingRatio / root * math.sin(damped * t));
  }
}
