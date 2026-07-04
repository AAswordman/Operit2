// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/link_host/LinkHostServer.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../core/runtime/RuntimeConnectionManager.dart';
import '../../l10n/generated/app_localizations.dart';
import '../features/settings/models/SettingsModels.dart';
import '../theme/OperitTheme.dart';
import 'navigation/AppNavigationModels.dart';
import 'screens/OperitMainScreen.dart';
import 'screens/OperitScreens.dart';
import 'screens/ScreenRouteRegistry.dart';

class OperitApp extends StatelessWidget {
  const OperitApp({super.key, this.startupWebAccessError});

  final String? startupWebAccessError;

  @override
  Widget build(BuildContext context) {
    return OperitTheme(
      child: _AppDialogHost(
        startupWebAccessError: startupWebAccessError,
        child: const _AiSetupGuideGate(child: OperitMainScreen()),
      ),
    );
  }
}

class _AiSetupGuideGate extends StatefulWidget {
  const _AiSetupGuideGate({required this.child});

  final Widget child;

  @override
  State<_AiSetupGuideGate> createState() => _AiSetupGuideGateState();
}

class _AiSetupGuideGateState extends State<_AiSetupGuideGate> {
  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(),
  );
  static const String _preferencesFileName = 'onboarding_preferences';
  static const String _guideSeenKey = 'ai_setup_guide_seen';

  late Future<_AiSetupGuideDecision> _decisionFuture;
  bool _hiddenInSession = false;

  @override
  void initState() {
    super.initState();
    _decisionFuture = _loadDecision();
  }

  Future<_AiSetupGuideDecision> _loadDecision() async {
    final configured = await _hasConfiguredChatModel();
    final guideSeen = await _readGuideSeen();
    return _AiSetupGuideDecision(
      showGuide: !configured && !guideSeen,
    );
  }

  Future<bool> _readGuideSeen() async {
    final value = await _clients.preferencesPreferenceStorageManager
        .getPreference(fileName: _preferencesFileName, key: _guideSeenKey);
    if (value == null) {
      return false;
    }
    return switch (value) {
      'true' => true,
      'false' => false,
      _ => throw FormatException('invalid ai setup guide flag: $value'),
    };
  }

  Future<void> _markGuideSeen() {
    return _clients.preferencesPreferenceStorageManager.setPreference(
      fileName: _preferencesFileName,
      key: _guideSeenKey,
      value: 'true',
    );
  }

  Future<bool> _hasConfiguredChatModel() async {
    final modelManager = _clients.preferencesModelConfigManager;
    final functionManager = _clients.preferencesFunctionalConfigManager;
    await modelManager.initializeIfNeeded();
    await functionManager.initializeIfNeeded();
    final chatBinding = await functionManager.getModelBindingForFunction(
      functionType: core_proxy.FunctionType.chat,
    );
    final providers = await modelManager.getProviderProfiles();

    core_proxy.ProviderProfile? boundProvider;
    for (final provider in providers) {
      if (provider.id == chatBinding.providerId) {
        boundProvider = provider;
        break;
      }
    }
    if (boundProvider == null) {
      return false;
    }

    var boundModelExists = false;
    for (final model in boundProvider.models) {
      if (model.id == chatBinding.modelId) {
        boundModelExists = true;
        break;
      }
    }
    if (!boundModelExists) {
      return false;
    }

    if (boundProvider.endpoint.trim().isEmpty) {
      return false;
    }
    return _providerHasApiKey(boundProvider);
  }

  bool _providerHasApiKey(core_proxy.ProviderProfile provider) {
    if (provider.apiKey.trim().isNotEmpty) {
      return true;
    }
    if (!provider.useMultipleApiKeys) {
      return false;
    }
    for (final key in provider.apiKeyPool) {
      if (key is String && key.trim().isNotEmpty) {
        return true;
      }
    }
    return false;
  }

  Future<void> _skipGuide() async {
    await _markGuideSeen();
    if (!mounted) {
      return;
    }
    setState(() {
      _hiddenInSession = true;
    });
  }

  Future<void> _openModelSettings() async {
    await _markGuideSeen();
    if (!mounted) {
      return;
    }
    setState(() {
      _hiddenInSession = true;
    });
    final entry = ScreenRouteRegistry.toEntry(
      screen: const SettingsScreenRoute(category: SettingsCategory.model),
    );
    AppRouterGateway.navigate(
      routeId: entry.routeId,
      args: entry.args,
      source: entry.source,
    );
  }

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: <Widget>[
        widget.child,
        FutureBuilder<_AiSetupGuideDecision>(
          future: _decisionFuture,
          builder: (context, snapshot) {
            if (snapshot.hasError) {
              Error.throwWithStackTrace(snapshot.error!, snapshot.stackTrace!);
            }
            final decision = snapshot.data;
            if (_hiddenInSession || decision == null || !decision.showGuide) {
              return const SizedBox.shrink();
            }
            return Positioned.fill(
              child: _AiSetupGuidePage(
                onConfigure: _openModelSettings,
                onSkip: _skipGuide,
              ),
            );
          },
        ),
      ],
    );
  }
}

class _AiSetupGuideDecision {
  const _AiSetupGuideDecision({required this.showGuide});

  final bool showGuide;
}

class _AiSetupGuidePage extends StatelessWidget {
  const _AiSetupGuidePage({
    required this.onConfigure,
    required this.onSkip,
  });

