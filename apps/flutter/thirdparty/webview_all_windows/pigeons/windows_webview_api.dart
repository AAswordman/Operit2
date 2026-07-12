import 'package:pigeon/pigeon.dart';

@ConfigurePigeon(
  PigeonOptions(
    dartOut: 'lib/src/windows_webview_api.g.dart',
    cppOptions: CppOptions(namespace: 'webview_all_windows'),
    cppHeaderOut: 'windows/generated/windows_webview_api.g.h',
    cppSourceOut: 'windows/generated/windows_webview_api.g.cpp',
  ),
)
class WindowsEnvironmentOptions {
  WindowsEnvironmentOptions({
    this.userDataPath,
    this.browserExePath,
    this.additionalArguments,
  });

  String? userDataPath;
  String? browserExePath;
  String? additionalArguments;
}

class WindowsCreateWebViewResult {
  WindowsCreateWebViewResult({required this.textureId});

  int textureId;
}

class WindowsCookieData {
  WindowsCookieData({
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

  String name;
  String value;
  String domain;
  String path;
  double? expires;
  bool? isHttpOnly;
  bool? isSecure;
  int? sameSite;
  bool? isSession;
}

class WindowsPointData {
  WindowsPointData({required this.x, required this.y});

  double x;
  double y;
}

class WindowsSizeData {
  WindowsSizeData({
    required this.width,
    required this.height,
    required this.scaleFactor,
  });

  double width;
  double height;
  double scaleFactor;
}

class WindowsPointerUpdateData {
  WindowsPointerUpdateData({
    required this.pointer,
    required this.event,
    required this.x,
    required this.y,
    required this.size,
    required this.pressure,
  });

  int pointer;
  int event;
  double x;
  double y;
  double size;
  double pressure;
}

class WindowsPointerButtonData {
  WindowsPointerButtonData({required this.button, required this.isDown});

  int button;
  bool isDown;
}

class WindowsVirtualHostMappingData {
  WindowsVirtualHostMappingData({
    required this.hostName,
    required this.path,
    required this.accessKind,
  });

  String hostName;
  String path;
  int accessKind;
}

class WindowsLoadRequestData {
  WindowsLoadRequestData({
    required this.url,
    required this.method,
    required this.headers,
    this.body,
  });

  String url;
  String method;
  String headers;
  Uint8List? body;
}

@HostApi()
abstract class WindowsWebViewHostApi {
  void initializeEnvironment(WindowsEnvironmentOptions options);

  String? getWebViewVersion();

  @async
  WindowsCreateWebViewResult createWebView();

  void disposeWebView(int textureId);

  void loadUrl(int textureId, String url);

  void loadRequest(int textureId, WindowsLoadRequestData request);

  void loadStringContent(int textureId, String content);

  void reload(int textureId);

  void stop(int textureId);

  void goBack(int textureId);

  void goForward(int textureId);

  @async
  String? addScriptToExecuteOnDocumentCreated(int textureId, String script);

  void removeScriptToExecuteOnDocumentCreated(int textureId, String scriptId);

  @async
  String executeScript(int textureId, String script);

  void postWebMessage(int textureId, String message);

  void setUserAgent(int textureId, String? userAgent);

  String? getUserAgent(int textureId);

  void setJavaScriptEnabled(int textureId, bool enabled);

  @async
  bool clearCookies(int textureId);

  void setCookie(int textureId, WindowsCookieData cookie);

  @async
  List<WindowsCookieData?> getCookies(int textureId, String url);

  void deleteCookie(int textureId, WindowsCookieData cookie);

  void deleteCookiesWithNameAndUrl(int textureId, String name, String url);

  void deleteCookiesWithNameDomainAndPath(
    int textureId,
    String name,
    String domain,
    String path,
  );

  void clearCache(int textureId);

  @async
  void clearLocalStorage(int textureId);

  void setCacheDisabled(int textureId, bool disabled);

  void openDevTools(int textureId);

  void setBackgroundColor(int textureId, int color);

  void setZoomControlEnabled(int textureId, bool enabled);

  void setZoomFactor(int textureId, double zoomFactor);

  void setPopupWindowPolicy(int textureId, int policy);

  void setJavaScriptDialogCallbacksEnabled(
    int textureId,
    bool alert,
    bool confirm,
    bool prompt,
  );

  void suspend(int textureId);

  void resume(int textureId);

  void setVirtualHostNameMapping(
    int textureId,
    WindowsVirtualHostMappingData mapping,
  );

  void clearVirtualHostNameMapping(int textureId, String hostName);

  void setFpsLimit(int textureId, int maxFps);

  void setPointerUpdate(int textureId, WindowsPointerUpdateData update);

  void setCursorPos(int textureId, WindowsPointData position);

  void setPointerButton(int textureId, WindowsPointerButtonData button);

  void setScrollDelta(int textureId, WindowsPointData delta);

  void setSize(int textureId, WindowsSizeData size);
}
