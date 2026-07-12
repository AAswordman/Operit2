/// Windows WebView2 cookie SameSite policy.
enum WindowsWebViewCookieSameSite {
  /// Cookie is sent in all contexts.
  none,

  /// Cookie is withheld on cross-site subrequests.
  lax,

  /// Cookie is only sent in same-site contexts.
  strict,
}

/// Full Windows WebView2 cookie data.
class WindowsWebViewCookie {
  /// Creates Windows WebView2 cookie data.
  const WindowsWebViewCookie({
    required this.name,
    required this.value,
    required this.domain,
    required this.path,
    this.expires,
    this.isHttpOnly,
    this.isSecure,
    this.sameSite,
    this.isSession,
  });

  /// Cookie name.
  final String name;

  /// Cookie value.
  final String value;

  /// Cookie domain.
  final String domain;

  /// Cookie path.
  final String path;

  /// Expiration time. Null means the setter leaves the native default.
  final DateTime? expires;

  /// Whether the cookie is HTTP-only.
  final bool? isHttpOnly;

  /// Whether the cookie is secure-only.
  final bool? isSecure;

  /// SameSite policy.
  final WindowsWebViewCookieSameSite? sameSite;

  /// Whether this cookie is a session cookie. Returned by WebView2.
  final bool? isSession;
}
