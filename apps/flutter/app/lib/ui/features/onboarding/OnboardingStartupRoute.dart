// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;
import 'dart:ui' show lerpDouble;

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../common/OperitLogoMark.dart';
import '../../common/components/CommonNetworkErrorView.dart';
import '../../main/navigation/StartupRouteStrategy.dart';

void registerOnboardingStartupRoute(StartupRouteRegistry registry) {
  registry.register(const OnboardingStartupRouteStrategy());
}

class OnboardingStartupRouteStrategy extends StartupRouteStrategy {
  const OnboardingStartupRouteStrategy();

  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(),
  );
  static const String _preferencesFileName = 'onboarding_preferences';
  static const String _guideSeenKey = 'ai_setup_guide_seen';

  @override
  Future<StartupRouteDecision?> resolve() async {
    final configured = await _hasConfiguredChatModel();
    final guideSeen = await _readGuideSeen();
    if (configured || guideSeen) {
      return null;
    }
    return StartupRouteDecision(
      builder: (context, complete) => _AiSetupGuidePage(
        clients: _clients,
        onComplete: () => _finishGuide(complete),
        onSkip: () => _finishGuide(complete),
      ),
    );
  }

  static Future<void> _finishGuide(StartupRouteCompleteCallback complete) async {
    await _markGuideSeen();
    complete();
  }

  static Future<bool> _readGuideSeen() async {
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

  static Future<void> _markGuideSeen() {
    return _clients.preferencesPreferenceStorageManager.setPreference(
      fileName: _preferencesFileName,
      key: _guideSeenKey,
      value: 'true',
    );
  }

  static Future<bool> _hasConfiguredChatModel() async {
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

  static bool _providerHasApiKey(core_proxy.ProviderProfile provider) {
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
}

class _AiSetupGuidePage extends StatefulWidget {
  const _AiSetupGuidePage({
    required this.clients,
    required this.onComplete,
    required this.onSkip,
  });

  final GeneratedCoreProxyClients clients;
  final Future<void> Function() onComplete;
  final Future<void> Function() onSkip;

  @override
  State<_AiSetupGuidePage> createState() => _AiSetupGuidePageState();
}

class _AiSetupGuidePageState extends State<_AiSetupGuidePage>
    with TickerProviderStateMixin, WidgetsBindingObserver {
  final PageController _pageController = PageController();
  final GlobalKey<FormState> _modelFormKey = GlobalKey<FormState>();
  final TextEditingController _endpointController = TextEditingController();
  final TextEditingController _apiKeyController = TextEditingController();
  int _currentPage = 0;
  bool _savingModel = false;
  bool _loadingModels = false;
  bool _readingOperit1Snapshot = false;
  bool _importingOperit1Snapshot = false;
  bool _requestingPermission = false;
  StreamSubscription<core_proxy.Operit1SnapshotImportProgress>?
      _operit1ImportProgressSubscription;
  String? _selectedProviderTypeId;
  String? _configuredProviderId;
  String? _selectedModelId;
  _AiSetupStartMode? _selectedStartMode;
  core_proxy.Operit1SnapshotPreview? _operit1Snapshot;
  core_proxy.Operit1SnapshotImportProgress? _operit1ImportProgress;
  String? _operit1SnapshotPath;
  String? _operit1SnapshotFileName;
  List<core_proxy.ProviderCatalogEntry> _catalogEntries =
      const <core_proxy.ProviderCatalogEntry>[];
  List<core_proxy.AvailableProviderModel> _availableModels =
      const <core_proxy.AvailableProviderModel>[];
  String? _setupError;
  bool _providerConfirmed = false;
  _OnboardingPermissionSnapshot _permissions =
      const _OnboardingPermissionSnapshot(
    location: false,
    bluetoothConnect: false,
    bluetoothScan: false,
    overlay: false,
    batteryOptimization: false,
  );

  late final AnimationController _introAnimationController;
  late final AnimationController _introExitController;

  static const int _introPageIndex = 0;
  static const int _modePageIndex = 1;
  static const int _modelPageIndex = 2;
  static const int _importPageIndex = 3;
  static const int _permissionPageIndex = 4;
  static const String _defaultProviderId = 'DEEPSEEK';

  int get _pageCount => 5;
  bool get _isModePage => _currentPage == _modePageIndex;
  bool get _isModelPage => _currentPage == _modelPageIndex;
  bool get _isImportPage => _currentPage == _importPageIndex;
  bool get _isPermissionPage => _currentPage == _permissionPageIndex;

  core_proxy.ProviderCatalogEntry get _selectedCatalog {
    for (final entry in _catalogEntries) {
      if (entry.providerTypeId == _selectedProviderTypeId) {
        return entry;
      }
    }
    throw StateError('selected provider type is not in catalog');
  }

  core_proxy.ProviderCatalogEntry _deepseekCatalog(
    List<core_proxy.ProviderCatalogEntry> entries,
  ) {
    for (final entry in entries) {
      if (entry.providerTypeId == 'DEEPSEEK') {
        return entry;
      }
    }
    throw StateError('DEEPSEEK provider type is not in catalog');
  }

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _introAnimationController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 3000),
    )..forward();
    _introExitController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 760),
    );
    _subscribeOperit1ImportProgress();
    _loadSetupData();
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _operit1ImportProgressSubscription?.cancel();
    _introAnimationController.dispose();
    _introExitController.dispose();
    _pageController.dispose();
    _endpointController.dispose();
    _apiKeyController.dispose();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed && _isPermissionPage) {
      _refreshPermissionSnapshot();
    }
  }

  Future<void> _loadSetupData() async {
    try {
      final modelManager = widget.clients.preferencesModelConfigManager;
      await modelManager.initializeIfNeeded();
      final entries = await modelManager.getProviderCatalogEntries();
      await _refreshPermissionSnapshot();
      if (!mounted) {
        return;
      }
      final defaultCatalog = _deepseekCatalog(entries);
      _applyCatalogDefaults(defaultCatalog);
      setState(() {
        _catalogEntries = entries;
        _selectedProviderTypeId = defaultCatalog.providerTypeId;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _setupError = '$error';
      });
    }
  }

  void _subscribeOperit1ImportProgress() {
    _operit1ImportProgressSubscription = widget
        .clients.application
        .operit1SnapshotImportProgressFlowChanges()
        .listen((progress) {
      if (!mounted) {
        return;
      }
      setState(() {
        _operit1ImportProgress = progress;
      });
    }, onError: (Object error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _setupError = '$error';
      });
    });
  }

  void _applyCatalogDefaults(core_proxy.ProviderCatalogEntry entry) {
    _endpointController.text = entry.defaultEndpoint;
  }

  void _selectProviderType(String? providerTypeId) {
    if (providerTypeId == null) {
      return;
    }
    for (final entry in _catalogEntries) {
      if (entry.providerTypeId == providerTypeId) {
        setState(() {
          _selectedProviderTypeId = providerTypeId;
          _providerConfirmed = false;
          _configuredProviderId = null;
          _selectedModelId = null;
          _availableModels = const <core_proxy.AvailableProviderModel>[];
          _applyCatalogDefaults(entry);
        });
        return;
      }
    }
  }

  Future<void> _goToPreviousPage() async {
    if (_currentPage == 0) {
      return;
    }
    if (_currentPage == _modePageIndex) {
      await _returnToIntro();
      return;
    }
    if (_currentPage == _importPageIndex) {
      await _animateToPage(_modePageIndex);
      return;
    }
    if (_currentPage == _permissionPageIndex &&
        _selectedStartMode == _AiSetupStartMode.quickStart) {
      await _animateToPage(_modelPageIndex);
      return;
    }
    if (_currentPage == _permissionPageIndex &&
        _selectedStartMode == _AiSetupStartMode.operit1Import) {
      await _animateToPage(_importPageIndex);
      return;
    }
    await _pageController.previousPage(
      duration: const Duration(milliseconds: 420),
      curve: Curves.easeOutQuart,
    );
  }

  Future<void> _goToNextPage() async {
    if (_currentPage == _introPageIndex) {
      await _advanceFromIntro();
      return;
    }
    if (_isModePage) {
      if (_selectedStartMode == _AiSetupStartMode.quickStart) {
        await _animateToPage(_modelPageIndex);
      } else if (_selectedStartMode == _AiSetupStartMode.operit1Import) {
        await _animateToPage(_importPageIndex);
      }
      return;
    }
    if (_isModelPage) {
      await _saveModelSetup();
      return;
    }
    if (_isImportPage) {
      await _saveOperit1Import();
      return;
    }
    if (_isPermissionPage) {
      await widget.onComplete();
      return;
    }
    _pageController.nextPage(
      duration: const Duration(milliseconds: 460),
      curve: Curves.easeOutQuart,
    );
  }

  Future<void> _animateToPage(int pageIndex) {
    return _pageController.animateToPage(
      pageIndex,
      duration: const Duration(milliseconds: 460),
      curve: Curves.easeOutQuart,
    );
  }

  Future<void> _advanceFromIntro() async {
    if (_introExitController.isAnimating) {
      return;
    }
    await _introExitController.forward();
    if (!mounted) {
      return;
    }
    await _pageController.nextPage(
      duration: const Duration(milliseconds: 520),
      curve: Curves.easeOutQuart,
    );
    if (!mounted) {
      return;
    }
    _introExitController.value = 1;
  }

  Future<void> _returnToIntro() async {
    if (_introExitController.isAnimating) {
      return;
    }
    await _pageController.previousPage(
      duration: const Duration(milliseconds: 260),
      curve: Curves.easeOutCubic,
    );
    if (!mounted) {
      return;
    }
    await _introExitController.reverse();
  }

  Future<void> _loadAvailableModels() async {
    final formState = _modelFormKey.currentState;
    if (formState == null) {
      throw StateError('model setup form state is not ready');
    }
    if (!formState.validate()) {
      return;
    }
    final catalog = _selectedCatalog;
    setState(() {
      _loadingModels = true;
      _setupError = null;
      _selectedModelId = null;
      _availableModels = const <core_proxy.AvailableProviderModel>[];
    });
    try {
      final modelManager = widget.clients.preferencesModelConfigManager;
      await modelManager.initializeIfNeeded();
      const providerId = _defaultProviderId;
      final provider = await modelManager.getProviderProfile(
        providerId: providerId,
      );
      await modelManager.updateProviderProfile(
        provider: core_proxy.ProviderProfile(
          id: provider.id,
          name: catalog.displayName.trim(),
          providerTypeId: catalog.providerTypeId,
          providerType: core_proxy.ApiProviderType.fromJson(
            catalog.providerTypeId,
          ),
          endpoint: _endpointController.text.trim(),
          apiKey: _apiKeyController.text.trim(),
          useMultipleApiKeys: provider.useMultipleApiKeys,
          apiKeyPool: provider.apiKeyPool,
          currentKeyIndex: provider.currentKeyIndex,
          keyRotationMode: provider.keyRotationMode,
          customHeaders: provider.customHeaders,
          requestLimitPerMinute: provider.requestLimitPerMinute,
          maxConcurrentRequests: provider.maxConcurrentRequests,
          models: provider.models,
        ),
      );
      final models = await modelManager.getAvailableProviderModels(
        providerId: providerId,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _configuredProviderId = providerId;
        _availableModels = models;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _setupError = '$error';
      });
    } finally {
      if (mounted) {
        setState(() {
          _loadingModels = false;
        });
      }
    }
  }

  Future<void> _saveModelSetup() async {
    final formState = _modelFormKey.currentState;
    if (formState == null) {
      throw StateError('model setup form state is not ready');
    }
    if (!formState.validate()) {
      return;
    }
    final providerId = _configuredProviderId;
    final modelId = _selectedModelId;
    if (providerId == null || providerId.isEmpty) {
      setState(() {
        _setupError = '请先拉取可用模型';
      });
      return;
    }
    if (modelId == null || modelId.isEmpty) {
      setState(() {
        _setupError = '请选择默认模型';
      });
      return;
    }
    setState(() {
      _savingModel = true;
      _setupError = null;
    });
    try {
      final modelManager = widget.clients.preferencesModelConfigManager;
      final functionManager = widget.clients.preferencesFunctionalConfigManager;
      await modelManager.initializeIfNeeded();
      await functionManager.initializeIfNeeded();
      final provider = await modelManager.getProviderProfile(
        providerId: providerId,
      );
      var selectedModelExists = false;
      for (final model in provider.models) {
        if (model.id == modelId) {
          selectedModelExists = true;
          break;
        }
      }
      if (!selectedModelExists) {
        await modelManager.addProviderModelFromAvailable(
          providerId: providerId,
          modelId: modelId,
        );
      }
      await functionManager.setModelForFunction(
        functionType: core_proxy.FunctionType.chat,
        providerId: providerId,
        modelId: modelId,
      );
      await _animateToPage(_permissionPageIndex);
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _setupError = '$error';
      });
    } finally {
      if (mounted) {
        setState(() {
          _savingModel = false;
        });
      }
    }
  }

  Future<void> _pickOperit1Snapshot() async {
    setState(() {
      _readingOperit1Snapshot = true;
      _setupError = null;
      _operit1Snapshot = null;
      _operit1ImportProgress = null;
      _operit1SnapshotPath = null;
      _operit1SnapshotFileName = null;
    });
    try {
      final picked = await _pickOperit1SnapshotFile();
      if (picked == null) {
        return;
      }
      final snapshot = await widget.clients.application.inspectOperit1SnapshotFile(
        path: picked.path,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _operit1Snapshot = snapshot;
        _operit1SnapshotPath = picked.path;
        _operit1SnapshotFileName = picked.name;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _setupError = '$error';
      });
    } finally {
      if (mounted) {
        setState(() {
          _readingOperit1Snapshot = false;
        });
      }
    }
  }

  Future<_PickedOperit1SnapshotFile?> _pickOperit1SnapshotFile() async {
    final result = await FilePicker.pickFiles(
      type: FileType.custom,
      allowedExtensions: const <String>['opsnapshot', 'zip'],
      allowMultiple: false,
      withData: false,
      withReadStream: false,
    );
    if (result == null || result.files.isEmpty) {
      return null;
    }
    final file = result.files.first;
    final path = file.path;
    if (path == null || path.isEmpty) {
      return null;
    }
    return _PickedOperit1SnapshotFile(path: path, name: file.name);
  }

  Future<void> _saveOperit1Import() async {
    final path = _operit1SnapshotPath;
    if (path == null || path.isEmpty) {
      setState(() {
        _setupError = '请选择 Operit1 快照文件';
      });
      return;
    }

    setState(() {
      _importingOperit1Snapshot = true;
      _setupError = null;
    });
    try {
      await widget.clients.application.importOperit1SnapshotFile(path: path);
      await _animateToPage(_permissionPageIndex);
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _setupError = '$error';
      });
    } finally {
      if (mounted) {
        setState(() {
          _importingOperit1Snapshot = false;
        });
      }
    }
  }

  Future<void> _refreshPermissionSnapshot() async {
    final snapshot = await _OnboardingPermissionBridge.snapshot();
    if (!mounted) {
      return;
    }
    setState(() {
      _permissions = snapshot;
    });
  }

  Future<void> _requestPermission(_OnboardingPermissionAction action) async {
    setState(() {
      _requestingPermission = true;
      _setupError = null;
    });
    try {
      await _OnboardingPermissionBridge.request(action);
      await _refreshPermissionSnapshot();
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _setupError = '$error';
      });
    } finally {
      if (mounted) {
        setState(() {
          _requestingPermission = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final modeReady = !_isModePage || _selectedStartMode != null;
    final modelReady = !_isModelPage ||
        (_providerConfirmed &&
            _configuredProviderId != null &&
            _configuredProviderId!.isNotEmpty &&
            _selectedModelId != null &&
            _selectedModelId!.isNotEmpty);
    final importReady = !_isImportPage || _operit1Snapshot != null;
    final canGoForward = !_savingModel &&
        !_loadingModels &&
        !_readingOperit1Snapshot &&
        !_importingOperit1Snapshot &&
        !_requestingPermission &&
        modeReady &&
        modelReady &&
        importReady;
    final introActive = _currentPage == _introPageIndex;
    final showChrome = !introActive;

    return Material(
      color: colorScheme.surface,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 420),
        curve: Curves.easeOutQuart,
        decoration: _backgroundDecoration(colorScheme, showChrome),
        child: SafeArea(
          child: Padding(
            padding: const EdgeInsets.fromLTRB(20, 16, 20, 18),
            child: LayoutBuilder(
              builder: (context, constraints) {
                final chromeReserved =
                    introActive ? _introExitController.value : 1.0;
                return Stack(
                  clipBehavior: Clip.none,
                  children: <Widget>[
                    Column(
                      children: <Widget>[
                        SizedBox(height: 46 * chromeReserved),
                        if (chromeReserved > 0) const SizedBox(height: 18),
                        Expanded(
                          child: PageView.builder(
                            controller: _pageController,
                            physics: const NeverScrollableScrollPhysics(),
                            onPageChanged: (index) {
                              setState(() {
                                _currentPage = index;
                              });
                            },
                            itemCount: _pageCount,
                            itemBuilder: (context, index) {
                              if (index == _introPageIndex) {
                                return _AiSetupIntroPage(
                                  animation: _introAnimationController,
                                  exitAnimation: _introExitController,
                                );
                              }
                              if (index == _modePageIndex) {
                                return _AiSetupModePage(
                                  selectedMode: _selectedStartMode,
                                  onModeChanged: (value) {
                                    setState(() {
                                      _selectedStartMode = value;
                                      _setupError = null;
                                    });
                                  },
                                );
                              }
                              if (index == _modelPageIndex) {
                                return _AiSetupModelPage(
                                  formKey: _modelFormKey,
                                  catalogEntries: _catalogEntries,
                                  selectedProviderTypeId:
                                      _selectedProviderTypeId,
                                  providerConfirmed: _providerConfirmed,
                                  endpointController: _endpointController,
                                  apiKeyController: _apiKeyController,
                                  availableModels: _availableModels,
                                  selectedModelId: _selectedModelId,
                                  loadingModels: _loadingModels,
                                  onProviderChanged: _selectProviderType,
                                  onProviderConfirmed: () {
                                    setState(() {
                                      _providerConfirmed = true;
                                      _setupError = null;
                                    });
                                  },
                                  onLoadModels: _loadAvailableModels,
                                  onModelChanged: (value) {
                                    setState(() {
                                      _selectedModelId = value;
                                    });
                                  },
                                  errorText: _setupError,
                                );
                              }
                              if (index == _importPageIndex) {
                                return _AiSetupImportPage(
                                  snapshot: _operit1Snapshot,
                                  fileName: _operit1SnapshotFileName,
                                  reading: _readingOperit1Snapshot,
                                  importing: _importingOperit1Snapshot,
                                  progress: _operit1ImportProgress,
                                  onPickSnapshot: _pickOperit1Snapshot,
                                  errorText: _setupError,
                                );
                              }
                              return _AiSetupPermissionPage(
                                permissions: _permissions,
                                requesting: _requestingPermission,
                                onRefresh: _refreshPermissionSnapshot,
                                onRequest: _requestPermission,
                                errorText: _setupError,
                              );
                            },
                          ),
                        ),
                        SizedBox(height: 56 * chromeReserved),
                      ],
                    ),
                    _AiSetupSharedChrome(
                      introActive: introActive,
                      introAnimation: _introAnimationController,
                      exitAnimation: _introExitController,
                      constraints: constraints,
                      canGoForward: canGoForward,
                      currentPage: _currentPage,
                      pageCount: _pageCount,
                      progressLabel: _progressLabel,
                      primaryActionLabel: _primaryActionLabel,
                      isPermissionPage: _isPermissionPage,
                      onBack: _goToPreviousPage,
                      onPrimary: _primaryAction,
                      onSkip: () {
                        widget.onSkip();
                      },
                    ),
                  ],
                );
              },
            ),
          ),
        ),
      ),
    );
  }

  String get _primaryActionLabel {
    if (_isModePage) {
      return '继续';
    }
    if (_isModelPage) {
      if (_loadingModels) {
        return '拉取中';
      }
      return _savingModel ? '保存中' : '继续';
    }
    if (_isImportPage) {
      if (_readingOperit1Snapshot) {
        return '读取中';
      }
      return _importingOperit1Snapshot ? '导入中' : '继续';
    }
    if (_isPermissionPage) {
      return '完成';
    }
    return '继续';
  }

  String get _progressLabel {
    if (_isModePage) {
      return '启动方式';
    }
    if (_isModelPage) {
      return '模型配置';
    }
    if (_isImportPage) {
      return '导入配置';
    }
    if (_isPermissionPage) {
      return '开启权限';
    }
    return '欢迎';
  }

  VoidCallback get _primaryAction {
    return () {
      _goToNextPage();
    };
  }

  BoxDecoration _backgroundDecoration(
    ColorScheme colorScheme,
    bool showChrome,
  ) {
    if (showChrome) {
      return BoxDecoration(color: colorScheme.surface);
    }
    return BoxDecoration(
      gradient: LinearGradient(
        begin: Alignment.topCenter,
        end: Alignment.bottomCenter,
        colors: <Color>[
          Color.alphaBlend(
            colorScheme.primary.withValues(alpha: 0.08),
            colorScheme.surface,
          ),
          colorScheme.surface,
          Color.alphaBlend(
            colorScheme.tertiary.withValues(alpha: 0.06),
            colorScheme.surface,
          ),
        ],
        stops: const <double>[0, 0.52, 1],
      ),
    );
  }
}

