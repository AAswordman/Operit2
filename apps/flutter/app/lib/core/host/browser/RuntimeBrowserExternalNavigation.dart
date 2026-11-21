// ignore_for_file: file_names

/// Classifies browser resources separately from operating-system application links.
bool isRuntimeBrowserExternalUrl(String url) {
  final uri = Uri.tryParse(url);
  if (uri == null || !uri.hasScheme) return false;
  return !const {
    'http',
    'https',
    'file',
    'content',
    'about',
    'data',
    'blob',
    'javascript',
  }.contains(uri.scheme);
}

/// Routes an application popup before allocating a browser tab and resolves relative pages.
Future<T?> openRuntimeBrowserPopup<T>({
  required String sourceUrl,
  required String target,
  required Future<void> Function(String url) openExternal,
  required Future<T> Function(String url) openTab,
}) async {
  final url = Uri.parse(sourceUrl).resolve(target).toString();
  if (isRuntimeBrowserExternalUrl(url)) {
    await openExternal(url);
    return null;
  }
  return openTab(url);
}

/// Runs confirmation and launch once per pending URL and reports every failed handoff.
class RuntimeBrowserExternalNavigation {
  /// Connects the browser owner to its UI prompts and platform application resolver.
  RuntimeBrowserExternalNavigation({
    required this.confirm,
    required this.failed,
  });

  final Future<bool> Function(String url) confirm;
  final Future<void> Function(String url, Object error) failed;
  final _pending = <String, Future<void>>{};

  /// Coalesces repeated navigation events without duplicating dialogs or launches.
  Future<void> open(String url, Future<bool> Function() launch) {
    final pending = _pending[url];
    if (pending != null) return pending;
    final operation = _open(url, launch);
    _pending[url] = operation;
    return operation.whenComplete(() => _pending.remove(url));
  }

  /// Keeps cancellation, system refusal and platform errors as explicit outcomes.
  Future<void> _open(String url, Future<bool> Function() launch) async {
    try {
      if (!await confirm(url)) return;
      if (!await launch()) {
        throw StateError('No application accepted the external link');
      }
    } catch (error) {
      await failed(url, error);
      rethrow;
    }
  }
}
