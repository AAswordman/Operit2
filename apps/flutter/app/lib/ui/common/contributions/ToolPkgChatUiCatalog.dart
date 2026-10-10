// ignore_for_file: file_names

import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import 'ChatUiContributionModels.dart';

/// Reads registered display contracts without retaining plugin business records.
class ToolPkgChatUiCatalog {
  /// Binds discovery to the actual receiving runtime and host display language.
  const ToolPkgChatUiCatalog({required this.clients, this.useEnglish = false});

  final GeneratedCoreProxyClients clients;
  final bool useEnglish;

  /// Resolves selector, identity, and background display data from actual owners.
  Future<ChatUiContext> loadContext({required String? chatId}) async {
    const api = 'chat.context.actions';
    final owners = await _owners(api);
    final routes = await _routes();
    ChatUiIdentity? identity;
    String? backgroundUri;
    for (final owner in owners) {
      final data = _object(
        await _invoke(owner, api, <String, Object?>{'chatId': chatId}),
        'chat context',
      );
      _fields(data, const <String>{
        'identity',
        'backgroundUri',
      }, 'chat context');
      final rawIdentity = data['identity'];
      if (rawIdentity != null) {
        if (identity != null) {
          throw StateError('Multiple API owners published a chat identity.');
        }
        final display = _object(rawIdentity, 'identity');
        _fields(display, const <String>{
          'title',
          'avatarUri',
          'action',
        }, 'identity');
        identity = ChatUiIdentity(
          title: _text(display['title'], 'identity.title'),
          avatarUri: _nullableText(display['avatarUri'], 'identity.avatarUri'),
          action: _action(display['action'], owner, routes),
        );
      }
      final background = _nullableText(data['backgroundUri'], 'backgroundUri');
      if (background != null) {
        if (backgroundUri != null) {
          throw StateError('Multiple API owners published a chat background.');
        }
        backgroundUri = background;
      }
    }
    return ChatUiContext(identity: identity, backgroundUri: backgroundUri);
  }

  /// Projects only the provided generic history summaries into owner sections.
  Future<List<ChatUiSection>> loadSections({
    required List<core_proxy.ChatHistoryListItem> chats,
  }) async {
    const api = 'chat.list.sections';
    final owners = await _owners(api);
    final routes = await _routes();
    final knownChatIds = chats.map((chat) => chat.id).toSet();
    final sections = <ChatUiSection>[];
    final payload = <String, Object?>{
      'chats': chats.map(chatUiHistorySummary).toList(growable: false),
    };
    for (final owner in owners) {
      final data = _object(await _invoke(owner, api, payload), 'chat sections');
      _fields(data, const <String>{'sections'}, 'chat sections');
      final ids = <String>{};
      for (final raw in _list(data['sections'], 'sections')) {
        final section = _object(raw, 'section');
        _fields(section, const <String>{
          'id',
          'title',
          'avatarUri',
          'chatIds',
          'selection',
          'preview',
        }, 'section');
        final id = _identity(section['id'], 'section.id');
        if (!ids.add(id)) throw const FormatException('Duplicate section ID.');
        final chatIds = _list(
          section['chatIds'],
          'section.chatIds',
        ).map((value) => _identity(value, 'chat ID')).toList(growable: false);
        if (chatIds.toSet().length != chatIds.length ||
            chatIds.any((chatId) => !knownChatIds.contains(chatId))) {
          throw const FormatException(
            'A section contains duplicate or unknown chat IDs.',
          );
        }
        sections.add(
          ChatUiSection(
            ownerPackageName: owner,
            id: id,
            title: _text(section['title'], 'section.title'),
            avatarUri: _nullableText(section['avatarUri'], 'section.avatarUri'),
            chatIds: List<String>.unmodifiable(chatIds),
            selection: section['selection'],
            preview: _action(section['preview'], owner, routes),
          ),
        );
      }
    }
    return List<ChatUiSection>.unmodifiable(sections);
  }

