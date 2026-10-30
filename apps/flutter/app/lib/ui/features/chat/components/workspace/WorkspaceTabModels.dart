// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit2/core/web_visit/WebVisitModels.dart';
import '../../../../common/contributions/PluginWorkspacePresentation.dart';
import 'file_preview/WorkspaceTextDocument.dart';

enum WorkspaceTabKind {
  home,
  workspacePicker,
  files,
  terminal,
  browser,
  webVisit,
  filePreview,
  plugin,
}

enum WorkspaceFilePreviewKind {
  image,
  audio,
  video,
  pdf,
  word,
  spreadsheet,
  presentation,
  html,
  markdown,
  text,
  binary,
}

class WorkspaceTab {
  /// Creates a workspace tab with optional persistent text editing state.
  const WorkspaceTab({
    required this.kind,
    required this.title,
    required this.icon,
    this.closable = true,
    this.filePath,
    this.absolutePath,
    this.textDocument,
    this.previewKind,
    this.url,
    this.userAgent,
    this.headers,
    this.workspaceHtmlPath,
    this.webVisitRequest,
    this.terminalSessionId,
    this.terminalType,
    this.terminalWorkingDir,
    this.browserSessionId,
    this.pluginEntryId,
    this.pluginPackageName,
    this.pluginUiModuleId,
    this.pluginPresentation,
    this.identityToken = '',
  });

  final WorkspaceTabKind kind;
  final String title;
  final IconData icon;
  final bool closable;
  final String? filePath;
  final String? absolutePath;
  final WorkspaceTextDocument? textDocument;
  final WorkspaceFilePreviewKind? previewKind;
  final String? url;
  final String? userAgent;
  final Map<String, String>? headers;
  final String? workspaceHtmlPath;
  final WebVisitRequest? webVisitRequest;
  final String? terminalSessionId;
  final String? terminalType;
  final String? terminalWorkingDir;
  final String? browserSessionId;
  final String? pluginEntryId;
  final String? pluginPackageName;
  final String? pluginUiModuleId;
  final PluginWorkspacePresentation? pluginPresentation;
  final String identityToken;
}

/// Carries a regular workspace tab through an internal pane drag operation.
@immutable
class WorkspaceTabDragPayload {
  /// Creates a payload for one tab that can move between workspace panes.
  const WorkspaceTabDragPayload({required this.tab});

  final WorkspaceTab tab;
}

WorkspaceFilePreviewKind workspacePreviewKindForPath(String path) {
  final extension = path.split('.').last.toLowerCase();
  switch (extension) {
    case 'png':
    case 'jpg':
    case 'jpeg':
    case 'gif':
    case 'bmp':
    case 'webp':
    case 'svg':
      return WorkspaceFilePreviewKind.image;
    case 'mp3':
    case 'wav':
    case 'm4a':
    case 'aac':
    case 'ogg':
    case 'opus':
    case 'flac':
      return WorkspaceFilePreviewKind.audio;
    case 'mp4':
    case 'm4v':
    case 'mov':
    case 'mkv':
    case 'avi':
    case 'webm':
    case '3gp':
      return WorkspaceFilePreviewKind.video;
    case 'pdf':
      return WorkspaceFilePreviewKind.pdf;
    case 'docx':
      return WorkspaceFilePreviewKind.word;
    case 'xlsx':
    case 'xlsm':
    case 'csv':
    case 'tsv':
      return WorkspaceFilePreviewKind.spreadsheet;
    case 'pptx':
      return WorkspaceFilePreviewKind.presentation;
    case 'html':
    case 'htm':
      return WorkspaceFilePreviewKind.html;
    case 'md':
    case 'markdown':
      return WorkspaceFilePreviewKind.markdown;
    case 'txt':
    case 'log':
    case 'json':
    case 'xml':
    case 'yaml':
    case 'yml':
    case 'toml':
    case 'ini':
    case 'dart':
    case 'kt':
    case 'java':
    case 'rs':
    case 'js':
    case 'ts':
    case 'tsx':
    case 'jsx':
    case 'css':
    case 'scss':
    case 'py':
    case 'go':
    case 'c':
    case 'cpp':
    case 'h':
    case 'hpp':
    case 'sh':
    case 'bat':
    case 'ps1':
      return WorkspaceFilePreviewKind.text;
  }
  return WorkspaceFilePreviewKind.binary;
}

IconData workspacePreviewIconForKind(WorkspaceFilePreviewKind kind) {
  switch (kind) {
    case WorkspaceFilePreviewKind.image:
      return Icons.image_outlined;
    case WorkspaceFilePreviewKind.audio:
      return Icons.audio_file_outlined;
    case WorkspaceFilePreviewKind.video:
      return Icons.video_file_outlined;
    case WorkspaceFilePreviewKind.pdf:
      return Icons.picture_as_pdf_outlined;
    case WorkspaceFilePreviewKind.word:
      return Icons.article_outlined;
    case WorkspaceFilePreviewKind.spreadsheet:
      return Icons.table_chart_outlined;
    case WorkspaceFilePreviewKind.presentation:
      return Icons.slideshow_outlined;
    case WorkspaceFilePreviewKind.html:
    case WorkspaceFilePreviewKind.markdown:
    case WorkspaceFilePreviewKind.text:
      return Icons.description_outlined;
    case WorkspaceFilePreviewKind.binary:
      return Icons.insert_drive_file_outlined;
  }
}
