// ignore_for_file: file_names

String normalizeWorkspaceBrowserUrl(String raw) {
  final value = raw.trim();
  final uri = Uri.tryParse(value);
  if (uri != null && uri.hasScheme) {
    return value;
  }
  final hostLikePattern = RegExp(r'^[^\s/]+\.[^\s]+(?:/.*)?$');
  if (hostLikePattern.hasMatch(value)) {
    return 'https://$value';
  }
  return Uri.https('www.bing.com', '/search', <String, String>{
    'q': value,
  }).toString();
}

bool isWorkspaceHtmlPreviewUrl(String url) {
  final uri = Uri.tryParse(url);
  if (uri == null) {
    return false;
  }
  final host = uri.host.toLowerCase();
  final isLoopback = host == '127.0.0.1' || host == 'localhost';
  if (!isLoopback) {
    return false;
  }
  final path = uri.path.toLowerCase();
  return path.endsWith('.html') || path.endsWith('.htm');
}