  /// Invokes the same section owner's binding API only after a chat exists.
  Future<void> writeBinding({
    required String ownerPackageName,
    required String chatId,
    required Object? selection,
  }) async {
    const api = 'chat.configuration.binding.write';
    final names = await clients.application
        .packageManager()
        .getToolPkgPublicApis(packageName: ownerPackageName);
    if (!names.contains(api)) {
      throw StateError('Section owner does not publish $api.');
    }
    await _invoke(ownerPackageName, api, <String, Object?>{
      'chatId': chatId,
      'selection': selection,
    });
  }

  /// Requires unique enabled owners that actually registered the exact API name.
  Future<List<String>> _owners(String api) async {
    final declarations = await clients.application
        .packageManager()
        .getToolPkgPublicApiOwners(apiName: api);
    final owners = <String>{};
    for (final declaration in declarations) {
      if (declaration.apiName != api ||
          declaration.containerPackageName.isEmpty ||
          !owners.add(declaration.containerPackageName)) {
        throw StateError('Invalid or duplicate public API owner for $api.');
      }
    }
    return owners.toList(growable: false);
  }

  /// Reads registered Compose routes from the same canonical package manager.
  Future<List<core_proxy.ToolPkgUiRoute>> _routes() => clients.application
      .packageManager()
      .getToolPkgUiRoutes(runtime: 'compose_dsl', useEnglish: useEnglish);

  /// Calls a published method without executing an AI tool or a navigation action.
  Future<Object?> _invoke(String owner, String api, Object? payload) =>
      clients.application.packageManager().invokeToolPkgPublicApi(
        packageName: owner,
        methodName: api,
        payload: payload,
      );

  /// Accepts only the generic route and input fields of a declared presentation.
  ChatUiAction _action(
    Object? raw,
    String owner,
    List<core_proxy.ToolPkgUiRoute> routes,
  ) {
    final data = _object(raw, 'presentation action');
    _fields(data, const <String>{'routeId', 'input'}, 'presentation action');
    return _resolveAction(owner, data['routeId'], data['input'], routes);
  }

  /// Requires exactly one route belonging to the actual publishing package.
  ChatUiAction _resolveAction(
    String owner,
    Object? routeId,
    Object? input,
    List<core_proxy.ToolPkgUiRoute> routes,
  ) {
    final id = _identity(routeId, 'routeId');
    final matches = routes.where(
      (route) => route.containerPackageName == owner && route.routeId == id,
    );
    if (matches.length != 1) {
      throw StateError('Unregistered or ambiguous owner route: $owner/$id');
    }
    return ChatUiAction(route: matches.single, input: input);
  }

  /// Requires a JSON object without guessing field aliases or serializing DTOs.
  Map<String, Object?> _object(Object? value, String path) {
    if (value is! Map || value.keys.any((key) => key is! String)) {
      throw FormatException('$path must be an object.');
    }
    return value.map((key, value) => MapEntry(key as String, value));
  }

  /// Requires the exact documented display fields, including nullable values.
  void _fields(Map<String, Object?> data, Set<String> keys, String path) {
    if (data.length != keys.length || !keys.every(data.containsKey)) {
      throw FormatException('$path has invalid fields.');
    }
  }

  /// Requires a JSON array without synthesizing absent registry data.
  List<Object?> _list(Object? value, String path) {
    if (value is! List) throw FormatException('$path must be an array.');
    return List<Object?>.of(value);
  }

  /// Requires display text while preserving the plugin's exact spelling.
  String _text(Object? value, String path) {
    if (value is! String) throw FormatException('$path must be text.');
    return value;
  }

  /// Requires an explicit nullable display value rather than an absent field.
  String? _nullableText(Object? value, String path) =>
      value == null ? null : _text(value, path);

  /// Rejects blank registered identities without normalizing them.
  String _identity(Object? value, String path) {
    final text = _text(value, path);
    if (text.trim().isEmpty) throw FormatException('$path must not be blank.');
    return text;
  }
}
