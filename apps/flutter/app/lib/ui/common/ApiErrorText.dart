// ignore_for_file: file_names

final _apiHttpStatus = RegExp(
  r'^\s*(?:[A-Za-z_][\w.]*:\s*)*'
  r'(?:(?:(?:list_models )?request failed|connection failed|model list fetch error|connection test error|'
  r'Codex (?:device authorization|device token polling|token refresh|token exchange) failed):\s*)*'
  r'(?:HTTP(?:/\d(?:\.\d)?)?\s+)?([45]\d{2})(?=\s|:|$)',
  caseSensitive: false,
);

/// Reads an explicit HTTP status from the provider's original request error.
int? apiErrorHttpStatus(String error) {
  final match = _apiHttpStatus.firstMatch(error);
  return match == null ? null : int.parse(match.group(1)!);
}

/// Presentation categories shared by HTTP and structured provider errors.
enum ApiErrorCategory {
  request,
  key,
  billing,
  access,
  address,
  timeout,
  limit,
  server,
  network,
  info,
  unknown,
}

/// A single explanation shape used by every API error surface.
typedef ApiErrorExplanation = ({
  String title,
  String message,
  ApiErrorCategory category,
});

/// Explains HTTP failures from their status without guessing from body text.
ApiErrorExplanation? apiErrorExplanation(int? status) {
  final description = switch (status) {
    400 || 422 => (
      '请求参数有误',
      'API 服务器无法处理请求参数，请检查模型名称、模型能力与参数配置，并查看原始错误中的具体原因。',
      ApiErrorCategory.request,
    ),
    401 => (
      'API Key 有误',
      '你的 API Key 未通过服务器验证，请检查密钥是否正确、完整且有效。',
      ApiErrorCategory.key,
    ),
    402 => (
      'API 欠费',
      'API 账户余额不足或额度已用完，请到 API 服务商处检查余额并充值。',
      ApiErrorCategory.billing,
    ),
    403 => (
      '请求被服务器风控拒绝',
      'API 服务器拒绝了请求，可能触发风控或访问限制，请联系 API 服务商确认。',
      ApiErrorCategory.access,
    ),
    404 || 410 => (
      '接口或模型不存在',
      'API 服务器未找到请求的接口或模型，或该资源已下线，请检查服务地址、接口路径和模型名称。',
      ApiErrorCategory.address,
    ),
    405 || 501 => (
      '接口不支持此请求方式',
      'API 服务器不支持当前请求方法或功能，请检查供应商类型与接口协议是否匹配。',
      ApiErrorCategory.request,
    ),
    406 || 415 => (
      '请求内容格式不受支持',
      'API 服务器不接受当前内容类型，请检查供应商协议、媒体格式和自定义请求头。',
      ApiErrorCategory.request,
    ),
    407 => (
      '代理验证失败',
      '网络代理要求身份验证，请检查代理账号和认证配置。',
      ApiErrorCategory.key,
    ),
    408 || 504 || 522 || 524 => (
      'API 请求超时',
      'API 请求或服务器之间的连接等待超时，请检查网络和 API 服务商状态后重试。',
      ApiErrorCategory.timeout,
    ),
    409 => (
      '请求状态冲突',
      'API 服务器报告请求与当前资源状态冲突，请查看原始错误中的冲突原因。',
      ApiErrorCategory.request,
    ),
    411 => (
      '请求缺少内容长度',
      'API 服务器要求声明请求内容长度，请联系 API 服务商确认接口要求。',
      ApiErrorCategory.request,
    ),
    413 || 414 || 431 => (
      '请求内容过大',
      '消息、附件、请求地址或请求头超过了服务器限制，请按原始错误提示减少对应内容。',
      ApiErrorCategory.request,
    ),
    423 => (
      '资源已锁定',
      'API 服务器暂时锁定了请求的资源，请稍后重试或联系 API 服务商。',
      ApiErrorCategory.access,
    ),
    425 => (
      '服务器暂不接受此请求',
      'API 服务器认为请求发送过早，请稍后重试。',
      ApiErrorCategory.limit,
    ),
    426 => (
      '接口协议需要升级',
      'API 服务器要求使用其他协议版本，请检查服务地址与供应商协议配置。',
      ApiErrorCategory.request,
    ),
    429 => (
      '服务器限流 / 风控',
      'API 服务器限制了请求频率或触发了风控，请稍后再试或联系 API 服务商。',
      ApiErrorCategory.limit,
    ),
    451 => (
      '服务因地区或法规限制不可用',
      'API 服务器因地区或法律政策拒绝提供资源，请联系 API 服务商确认可用范围。',
      ApiErrorCategory.access,
    ),
    499 => (
      '服务器风控 / 请求中断',
      'API 请求被中断，可能与服务器风控、网关或连接关闭有关，请结合原始错误联系 API 服务商确认。',
      ApiErrorCategory.network,
    ),
    500 => (
      'API 服务器内部错误',
      'API 服务器处理请求时发生内部错误，请稍后重试或向 API 服务商提供原始错误。',
      ApiErrorCategory.server,
    ),
    502 || 520 => (
      'API 网关响应异常',
      'API 网关未能从上游服务取得有效响应，请稍后重试或联系 API 服务商。',
      ApiErrorCategory.server,
    ),
    503 || 529 => (
      'API 服务暂时不可用',
      'API 服务可能正在维护、过载或暂无可用资源，请稍后重试或联系 API 服务商。',
      ApiErrorCategory.server,
    ),
    521 || 523 => (
      'API 源站无法连接',
      'API 网关无法连接源站，可能是源站离线或路由异常，请联系 API 服务商。',
      ApiErrorCategory.server,
    ),
    525 || 526 => (
      'API 服务器 TLS 或证书异常',
      'API 网关与源站之间的 TLS 握手或证书验证失败，请联系 API 服务商修复。',
      ApiErrorCategory.server,
    ),
    507 => (
      'API 服务器资源不足',
      'API 服务器没有足够的存储资源完成请求，请联系 API 服务商。',
      ApiErrorCategory.server,
    ),
    final int code when code >= 400 && code < 500 => (
      'API 服务器拒绝请求',
      'API 服务器返回请求错误，请查看原始错误中的具体原因并检查接口配置。',
      ApiErrorCategory.request,
    ),
    final int code when code >= 500 && code < 600 => (
      'API 服务器异常',
      'API 服务器未能完成请求，请向 API 服务商提供状态码和原始错误。',
      ApiErrorCategory.server,
    ),
    _ => null,
  };
  if (description == null) return null;
  final (title, message, category) = description;
  return (title: title, message: message, category: category);
}

/// Prepends a recognized explanation while preserving the entire original error.
String apiErrorText(String original) {
  final status = apiErrorHttpStatus(original);
  final explanation = apiErrorExplanation(status);
  if (explanation == null) return original;
  return 'HTTP $status：${explanation.title}\n${explanation.message}\n\n原始错误：\n$original';
}
