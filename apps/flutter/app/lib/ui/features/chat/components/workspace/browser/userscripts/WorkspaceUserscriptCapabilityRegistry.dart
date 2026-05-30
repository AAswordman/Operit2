// ignore_for_file: file_names

class WorkspaceUserscriptCapability {
  const WorkspaceUserscriptCapability({
    required this.canonicalGrant,
    this.aliases = const <String>{},
    this.highRisk = false,
    this.requiresDocumentStartInjection = false,
    this.requiresRequestInterception = false,
  });

  final String canonicalGrant;
  final Set<String> aliases;
  final bool highRisk;
  final bool requiresDocumentStartInjection;
  final bool requiresRequestInterception;
}

class WorkspaceUserscriptCapabilityRegistry {
  WorkspaceUserscriptCapabilityRegistry._();

  static const List<WorkspaceUserscriptCapability> capabilities =
      <WorkspaceUserscriptCapability>[
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.addElement',
          aliases: <String>{'GM_addElement'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.addStyle',
          aliases: <String>{'GM_addStyle'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.deleteValue',
          aliases: <String>{'GM_deleteValue'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.deleteValues',
          aliases: <String>{'GM_deleteValues'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.download',
          aliases: <String>{'GM_download'},
          highRisk: true,
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.getResourceText',
          aliases: <String>{'GM_getResourceText'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.getResourceURL',
          aliases: <String>{'GM_getResourceURL'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.getTab',
          aliases: <String>{'GM_getTab'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.getTabs',
          aliases: <String>{'GM_getTabs'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.getValue',
          aliases: <String>{'GM_getValue'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.getValues',
          aliases: <String>{'GM_getValues'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.info',
          aliases: <String>{'GM_info'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.listValues',
          aliases: <String>{'GM_listValues'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.log',
          aliases: <String>{'GM_log'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.notification',
          aliases: <String>{'GM_notification'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.openInTab',
          aliases: <String>{'GM_openInTab'},
          highRisk: true,
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.registerMenuCommand',
          aliases: <String>{'GM_registerMenuCommand'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.addValueChangeListener',
          aliases: <String>{'GM_addValueChangeListener'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.removeValueChangeListener',
          aliases: <String>{'GM_removeValueChangeListener'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.saveTab',
          aliases: <String>{'GM_saveTab'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.setClipboard',
          aliases: <String>{'GM_setClipboard'},
          highRisk: true,
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.setValue',
          aliases: <String>{'GM_setValue'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.setValues',
          aliases: <String>{'GM_setValues'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.unregisterMenuCommand',
          aliases: <String>{'GM_unregisterMenuCommand'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.webRequest',
          aliases: <String>{'GM_webRequest'},
          highRisk: true,
          requiresDocumentStartInjection: true,
          requiresRequestInterception: true,
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.xmlHttpRequest',
          aliases: <String>{'GM_xmlhttpRequest', 'GM_xmlHttpRequest'},
          highRisk: true,
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.audio',
          aliases: <String>{'GM_audio'},
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'GM.cookie',
          aliases: <String>{'GM_cookie'},
          highRisk: true,
        ),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'unsafeWindow',
          highRisk: true,
        ),
        WorkspaceUserscriptCapability(canonicalGrant: 'window.close'),
        WorkspaceUserscriptCapability(canonicalGrant: 'window.focus'),
        WorkspaceUserscriptCapability(
          canonicalGrant: 'window.onurlchange',
          requiresDocumentStartInjection: true,
        ),
        WorkspaceUserscriptCapability(canonicalGrant: 'none'),
      ];

  static final Map<String, String> _aliasToCanonical = <String, String>{
    for (final capability in capabilities) ...<String, String>{
      capability.canonicalGrant: capability.canonicalGrant,
      for (final alias in capability.aliases) alias: capability.canonicalGrant,
    },
  };

  static final Map<String, WorkspaceUserscriptCapability>
  _capabilityByCanonical = <String, WorkspaceUserscriptCapability>{
    for (final capability in capabilities)
      capability.canonicalGrant: capability,
  };

  static String? canonicalGrant(String rawGrant) {
    return _aliasToCanonical[rawGrant.trim()];
  }

  static bool isGrantKnown(String rawGrant) {
    return canonicalGrant(rawGrant) != null;
  }

  static List<String> knownGrants(List<String> grants) {
    return grants
        .map(canonicalGrant)
        .whereType<String>()
        .toSet()
        .toList(growable: false);
  }

  static List<String> unknownGrants(List<String> grants) {
    return grants
        .map((grant) => grant.trim())
        .where((grant) => grant.isNotEmpty && !isGrantKnown(grant))
        .toSet()
        .toList(growable: false);
  }

  static List<String> warningReasons(List<String> grants) {
    final canonicalGrants = knownGrants(grants);
    final reasons = <String>[];
    final unknown = unknownGrants(grants);
    if (unknown.isNotEmpty) {
      reasons.add('未知 grant：${unknown.join(', ')}');
    }
    if (canonicalGrants.contains('none') && canonicalGrants.length > 1) {
      reasons.add('@grant none 不能和其他 grant 同时使用');
    }
    final highRisk = canonicalGrants
        .where((grant) => _capabilityByCanonical[grant]?.highRisk == true)
        .toList(growable: false);
    if (highRisk.isNotEmpty) {
      reasons.add('高危能力：${highRisk.join(', ')}');
    }
    return reasons;
  }
}
