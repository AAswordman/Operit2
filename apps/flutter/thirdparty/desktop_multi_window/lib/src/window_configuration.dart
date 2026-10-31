class WindowConfiguration {
  const WindowConfiguration({
    required this.arguments,
    this.hiddenAtLaunch = true,
    this.transparent = false,
  });

  /// The arguments passed to the new window.
  final String arguments;

  final bool hiddenAtLaunch;

  /// Requests an alpha-capable host visual before the GTK window is realized.
  final bool transparent;

  factory WindowConfiguration.fromJson(Map<String, dynamic> json) {
    return WindowConfiguration(
      arguments: json['arguments'] as String? ?? '',
      hiddenAtLaunch: json['hiddenAtLaunch'] as bool? ?? false,
      transparent: json['transparent'] as bool? ?? false,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'arguments': arguments,
      'hiddenAtLaunch': hiddenAtLaunch,
      'transparent': transparent,
    };
  }

  @override
  String toString() {
    return 'WindowConfiguration(arguments: $arguments, hiddenAtLaunch: $hiddenAtLaunch)';
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    return other is WindowConfiguration &&
        other.arguments == arguments &&
        other.hiddenAtLaunch == hiddenAtLaunch &&
        other.transparent == transparent;
  }

  @override
  int get hashCode {
    return Object.hash(arguments, hiddenAtLaunch, transparent);
  }
}
