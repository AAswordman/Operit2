import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

/// Web implementation of [PlatformSslAuthError].
///
/// Browser iframe APIs do not expose recoverable TLS certificate decisions to
/// embedded Flutter Web apps. This type keeps the platform interface surface
/// explicit while reporting the platform limitation at the call site.
class WebPlatformSslAuthError extends PlatformSslAuthError {
  /// Creates a [WebPlatformSslAuthError].
  WebPlatformSslAuthError({
    String description =
        'Recoverable SSL certificate decisions are not exposed by browser iframe APIs.',
  }) : super(certificate: null, description: description);

  static const String _unsupportedMessage =
      'Recoverable SSL certificate decisions are not supported on web because '
      'browser iframe APIs do not allow embedded apps to proceed past or '
      'cancel TLS certificate errors.';

  @override
  Future<void> proceed() async {
    throw UnsupportedError(_unsupportedMessage);
  }

  @override
  Future<void> cancel() async {
    throw UnsupportedError(_unsupportedMessage);
  }
}