class _AiSetupProgressPill extends StatelessWidget {
  const _AiSetupProgressPill({
    required this.currentPage,
    required this.pageCount,
    required this.color,
    required this.trackColor,
    required this.textColor,
    required this.label,
  });

  final int currentPage;
  final int pageCount;
  final Color color;
  final Color trackColor;
  final Color textColor;
  final String label;

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    return LayoutBuilder(
      builder: (context, constraints) {
        final compact = constraints.maxWidth < 150 || pageCount >= 5;
        final horizontalPadding = compact ? 12.0 : 14.0;
        final activeDotWidth = compact ? 18.0 : 22.0;
        final dotSize = compact ? 6.0 : 7.0;
        final dotGap = compact ? 4.0 : 6.0;
        final labelGap = compact ? 8.0 : 12.0;

        return Container(
          height: 40,
          padding: EdgeInsets.symmetric(horizontal: horizontalPadding),
          decoration: BoxDecoration(
            color: trackColor.withValues(alpha: 0.72),
            borderRadius: BorderRadius.circular(99),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.max,
            children: <Widget>[
              for (var index = 0; index < pageCount; index++) ...<Widget>[
                AnimatedContainer(
                  duration: const Duration(milliseconds: 220),
                  curve: Curves.easeOutCubic,
                  width: index == currentPage ? activeDotWidth : dotSize,
                  height: dotSize,
                  decoration: BoxDecoration(
                    color: index == currentPage
                        ? color
                        : textColor.withValues(alpha: 0.25),
                    borderRadius: BorderRadius.circular(99),
                  ),
                ),
                if (index != pageCount - 1) SizedBox(width: dotGap),
              ],
              SizedBox(width: labelGap),
              Flexible(
                child: Text(
                  label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.labelMedium?.copyWith(
                    color: textColor,
                    fontWeight: FontWeight.w800,
                    letterSpacing: 0,
                  ),
                ),
              ),
            ],
          ),
        );
      },
    );
  }
}

