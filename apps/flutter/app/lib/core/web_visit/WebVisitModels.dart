// ignore_for_file: file_names

import 'dart:convert';

import '../proxy/generated/CoreProxyModels.g.dart';

class WebVisitHeader {
  const WebVisitHeader({required this.name, required this.value});

  final String name;
  final String value;

  static WebVisitHeader fromJson(Map<String, Object?> json) {
    return WebVisitHeader(
      name: json['name'] as String,
      value: json['value'] as String,
    );
  }
}

class WebVisitRequest {
  const WebVisitRequest({
    required this.requestId,
    required this.url,
    required this.headers,
    required this.userAgent,
    required this.includeImageLinks,
  });

  final String requestId;
  final String url;
  final List<WebVisitHeader> headers;
  final String userAgent;
  final bool includeImageLinks;

  factory WebVisitRequest.fromHostPayload(
    RuntimeHostInteractionWebVisitPayload payload,
  ) {
    return WebVisitRequest(
      requestId: payload.requestId,
      url: payload.url,
      headers: payload.headers
          .map(
            (header) => WebVisitHeader(
              name: header.name,
              value: header.value,
            ),
          )
          .toList(growable: false),
      userAgent: payload.userAgent,
      includeImageLinks: payload.includeImageLinks,
    );
  }

  static WebVisitRequest? decode(String? responseText) {
    if (responseText == null || responseText == 'null') {
      return null;
    }
    final json = jsonDecode(responseText) as Map<String, Object?>;
    final headersJson = json['headers'] as List<Object?>;
    return WebVisitRequest(
      requestId: json['requestId'] as String,
      url: json['url'] as String,
      headers: headersJson
          .cast<Map<String, Object?>>()
          .map(WebVisitHeader.fromJson)
          .toList(growable: false),
      userAgent: json['userAgent'] as String,
      includeImageLinks: json['includeImageLinks'] as bool,
    );
  }
}

class WebVisitLink {
  const WebVisitLink({required this.url, required this.text});

  final String url;
  final String text;

  Map<String, Object?> toJson() {
    return <String, Object?>{'url': url, 'text': text};
  }
}

class WebVisitResult {
  const WebVisitResult({
    required this.url,
    required this.title,
    required this.content,
    required this.metadata,
    required this.links,
    required this.imageLinks,
  });

  final String url;
  final String title;
  final String content;
  final Map<String, String> metadata;
  final List<WebVisitLink> links;
  final List<String> imageLinks;

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'url': url,
      'title': title,
      'content': content,
      'metadata': metadata,
      'links': links.map((link) => link.toJson()).toList(growable: false),
      'imageLinks': imageLinks,
    };
  }
}

class WebVisitResponse {
  const WebVisitResponse({
    required this.requestId,
    required this.success,
    this.result,
    this.error,
  });

  final String requestId;
  final bool success;
  final WebVisitResult? result;
  final String? error;

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'requestId': requestId,
      'success': success,
      'result': result?.toJson(),
      'error': error,
    };
  }
}
