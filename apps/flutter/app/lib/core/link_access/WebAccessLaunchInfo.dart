// ignore_for_file: file_names

/// Describes a Web Access URL that can bootstrap the browser runtime.
class WebAccessLaunchInfo {
  /// Creates one Web Access launch descriptor.
  const WebAccessLaunchInfo({required this.baseUrl, required this.token});

  final String baseUrl;
  final String token;
}