class _AiSetupSharedChrome extends StatelessWidget {
  const _AiSetupSharedChrome({
    required this.introActive,
    required this.introAnimation,
    required this.exitAnimation,
    required this.constraints,
    required this.canGoForward,
    required this.currentPage,
    required this.pageCount,
    required this.progressLabel,
    required this.primaryActionLabel,
    required this.isPermissionPage,
    required this.onBack,
    required this.onPrimary,
    required this.onSkip,
  });

  final bool introActive;
  final Animation<double> introAnimation;
  final Animation<double> exitAnimation;
  final BoxConstraints constraints;
  final bool canGoForward;
  final int currentPage;
  final int pageCount;
  final String progressLabel;
  final String primaryActionLabel;
  final bool isPermissionPage;
  final VoidCallback onBack;
  final VoidCallback onPrimary;
  final VoidCallback onSkip;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return AnimatedBuilder(
      animation: Listenable.merge(<Listenable>[introAnimation, exitAnimation]),
      builder: (context, child) {
        final introProgress = CurvedAnimation(
          parent: introAnimation,
          curve: const Interval(0, 0.68, curve: Curves.easeOutCubic),
        ).value;
        final exitProgress = CurvedAnimation(
          parent: exitAnimation,
          curve: Curves.easeInOutCubic,
        ).value;
        final loadingProgress = CurvedAnimation(
          parent: introAnimation,
          curve: const Interval(0.20, 0.58, curve: Curves.easeOutCubic),
        ).value;
        final loadingOpacity = introActive
            ? (math.sin(loadingProgress * math.pi) * (1 - exitProgress))
                .clamp(0.0, 1.0)
                .toDouble()
            : 0.0;
        final introLift = CurvedAnimation(
          parent: introAnimation,
          curve: const Interval(0.78, 0.92, curve: Curves.easeInOutCubic),
        ).value;
        final bodyProgress = CurvedAnimation(
          parent: introAnimation,
          curve: const Interval(0.92, 1, curve: Curves.easeOutCubic),
        ).value;
        final chromeProgress = introActive ? exitProgress : 1.0;
        final compact = constraints.maxHeight < 560;
        const chromeBarHeight = 40.0;
        final introLogoSize = compact ? 82.0 : 96.0;
        final introTitleFontSize = textTheme.headlineMedium?.fontSize ?? 28;
        final introScale = lerpDouble(0.96, 1, introProgress)!;
        final scaledIntroLogoSize = introLogoSize * introScale;
        final scaledIntroTitleFontSize = introTitleFontSize * introScale;
        final chromeTitleFontSize = textTheme.titleMedium?.fontSize ?? 16;
        final introBrandHeight =
            scaledIntroLogoSize + 18 + scaledIntroTitleFontSize * 1.18;
        final introBrandTop =
            (constraints.maxHeight - introBrandHeight) * 0.5 - 76 * introLift;
        final logoSize = lerpDouble(scaledIntroLogoSize, 28, chromeProgress)!;
        final chromeLogoTop = (chromeBarHeight - 28) * 0.5;
        final logoTop = lerpDouble(
          introBrandTop,
          chromeLogoTop,
          chromeProgress,
        )!;
        final logoLeft = lerpDouble(
          (constraints.maxWidth - scaledIntroLogoSize) * 0.5,
          0,
          chromeProgress,
        )!;
        final logoTextOpacity = introActive
            ? introProgress
            : Curves.easeOutCubic.transform(chromeProgress);
        final titleFontSize = lerpDouble(
          scaledIntroTitleFontSize,
          chromeTitleFontSize,
          chromeProgress,
        )!;
        final chromeTitleLeft = logoLeft + logoSize + 10;
        final introTitleTop = introBrandTop + scaledIntroLogoSize + 18;
        final chromeTitleTop =
            (chromeBarHeight - titleFontSize) * 0.5 - 1;
        final titleTop = lerpDouble(
          introTitleTop,
          chromeTitleTop,
          chromeProgress,
        )!;
        final titleAlignmentX = lerpDouble(0, -1, chromeProgress)!;
        final buttonWidth = 104.0;
        final buttonHeight = lerpDouble(44, 46, chromeProgress)!;
        final buttonLeft = lerpDouble(
          (constraints.maxWidth - buttonWidth) * 0.5,
          constraints.maxWidth - buttonWidth,
          chromeProgress,
        )!;
        final buttonTop = lerpDouble(
          constraints.maxHeight * 0.5 + (compact ? 146 : 170),
          constraints.maxHeight - buttonHeight,
          chromeProgress,
        )!;
        final progressOpacity =
            introActive ? Curves.easeOutCubic.transform(exitProgress) : 1.0;
        final skipOpacity =
            introActive ? Curves.easeOutCubic.transform(exitProgress) : 1.0;
        return Stack(
          clipBehavior: Clip.none,
          children: <Widget>[
            Positioned(
              left: logoLeft,
              top: logoTop,
              child: IgnorePointer(
                child: OperitLogoMark(
                  size: logoSize,
                  color: colorScheme.primary.withValues(
                    alpha:
                    logoTextOpacity.clamp(0.0, 1.0).toDouble(),
                  ),
                ),
              ),
            ),
            Positioned(
              left: 0,
              top: titleTop,
              width: constraints.maxWidth,
              child: IgnorePointer(
                child: Transform.translate(
                  offset: Offset(
                    lerpDouble(0, chromeTitleLeft, chromeProgress)!,
                    0,
                  ),
                  child: Align(
                    alignment: Alignment(
                      titleAlignmentX,
                      0,
                    ),
                    child: _AiSetupBrandText(
                      opacity: logoTextOpacity,
                      fontSize: titleFontSize,
                      height: lerpDouble(1.18, 1.0, chromeProgress),
                    ),
                  ),
                ),
              ),
            ),
            Positioned(
              left: 0,
              top: introTitleTop + scaledIntroTitleFontSize * 1.18 + 26,
              width: constraints.maxWidth,
              child: IgnorePointer(
                child: Opacity(
                  opacity: loadingOpacity,
                  child: Transform.translate(
                    offset: Offset(0, 8 * (1 - loadingProgress)),
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        _AiSetupLiquidProgress(
                          color: colorScheme.primary,
                          trackColor: colorScheme.surfaceContainerHighest,
                          progress: introAnimation.value,
                        ),
                        const SizedBox(height: 10),
                        Text(
                          '正在准备本地运行时',
                          style: textTheme.labelMedium?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                            fontWeight: FontWeight.w700,
                            letterSpacing: 0,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
            Positioned(
              right: 0,
              top: 0,
              height: chromeBarHeight,
              child: IgnorePointer(
                ignoring: skipOpacity == 0,
                child: Opacity(
                  opacity: skipOpacity,
                  child: Center(
                    child: TextButton(
                      onPressed: onSkip,
                      style: TextButton.styleFrom(
                        minimumSize: const Size(0, 36),
                        padding: const EdgeInsets.symmetric(horizontal: 12),
                        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                      ),
                      child: const Text('跳过'),
                    ),
                  ),
                ),
              ),
            ),
            Positioned(
              left: 0,
              bottom: 2,
              child: IgnorePointer(
                ignoring: chromeProgress == 0,
                child: Opacity(
                  opacity: progressOpacity,
                  child: IconButton(
                    onPressed: currentPage == 0 ? null : onBack,
                    icon: const Icon(Icons.arrow_back_rounded),
                    color: colorScheme.primary,
                    disabledColor: colorScheme.onSurface.withValues(alpha: 0.3),
                    tooltip: '上一页',
                  ),
                ),
              ),
            ),
            Positioned(
              left: 58,
              right: 112,
              bottom: 5,
              child: IgnorePointer(
                ignoring: chromeProgress == 0,
                child: Opacity(
                  opacity: progressOpacity,
                  child: _AiSetupProgressPill(
                    currentPage: currentPage,
                    pageCount: pageCount,
                    color: colorScheme.primary,
                    trackColor: colorScheme.surfaceContainerHighest,
                    textColor: colorScheme.onSurfaceVariant,
                    label: progressLabel,
                  ),
                ),
              ),
            ),
            Positioned(
              left: buttonLeft,
              top: buttonTop,
              width: buttonWidth,
              height: buttonHeight,
              child: Opacity(
                opacity: bodyProgress,
                child: FilledButton(
                  onPressed: canGoForward ? onPrimary : null,
                  style: FilledButton.styleFrom(
                    padding: const EdgeInsets.symmetric(horizontal: 16),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(18),
                    ),
                  ),
                  child: AnimatedSwitcher(
                    duration: const Duration(milliseconds: 180),
                    child: Row(
                      key: ValueKey<String>(
                        introActive && chromeProgress < 0.5
                            ? 'intro-action'
                            : primaryActionLabel,
                      ),
                      mainAxisSize: MainAxisSize.min,
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: <Widget>[
                        Text(
                          introActive && chromeProgress < 0.5
                              ? '开始'
                              : primaryActionLabel,
                          maxLines: 1,
                        ),
                        const SizedBox(width: 6),
                        Icon(
                          isPermissionPage
                              ? Icons.check_rounded
                              : Icons.arrow_forward_rounded,
                          size: 18,
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ],
        );
      },
    );
  }
}

class _AiSetupLiquidProgress extends StatelessWidget {
  const _AiSetupLiquidProgress({
    required this.color,
    required this.trackColor,
    required this.progress,
  });

  final Color color;
  final Color trackColor;
  final double progress;

  @override
  Widget build(BuildContext context) {
    return CustomPaint(
      size: const Size(168, 16),
      painter: _AiSetupLiquidProgressPainter(
        color: color,
        trackColor: trackColor,
        progress: progress,
      ),
    );
  }
}

class _AiSetupLiquidProgressPainter extends CustomPainter {
  const _AiSetupLiquidProgressPainter({
    required this.color,
    required this.trackColor,
    required this.progress,
  });

  final Color color;
  final Color trackColor;
  final double progress;

  @override
  void paint(Canvas canvas, Size size) {
    final radius = Radius.circular(size.height / 2);
    final rect = Offset.zero & size;
    final rrect = RRect.fromRectAndRadius(rect, radius);
    canvas.drawRRect(
      rrect,
      Paint()..color = trackColor.withValues(alpha: 0.58),
    );

    canvas.save();
    canvas.clipRRect(rrect);

    final phase = progress * math.pi * 2;
    final fillWidth = size.width * (0.42 + 0.18 * math.sin(progress * math.pi));
    final fillLeft = ((size.width + fillWidth) * progress - fillWidth)
        .clamp(-fillWidth, size.width)
        .toDouble();
    final fillRect = Rect.fromLTWH(fillLeft, 0, fillWidth, size.height);
    final fillPaint = Paint()
      ..shader = LinearGradient(
        colors: <Color>[
          color.withValues(alpha: 0.16),
          color.withValues(alpha: 0.92),
          Color.lerp(color, Colors.white, 0.28)!.withValues(alpha: 0.82),
          color.withValues(alpha: 0.24),
        ],
        stops: const <double>[0, 0.42, 0.68, 1],
      ).createShader(fillRect);
    canvas.drawRect(fillRect, fillPaint);

    final wavePaint = Paint()
      ..color = Color.lerp(color, Colors.white, 0.46)!.withValues(alpha: 0.38)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.2
      ..strokeCap = StrokeCap.round;
    final wave = Path();
    for (var x = 0.0; x <= size.width; x += 4) {
      final y = size.height * 0.5 +
          math.sin((x / size.width * math.pi * 2) + phase) * 2.2;
      if (x == 0) {
        wave.moveTo(x, y);
      } else {
        wave.lineTo(x, y);
      }
    }
    canvas.drawPath(wave, wavePaint);
    canvas.restore();
  }

  @override
  bool shouldRepaint(_AiSetupLiquidProgressPainter oldDelegate) {
    return color != oldDelegate.color ||
        trackColor != oldDelegate.trackColor ||
        progress != oldDelegate.progress;
  }
}

class _AiSetupBrandText extends StatelessWidget {
  const _AiSetupBrandText({
    required this.opacity,
    required this.fontSize,
    required this.height,
  });

  final double opacity;
  final double fontSize;
  final double? height;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final textOpacity = opacity.clamp(0.0, 1.0).toDouble();
    final textStyle = (textTheme.headlineMedium ?? const TextStyle()).copyWith(
      color: Colors.white.withValues(alpha: textOpacity),
      fontSize: fontSize,
      fontWeight: FontWeight.w900,
      height: height,
      letterSpacing: 0,
      shadows: <Shadow>[
        Shadow(
          color: colorScheme.primary.withValues(alpha: 0.18 * textOpacity),
          blurRadius: 18,
          offset: const Offset(0, 8),
        ),
      ],
    );

    return ShaderMask(
      blendMode: BlendMode.srcIn,
      shaderCallback: (bounds) {
        return LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: <Color>[
            Color.lerp(colorScheme.primary, colorScheme.onSurface, 0.22)!,
            colorScheme.onSurface,
            Color.lerp(colorScheme.tertiary, colorScheme.primary, 0.32)!,
          ],
          stops: const <double>[0, 0.56, 1],
        ).createShader(bounds);
      },
      child: Text(
        'Operit',
        textAlign: TextAlign.center,
        style: textStyle,
      ),
    );
  }
}

class _AiSetupIntroPage extends StatelessWidget {
  const _AiSetupIntroPage({
    required this.animation,
    required this.exitAnimation,
  });

  final Animation<double> animation;
  final Animation<double> exitAnimation;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final bodyColor = colorScheme.onSurfaceVariant;
    return ColoredBox(
      color: Colors.transparent,
      child: AnimatedBuilder(
        animation: Listenable.merge(<Listenable>[animation, exitAnimation]),
        builder: (context, child) {
          final bodyProgress = CurvedAnimation(
            parent: animation,
            curve: const Interval(0.92, 1, curve: Curves.easeOutCubic),
          ).value;
          final exitProgress = CurvedAnimation(
            parent: exitAnimation,
            curve: Curves.easeOutCubic,
          ).value;
          return LayoutBuilder(
            builder: (context, constraints) {
              final compact = constraints.maxHeight < 560;
              return Center(
                child: SingleChildScrollView(
                  padding: EdgeInsets.symmetric(vertical: compact ? 16 : 24),
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(maxWidth: 440),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.center,
                      children: <Widget>[
                        SizedBox(height: compact ? 126 : 154),
                        Opacity(
                          opacity: (bodyProgress * (1 - exitProgress))
                              .clamp(0.0, 1.0)
                              .toDouble(),
                          child: Transform.translate(
                            offset: Offset(
                              0,
                              18 * (1 - bodyProgress) - 22 * exitProgress,
                            ),
                            child: Padding(
                              padding:
                                  const EdgeInsets.symmetric(horizontal: 12),
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.center,
                                children: <Widget>[
                                  ConstrainedBox(
                                    constraints:
                                        const BoxConstraints(maxWidth: 420),
                                    child: Text(
                                      '让日常任务，从这里变得简单',
                                      textAlign: TextAlign.center,
                                      maxLines: 1,
                                      style: textTheme.titleSmall?.copyWith(
                                        color: bodyColor,
                                        height: 1.2,
                                        fontWeight: FontWeight.w700,
                                        letterSpacing: 0,
                                      ),
                                    ),
                                  ),
                                ],
                              ),
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              );
            },
          );
        },
      ),
    );
  }
}

class _SetupSectionHeader extends StatelessWidget {
  const _SetupSectionHeader({
    required this.icon,
    required this.eyebrow,
    required this.title,
    required this.description,
  });

  final IconData icon;
  final String eyebrow;
  final String title;
  final String description;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(2, 2, 2, 0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Container(
                width: 32,
                height: 32,
                decoration: BoxDecoration(
                  color: colorScheme.primary.withValues(alpha: 0.10),
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Icon(icon, color: colorScheme.primary, size: 18),
              ),
              const SizedBox(width: 10),
              Text(
                eyebrow,
                style: textTheme.labelLarge?.copyWith(
                  color: colorScheme.primary,
                  fontWeight: FontWeight.w800,
                  letterSpacing: 0,
                ),
              ),
            ],
          ),
          const SizedBox(height: 14),
          Text(
            title,
            style: textTheme.headlineSmall?.copyWith(
              color: colorScheme.onSurface,
              fontWeight: FontWeight.w800,
              height: 1.08,
              letterSpacing: 0,
            ),
          ),
          const SizedBox(height: 8),
          ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 480),
            child: Text(
              description,
              style: textTheme.bodyMedium?.copyWith(
                color: colorScheme.onSurfaceVariant,
                height: 1.36,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _AiSetupModePage extends StatelessWidget {
  const _AiSetupModePage({
    required this.selectedMode,
    required this.onModeChanged,
  });

  final _AiSetupStartMode? selectedMode;
  final ValueChanged<_AiSetupStartMode> onModeChanged;

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: Alignment.topCenter,
      child: SingleChildScrollView(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              const _SetupSectionHeader(
                icon: Icons.route_rounded,
                eyebrow: '启动方式',
                title: '选择上手方式',
                description: '快速开始适合首次使用；已有 Operit1 数据时，可以从导入入口继续。',
              ),
              const SizedBox(height: 22),
              _SetupModeTile(
                icon: Icons.flash_on_rounded,
                title: '快速开始',
                subtitle: '配置模型供应商，直接完成基础设置',
                selected: selectedMode == _AiSetupStartMode.quickStart,
                onTap: () => onModeChanged(_AiSetupStartMode.quickStart),
              ),
              const SizedBox(height: 10),
              _SetupModeTile(
                icon: Icons.move_to_inbox_rounded,
                title: '从 Operit1 导入',
                subtitle: '导入旧版配置和数据',
                selected: selectedMode == _AiSetupStartMode.operit1Import,
                onTap: () => onModeChanged(_AiSetupStartMode.operit1Import),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _AiSetupImportPage extends StatelessWidget {
  const _AiSetupImportPage({
    required this.snapshot,
    required this.fileName,
    required this.reading,
    required this.importing,
    required this.progress,
    required this.onPickSnapshot,
    required this.errorText,
  });

  final core_proxy.Operit1SnapshotPreview? snapshot;
  final String? fileName;
  final bool reading;
  final bool importing;
  final core_proxy.Operit1SnapshotImportProgress? progress;
  final VoidCallback onPickSnapshot;
  final String? errorText;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final preview = snapshot;
    final importProgress = importing ? progress : null;
    return Align(
      alignment: Alignment.topCenter,
      child: SingleChildScrollView(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              const _SetupSectionHeader(
                icon: Icons.move_to_inbox_rounded,
                eyebrow: '导入配置',
                title: '从 Operit1 导入',
                description: '选择旧版快照，将配置、聊天、角色卡、资源等数据迁移到 Operit2。',
              ),
              const SizedBox(height: 22),
              Align(
                alignment: Alignment.centerLeft,
                child: FilledButton.tonalIcon(
                  onPressed: reading || importing ? null : onPickSnapshot,
                  icon: reading
                      ? const SizedBox.square(
                          dimension: 18,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.folder_open_rounded, size: 18),
                  label: Text(reading ? '正在读取快照' : '选择快照文件'),
                ),
              ),
              if (fileName != null) ...<Widget>[
                const SizedBox(height: 12),
                Text(
                  fileName!,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.bodyMedium?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
              if (preview != null) ...<Widget>[
                const SizedBox(height: 18),
                Text(
                  '检测到可迁移内容',
                  style: textTheme.titleSmall?.copyWith(
                    color: colorScheme.onSurface,
                    fontWeight: FontWeight.w800,
                    letterSpacing: 0,
                  ),
                ),
                const SizedBox(height: 10),
                Wrap(
                  spacing: 10,
                  runSpacing: 10,
                  children: <Widget>[
                    _SnapshotMetricChip(
                      label: '模型配置',
                      value: '${preview.modelConfig.configs.length}',
                    ),
                    _SnapshotMetricChip(
                      label: '聊天',
                      value: '${preview.chatCount}',
                    ),
                    _SnapshotMetricChip(
                      label: '消息',
                      value: '${preview.messageCount}',
                    ),
                    _SnapshotMetricChip(
                      label: '偏好文件',
                      value: '${preview.datastoreFiles.length}',
                    ),
                    _SnapshotMetricChip(
                      label: '资源文件',
                      value: '${preview.importedFileCount}',
                    ),
                    _SnapshotMetricChip(
                      label: '外部资源',
                      value: '${preview.importedExternalFileCount}',
                    ),
                  ],
                ),
                if (preview.detectedDomains.isNotEmpty) ...<Widget>[
                  const SizedBox(height: 14),
                  Text(
                    preview.detectedDomains.join(' / '),
                    maxLines: 3,
                    overflow: TextOverflow.ellipsis,
                    style: textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                      height: 1.36,
                    ),
                  ),
                ],
                if (preview.modelConfig.chatModelId != null) ...<Widget>[
                  const SizedBox(height: 12),
                  Text(
                    '默认聊天模型：${preview.modelConfig.chatModelId}',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
                const SizedBox(height: 14),
                Text(
                  importing ? '正在导入快照内容，请稍候。' : '点击继续后会开始迁移整份快照。',
                  style: textTheme.bodyMedium?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                    height: 1.36,
                  ),
                ),
                if (importProgress != null) ...<Widget>[
                  const SizedBox(height: 14),
                  _Operit1ImportProgressPanel(progress: importProgress),
                ],
              ],
              if (errorText != null) ...<Widget>[
                const SizedBox(height: 12),
                CommonNetworkErrorView(errorText: errorText!),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _Operit1ImportProgressPanel extends StatelessWidget {
  const _Operit1ImportProgressPanel({required this.progress});

  final core_proxy.Operit1SnapshotImportProgress progress;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final value = progress.progress.clamp(0.0, 1.0).toDouble();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Row(
          children: <Widget>[
            Expanded(
              child: AnimatedSwitcher(
                duration: const Duration(milliseconds: 180),
                child: Text(
                  progress.title,
                  key: ValueKey<String>(progress.stage),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.labelLarge?.copyWith(
                    color: colorScheme.onSurface,
                    fontWeight: FontWeight.w800,
                    letterSpacing: 0,
                  ),
                ),
              ),
            ),
            const SizedBox(width: 12),
            Text(
              '${(value * 100).round()}%',
              style: textTheme.labelMedium?.copyWith(
                color: colorScheme.primary,
                fontWeight: FontWeight.w800,
                letterSpacing: 0,
              ),
            ),
          ],
        ),
        const SizedBox(height: 8),
        ClipRRect(
          borderRadius: BorderRadius.circular(999),
          child: LinearProgressIndicator(
            value: value,
            minHeight: 6,
            backgroundColor:
                colorScheme.surfaceContainerHighest.withValues(alpha: 0.72),
          ),
        ),
        const SizedBox(height: 8),
        AnimatedSwitcher(
          duration: const Duration(milliseconds: 180),
          child: Text(
            progress.detail,
            key: ValueKey<String>('${progress.stage}:${progress.detail}'),
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: textTheme.bodySmall?.copyWith(
              color: colorScheme.onSurfaceVariant,
              height: 1.35,
            ),
          ),
        ),
      ],
    );
  }
}

class _SnapshotMetricChip extends StatelessWidget {
  const _SnapshotMetricChip({
    required this.label,
    required this.value,
  });

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.48),
        borderRadius: BorderRadius.circular(14),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 9),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Text(
              value,
              style: textTheme.labelLarge?.copyWith(
                color: colorScheme.primary,
                fontWeight: FontWeight.w800,
                letterSpacing: 0,
              ),
            ),
            const SizedBox(width: 6),
            Text(
              label,
              style: textTheme.labelMedium?.copyWith(
                color: colorScheme.onSurfaceVariant,
                letterSpacing: 0,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _SetupModeTile extends StatelessWidget {
  const _SetupModeTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.selected,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Material(
      color: selected
          ? colorScheme.primaryContainer.withValues(alpha: 0.46)
          : colorScheme.surfaceContainerHighest.withValues(alpha: 0.42),
      borderRadius: BorderRadius.circular(22),
      child: InkWell(
        borderRadius: BorderRadius.circular(22),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
          child: Row(
            children: <Widget>[
              Icon(
                icon,
                size: 24,
                color: selected
                    ? colorScheme.primary
                    : colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 14),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      title,
                      style: textTheme.titleMedium?.copyWith(
                        color: colorScheme.onSurface,
                        fontWeight: FontWeight.w800,
                        letterSpacing: 0,
                      ),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      subtitle,
                      style: textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                        height: 1.32,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 12),
              Icon(
                selected
                    ? Icons.check_circle_rounded
                    : Icons.radio_button_unchecked_rounded,
                color: selected
                    ? colorScheme.primary
                    : colorScheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _AiSetupModelPage extends StatelessWidget {
  const _AiSetupModelPage({
    required this.formKey,
    required this.catalogEntries,
    required this.selectedProviderTypeId,
    required this.providerConfirmed,
    required this.endpointController,
    required this.apiKeyController,
    required this.availableModels,
    required this.selectedModelId,
    required this.loadingModels,
    required this.onProviderChanged,
    required this.onProviderConfirmed,
    required this.onLoadModels,
    required this.onModelChanged,
    required this.errorText,
  });

  final GlobalKey<FormState> formKey;
  final List<core_proxy.ProviderCatalogEntry> catalogEntries;
  final String? selectedProviderTypeId;
  final bool providerConfirmed;
  final TextEditingController endpointController;
  final TextEditingController apiKeyController;
  final List<core_proxy.AvailableProviderModel> availableModels;
  final String? selectedModelId;
  final bool loadingModels;
  final ValueChanged<String?> onProviderChanged;
  final VoidCallback onProviderConfirmed;
  final VoidCallback onLoadModels;
  final ValueChanged<String?> onModelChanged;
  final String? errorText;

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: Alignment.topCenter,
      child: SingleChildScrollView(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: Form(
            key: formKey,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                _SetupSectionHeader(
                  icon: Icons.auto_awesome_rounded,
                  eyebrow: '模型配置',
                  title: '完成模型配置',
                  description: '选择模型供应商，填写 API Key，拉取并设置默认模型。',
                ),
                const SizedBox(height: 22),
                DropdownButtonFormField<String>(
                  initialValue: selectedProviderTypeId,
                  isExpanded: true,
                  decoration: const InputDecoration(labelText: '模型供应商'),
                  items: catalogEntries
                      .map(
                        (entry) => DropdownMenuItem<String>(
                          value: entry.providerTypeId,
                          child: Text(entry.displayName),
                        ),
                      )
                      .toList(growable: false),
                  onChanged: onProviderChanged,
                  validator: (value) {
                    if (value == null || value.isEmpty) {
                      return '请选择模型供应商';
                    }
                    return null;
                  },
                ),
                const SizedBox(height: 14),
                AnimatedSwitcher(
                  duration: const Duration(milliseconds: 260),
                  switchInCurve: Curves.easeOutCubic,
                  switchOutCurve: Curves.easeInCubic,
                  child: providerConfirmed
                      ? const SizedBox.shrink(key: ValueKey<String>('ready'))
                      : Align(
                          key: const ValueKey<String>('confirm-provider'),
                          alignment: Alignment.centerLeft,
                          child: FilledButton.tonalIcon(
                            onPressed: selectedProviderTypeId == null
                                ? null
                                : onProviderConfirmed,
                            icon: const Icon(
                              Icons.arrow_forward_rounded,
                              size: 18,
                            ),
                            label: const Text('继续配置'),
                          ),
                        ),
                ),
                AnimatedSize(
                  duration: const Duration(milliseconds: 320),
                  curve: Curves.easeOutCubic,
                  alignment: Alignment.topCenter,
                  child: providerConfirmed
                      ? Column(
                          key: const ValueKey<String>('model-credentials'),
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: <Widget>[
                            const SizedBox(height: 12),
                            TextFormField(
                              controller: endpointController,
                              decoration:
                                  const InputDecoration(labelText: '服务地址'),
                              keyboardType: TextInputType.url,
                              validator: _requiredField,
                            ),
                            const SizedBox(height: 12),
                            TextFormField(
                              controller: apiKeyController,
                              decoration:
                                  const InputDecoration(labelText: 'API Key'),
                              validator: _requiredField,
                            ),
                            const SizedBox(height: 16),
                            Align(
                              alignment: Alignment.centerLeft,
                              child: FilledButton.tonalIcon(
                                onPressed: loadingModels ? null : onLoadModels,
                                icon: loadingModels
                                    ? const SizedBox.square(
                                        dimension: 18,
                                        child: CircularProgressIndicator(
                                          strokeWidth: 2,
                                        ),
                                      )
                                    : const Icon(Icons.sync_rounded, size: 18),
                                label: Text(
                                  loadingModels ? '正在拉取模型' : '拉取可用模型',
                                ),
                              ),
                            ),
                          ],
                        )
                      : const SizedBox.shrink(
                          key: ValueKey<String>('model-credentials-empty'),
                        ),
                  ),
                if (availableModels.isNotEmpty) ...<Widget>[
                  const SizedBox(height: 16),
                  DropdownButtonFormField<String>(
                    initialValue: selectedModelId,
                    isExpanded: true,
                    decoration: const InputDecoration(labelText: '默认模型'),
                    items: availableModels
                        .map(
                          (model) => DropdownMenuItem<String>(
                            value: model.modelId,
                            child: Text(model.modelId),
                          ),
                        )
                        .toList(growable: false),
                    onChanged: onModelChanged,
                  ),
                ],
                if (errorText != null) ...<Widget>[
                  const SizedBox(height: 12),
                  CommonNetworkErrorView(errorText: errorText!),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _AiSetupPermissionPage extends StatelessWidget {
  const _AiSetupPermissionPage({
    required this.permissions,
    required this.requesting,
    required this.onRefresh,
    required this.onRequest,
    required this.errorText,
  });

  final _OnboardingPermissionSnapshot permissions;
  final bool requesting;
  final VoidCallback onRefresh;
  final ValueChanged<_OnboardingPermissionAction> onRequest;
  final String? errorText;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Align(
      alignment: Alignment.topCenter,
      child: SingleChildScrollView(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              _SetupSectionHeader(
                icon: Icons.admin_panel_settings_rounded,
                eyebrow: '按需开启',
                title: '给工具箱必要能力',
                description: '附近设备、悬浮入口和持续任务会在对应场景里使用。',
              ),
              const SizedBox(height: 22),
              _PermissionTile(
                title: '附近设备',
                subtitle: '用于发现、连接和管理身边的设备',
                granted: permissions.location,
                requesting: requesting,
                onTap: () => onRequest(_OnboardingPermissionAction.location),
              ),
              _PermissionTile(
                title: '蓝牙连接',
                subtitle: '用于和已授权的设备保持稳定通信',
                granted: permissions.bluetoothConnect && permissions.bluetoothScan,
                requesting: requesting,
                onTap: () => onRequest(_OnboardingPermissionAction.bluetooth),
              ),
              _PermissionTile(
                title: '悬浮入口',
                subtitle: '用于在其他应用中快速唤起工具箱',
                granted: permissions.overlay,
                requesting: requesting,
                onTap: () => onRequest(_OnboardingPermissionAction.overlay),
              ),
              _PermissionTile(
                title: '持续任务',
                subtitle: '用于让同步、协作和长任务保持连续',
                granted: permissions.batteryOptimization,
                requesting: requesting,
                onTap: () => onRequest(_OnboardingPermissionAction.battery),
              ),
              TextButton.icon(
                onPressed: requesting ? null : onRefresh,
                icon: const Icon(Icons.refresh_rounded),
                label: const Text('刷新授权状态'),
              ),
              if (errorText != null)
                Text(
                  errorText!,
                  style: textTheme.bodySmall?.copyWith(
                    color: colorScheme.error,
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class _PermissionTile extends StatelessWidget {
  const _PermissionTile({
    required this.title,
    required this.subtitle,
    required this.granted,
    required this.requesting,
    required this.onTap,
  });

  final String title;
  final String subtitle;
  final bool granted;
  final bool requesting;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Card(
      elevation: 0,
      color: granted
          ? colorScheme.primaryContainer
          : colorScheme.surfaceContainerHighest,
      margin: const EdgeInsets.only(bottom: 10),
      child: ListTile(
        leading: Icon(
          granted ? Icons.check_circle_rounded : Icons.radio_button_unchecked,
          color: granted ? colorScheme.primary : colorScheme.onSurfaceVariant,
        ),
        title: Text(title),
        subtitle: Text(subtitle),
        trailing: TextButton(
          onPressed: granted || requesting ? null : onTap,
          child: Text(granted ? '已授权' : '去授权'),
        ),
      ),
    );
  }
}

String? _requiredField(String? value) {
  if (value == null || value.trim().isEmpty) {
    return '必填';
  }
  return null;
}

enum _OnboardingPermissionAction {
  location('location'),
  bluetooth('bluetooth'),
  overlay('overlay'),
  battery('battery');

  const _OnboardingPermissionAction(this.methodName);

  final String methodName;
}

enum _AiSetupStartMode {
  quickStart,
  operit1Import,
}

class _OnboardingPermissionSnapshot {
  const _OnboardingPermissionSnapshot({
    required this.location,
    required this.bluetoothConnect,
    required this.bluetoothScan,
    required this.overlay,
    required this.batteryOptimization,
  });

  factory _OnboardingPermissionSnapshot.fromJson(Map<Object?, Object?> json) {
    return _OnboardingPermissionSnapshot(
      location: json['location'] == true,
      bluetoothConnect: json['bluetoothConnect'] == true,
      bluetoothScan: json['bluetoothScan'] == true,
      overlay: json['overlay'] == true,
      batteryOptimization: json['batteryOptimization'] == true,
    );
  }

  final bool location;
  final bool bluetoothConnect;
  final bool bluetoothScan;
  final bool overlay;
  final bool batteryOptimization;
}

class _OnboardingPermissionBridge {
  static const MethodChannel _channel = MethodChannel('operit/runtime');

  static Future<_OnboardingPermissionSnapshot> snapshot() async {
    final result = await _channel.invokeMapMethod<Object?, Object?>(
      'androidOnboardingPermissionSnapshot',
    );
    if (result == null) {
      throw StateError('android onboarding permission snapshot is empty');
    }
    return _OnboardingPermissionSnapshot.fromJson(result);
  }

  static Future<void> request(_OnboardingPermissionAction action) {
    return _channel.invokeMethod<void>(
      'androidOnboardingRequestPermission',
      <String, Object?>{'permission': action.methodName},
    );
  }
}

class _PickedOperit1SnapshotFile {
  const _PickedOperit1SnapshotFile({
    required this.path,
    required this.name,
  });

  final String path;
  final String name;
}

