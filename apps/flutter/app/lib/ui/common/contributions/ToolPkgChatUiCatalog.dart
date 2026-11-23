// ignore_for_file: file_names

import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../features/chat/components/workspace/WorkspaceOverviewModels.dart';
import 'ChatModelSelection.dart';

/// Reads registered display contracts without retaining plugin business records.
class ToolPkgChatUiCatalog {
  /// Binds discovery to the actual receiving runtime and host display language.
  const ToolPkgChatUiCatalog({required this.clients, this.useEnglish = false});

  final GeneratedCoreProxyClients clients;
  final bool useEnglish;

  /// Reads model rules from the actual execution configuration owner for this chat.
  Future<ChatModelSelection> loadModelSelection({
    required String? chatId,
  }) async {
    final global = chatId == null
        ? await clients.preferencesFunctionalConfigManager
              .getModelBindingForFunction(
                functionType: core_proxy.FunctionType.chat,
              )
        : await clients.chatRuntimeHolderMain.chatModelBinding(chatId: chatId);
    final owners = await _owners('chat.configuration.resolve');
    if (owners.isEmpty) {
      return ChatModelSelection(globalBinding: global, participants: const []);
    }
    if (owners.length != 1) {
      throw StateError('Multiple chat configuration owners.');
    }
    final owner = owners.single;
    const api = 'chat.model.selection';
    final apis = await clients.application
        .packageManager()
        .getToolPkgPublicApis(packageName: owner);
    if (!apis.contains(api)) {
      throw StateError('Chat configuration owner does not publish $api.');
    }
    final data = _object(
      await _invoke(owner, api, <String, Object?>{
        'chatId': chatId,
        'defaultModelBinding': <String, Object?>{
          'providerId': global.providerId,
          'modelId': global.modelId,
        },
      }),
      'chat model selection',
    );
    _fields(data, const {'participants'}, 'chat model selection');
    final participants = <ChatParticipantModel>[];
    final ids = <String>{};
    for (final raw in _list(data['participants'], 'participants')) {
      final participant = _object(raw, 'participant model');
      _fields(participant, const {
        'id',
        'name',
        'binding',
        'followsGlobal',
      }, 'participant model');
      final id = _identity(participant['id'], 'participant.id');
      if (!ids.add(id)) {
        throw const FormatException('Duplicate model participant.');
      }
      final followsGlobal = participant['followsGlobal'];
      if (followsGlobal is! bool) {
        throw const FormatException('followsGlobal must be a boolean.');
      }
      final binding = _object(participant['binding'], 'model binding');
      _fields(binding, const {'providerId', 'modelId'}, 'model binding');
      final providerId = _identity(binding['providerId'], 'providerId');
      final modelId = _identity(binding['modelId'], 'modelId');
      if (followsGlobal &&
          (providerId != global.providerId || modelId != global.modelId)) {
        throw const FormatException(
          'Global participant has a different model binding.',
        );
      }
      participants.add(
        ChatParticipantModel(
          id: id,
          name: _identity(participant['name'], 'participant.name'),
          followsGlobal: followsGlobal,
          binding: core_proxy.FunctionModelBinding(
            providerId: providerId,
            modelId: modelId,
            followsChat: false,
          ),
        ),
      );
    }
    if (participants.isEmpty) {
      throw const FormatException('Chat model selection has no participants.');
    }
    return ChatModelSelection(
      globalBinding: global,
      participants: List.unmodifiable(participants),
    );
  }

  /// Resolves selector, identity, and background display data from actual owners.
  Future<
    ({
      core_proxy.ChatDisplayIdentity? identity,
      ({core_proxy.ToolPkgUiRoute route, Object? input})? avatarAction,
      String? backgroundUri,
    })
  >
  loadContext({required String? chatId}) async {
    const api = 'chat.context.actions';
    final owners = await _owners(api);
    final routes = await _routes();
    core_proxy.ChatDisplayIdentity? identity;
    ({core_proxy.ToolPkgUiRoute route, Object? input})? resolvedAvatarAction;
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
        _fields(display, <String>{
          'title',
          'avatarUri',
          if (display.containsKey('avatarAction')) 'avatarAction',
          // Older producers used action for the same avatar preview route.
          if (display.containsKey('action')) 'action',
        }, 'identity');
        if (display['avatarAction'] != null && display['action'] != null) {
          throw StateError(
            'Identity cannot publish two avatar preview actions.',
          );
        }
        final avatarAction = display['avatarAction'] ?? display['action'];
        identity = core_proxy.ChatDisplayIdentity.fromJson(display);
        resolvedAvatarAction = avatarAction == null
            ? null
            : _action(avatarAction, owner, routes);
      }
      final background = _nullableText(data['backgroundUri'], 'backgroundUri');
      if (background != null) {
        if (backgroundUri != null) {
          throw StateError('Multiple API owners published a chat background.');
        }
        backgroundUri = background;
      }
    }
    return (
      identity: identity,
      avatarAction: resolvedAvatarAction,
      backgroundUri: backgroundUri,
    );
  }

  /// Reads owner sections directly into the workspace's display records.
  Future<List<WorkspaceContributionUsage>> loadWorkspaceUsages({
    required List<core_proxy.ChatHistoryListItem> chats,
  }) async {
    const api = 'chat.list.sections';
    final owners = await _owners(api);
    final routes = await _routes();
    final knownChatIds = chats.map((chat) => chat.id).toSet();
    final sections = <WorkspaceContributionUsage>[];
    final payload = <String, Object?>{
      'chats': chats
          .map(
            (chat) => chat.toJson()
              ..remove('group')
              ..remove('parentChatId'),
          )
          .toList(growable: false),
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
        final preview = _action(section['preview'], owner, routes);
        sections.add(
          WorkspaceContributionUsage(
            ownerPackageName: owner,
            sectionId: id,
            title: _text(section['title'], 'section.title'),
            avatarUri: _nullableText(section['avatarUri'], 'section.avatarUri'),
            conversationCount: chatIds.length,
            previewRoute: preview.route,
            previewInput: preview.input,
          ),
        );
      }
    }
    return List<WorkspaceContributionUsage>.unmodifiable(sections);
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
  ({core_proxy.ToolPkgUiRoute route, Object? input}) _action(
    Object? raw,
    String owner,
    List<core_proxy.ToolPkgUiRoute> routes,
  ) {
    final data = _object(raw, 'presentation action');
    _fields(data, const <String>{'routeId', 'input'}, 'presentation action');
    return _resolveAction(owner, data['routeId'], data['input'], routes);
  }

  /// Requires exactly one route belonging to the actual publishing package.
  ({core_proxy.ToolPkgUiRoute route, Object? input}) _resolveAction(
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
    return (route: matches.single, input: input);
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