  final VoidCallback onConfigure;
  final VoidCallback onSkip;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Material(
      color: colorScheme.surface,
      child: SafeArea(
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 720),
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Card(
                elevation: 0,
                color: colorScheme.surfaceContainerHighest,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(28),
                  side: BorderSide(color: colorScheme.outlineVariant),
                ),
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(28, 28, 28, 24),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Container(
                        width: 64,
                        height: 64,
                        decoration: BoxDecoration(
                          color: colorScheme.primaryContainer,
                          borderRadius: BorderRadius.circular(22),
                        ),
                        child: Icon(
                          Icons.auto_awesome,
                          color: colorScheme.onPrimaryContainer,
                          size: 34,
                        ),
                      ),
                      const SizedBox(height: 24),
                      Text(
                        '先配置 AI 接口',
                        style: textTheme.headlineSmall?.copyWith(
                          fontWeight: FontWeight.w800,
                          color: colorScheme.onSurface,
                        ),
                      ),
                      const SizedBox(height: 12),
                      Text(
                        'Operit2 的聊天、总结和智能工具需要你配置自己的模型服务。市场、本地插件和设置可以先正常使用，但开始对话前需要填入 API Endpoint、API Key，并选择默认聊天模型。',
                        style: textTheme.bodyLarge?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                          height: 1.45,
                        ),
                      ),
                      const SizedBox(height: 24),
                      _AiSetupGuideStep(
                        icon: Icons.key_outlined,
                        title: '填写 API Key',
                        description: '默认会准备 DeepSeek 配置，你只需要补上自己的 Key。',
                      ),
                      const SizedBox(height: 12),
                      _AiSetupGuideStep(
                        icon: Icons.hub_outlined,
                        title: '选择聊天模型',
                        description: '配置完成后，将模型设为聊天默认模型即可开始使用。',
                      ),
                      const SizedBox(height: 12),
                      _AiSetupGuideStep(
                        icon: Icons.verified_outlined,
                        title: '建议测试连接',
                        description: '测试通过后，应用会按结果更新模型能力开关。',
                      ),
                      const SizedBox(height: 28),
                      Row(
                        children: <Widget>[
                          TextButton(
                            onPressed: onSkip,
                            child: const Text('稍后再说'),
                          ),
                          const Spacer(),
                          FilledButton.icon(
                            onPressed: onConfigure,
                            icon: const Icon(Icons.settings_outlined),
                            label: const Text('去配置模型'),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _AiSetupGuideStep extends StatelessWidget {
  const _AiSetupGuideStep({
    required this.icon,
    required this.title,
    required this.description,
  });

  final IconData icon;
  final String title;
  final String description;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Icon(icon, color: colorScheme.primary, size: 22),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                title,
                style: textTheme.titleSmall?.copyWith(
                  fontWeight: FontWeight.w700,
                  color: colorScheme.onSurface,
                ),
              ),
              const SizedBox(height: 2),
              Text(
                description,
                style: textTheme.bodyMedium?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                  height: 1.35,
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _AppDialogHost extends StatefulWidget {
  const _AppDialogHost({
    required this.startupWebAccessError,
    required this.child,
  });

  final String? startupWebAccessError;
  final Widget child;

  @override
  State<_AppDialogHost> createState() => _AppDialogHostState();
}

class _AppDialogHostState extends State<_AppDialogHost> {
  bool _shownStartupWebAccessError = false;
  String _shownPairingId = '';

  @override
  void initState() {
    super.initState();
    LinkHostServer.instance.addListener(_onWebAccessChanged);
    RuntimeConnectionManager.instance.addListener(_onManagerChanged);
  }

  @override
  void dispose() {
    LinkHostServer.instance.removeListener(_onWebAccessChanged);
    RuntimeConnectionManager.instance.removeListener(_onManagerChanged);
    super.dispose();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _showStartupWebAccessError();
    _showPendingRemoteError();
  }

  void _showStartupWebAccessError() {
    final error = widget.startupWebAccessError;
    if (_shownStartupWebAccessError || error == null) {
      return;
    }
    _shownStartupWebAccessError = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      final l10n = AppLocalizations.of(context)!;
      showDialog<void>(
        context: context,
        builder: (context) {
          return AlertDialog(
            title: Text(l10n.settingsWebAccessService),
            content: SingleChildScrollView(
              child: SelectableText(l10n.settingsWebAccessStartFailed(error)),
            ),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: Text(l10n.ok),
              ),
            ],
          );
        },
      );
    });
  }

  void _onWebAccessChanged() {
    final record = LinkHostServer.instance.lastPairingCode;
    if (record == null || record.pairingId == _shownPairingId) {
      return;
    }
    _shownPairingId = record.pairingId;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      final l10n = AppLocalizations.of(context)!;
      showDialog<void>(
        context: context,
        builder: (context) {
          return AlertDialog(
            title: Text(l10n.settingsWebAccessPairingRequest),
            content: SelectableText(
              l10n.settingsWebAccessPairingRequestMessage(
                record.pairingCode,
                record.clientDeviceId,
              ),
            ),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: Text(l10n.ok),
              ),
            ],
          );
        },
      );
    });
  }

  void _onManagerChanged() {
    _showPendingRemoteError();
  }

  void _showPendingRemoteError() {
    final error = RuntimeConnectionManager.instance.consumePendingRemoteError();
    if (error == null || !mounted) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final l10n = AppLocalizations.of(context)!;
      showDialog<void>(
        context: context,
        builder: (context) {
          return AlertDialog(
            title: Text(l10n.settingsRuntimeRemoteDisconnected),
            content: SingleChildScrollView(
              child: SelectableText(
                l10n.settingsRuntimeRemoteDisconnectedMessage(error.toString()),
              ),
            ),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: Text(l10n.ok),
              ),
            ],
          );
        },
      );
    });
  }

  @override
  Widget build(BuildContext context) {
    return widget.child;
  }
}
