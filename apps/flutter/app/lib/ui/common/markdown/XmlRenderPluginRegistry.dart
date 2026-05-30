// ignore_for_file: file_names

import 'package:flutter/material.dart';

typedef XmlRenderPlugin = Widget? Function({
  required String tagName,
  required String xmlContent,
  required Color textColor,
  required bool isStreaming,
  required Stream<String>? xmlStream,
});

class XmlRenderPluginRegistry {
  XmlRenderPluginRegistry._();

  static final List<XmlRenderPlugin> _plugins = <XmlRenderPlugin>[];

  static void register(XmlRenderPlugin plugin) {
    _plugins.add(plugin);
  }

  static Widget? renderIfMatched({
    required String tagName,
    required String xmlContent,
    required Color textColor,
    required bool isStreaming,
    required Stream<String>? xmlStream,
  }) {
    for (final plugin in _plugins) {
      final rendered = plugin(
        tagName: tagName,
        xmlContent: xmlContent,
        textColor: textColor,
        isStreaming: isStreaming,
        xmlStream: xmlStream,
      );
      if (rendered != null) {
        return rendered;
      }
    }
    return null;
  }
}
