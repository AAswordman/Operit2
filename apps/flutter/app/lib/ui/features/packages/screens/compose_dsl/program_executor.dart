// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Addresses installed code and parameters; it owns neither mutable properties nor child nodes.
class _ComposeProgramNode {
  /// Stores a thin cursor into a shared program instance.
  const _ComposeProgramNode._(this.source, this.local);
  final _ComposeProgramInstance source;
  final int local;
  /// Provides an opaque logical address for diagnostics, independent of Flutter mount identity.
  String get id => '${source.data.id}:$local';
  /// Prevents deferred native builders from reading a released source.
  bool get isDisposed => source.isDisposed;
  /// Records a source read before resolving one installed instruction.
  _ComposeProgramProjection get _view { _ComposeProgramReadScope.read(source); return source.project(local); }
  /// Reads the already canonicalized widget type.
  String get type => _view.type;
  /// Reads the instruction's dynamic and fixed property operands.
  Map<String, Object?> get props => _view.props;
  /// Reads logical child addresses without expanding descendants.
  List<_ComposeProgramNode> get children => _view.children;
  /// Reads named child operands without creating retained node records.
  Map<String, List<_ComposeProgramNode>> get slots => _view.slots;
}
