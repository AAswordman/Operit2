// ignore_for_file: file_names

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/M3LoadingIndicator.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/EmptyState.dart';
import '../components/PackageGrid.dart';
import '../components/PackageListItem.dart';

class SkillConfigScreen extends StatefulWidget {
  const SkillConfigScreen({
    super.key,
    required this.clients,
    required this.searchQuery,
    required this.reloadRevision,
  });

  final GeneratedCoreProxyClients clients;
  final String searchQuery;
  final int reloadRevision;

  @override
  State<SkillConfigScreen> createState() => _SkillConfigScreenState();
}

class _SkillConfigScreenState extends State<SkillConfigScreen> {
  bool _loading = true;
  String? _errorMessage;
  String _skillsDirectory = '';
  Map<String, core_proxy.SkillPackage> _skills =
      <String, core_proxy.SkillPackage>{};
  Map<String, String> _loadErrors = <String, String>{};
  Set<String> _visibleSkillNames = <String>{};
  List<core_proxy.BundledExternalSkillCandidate> _moreSkills =
      <core_proxy.BundledExternalSkillCandidate>[];

  GeneratedSkillRepositoryCoreProxy get _repository =>
      widget.clients.skillRepository;

  @override
  void initState() {
    super.initState();
    _loadSkills();
  }

  @override
  void didUpdateWidget(covariant SkillConfigScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.reloadRevision != widget.reloadRevision) {
      _loadSkills();
    }
  }

  Future<void> _loadSkills() async {
    setState(() {
      _loading = true;
      _errorMessage = null;
    });
    try {
      final baseResults = await Future.wait<Object>(<Future<Object>>[
        _repository.getSkillsDirectoryPath(),
        _repository.getAvailableSkillPackages(),
        _repository.getSkillLoadErrors(),
        _repository.getBundledExternalSkillCandidates(),
      ]);
      final skillsDirectory = baseResults[0] as String;
      final skills = baseResults[1] as Map<String, core_proxy.SkillPackage>;
      final loadErrors = baseResults[2] as Map<String, String>;
      final moreSkills =
          baseResults[3] as List<core_proxy.BundledExternalSkillCandidate>;
      final visibilityResults = await Future.wait<bool>(
        skills.keys.map(
          (skillName) => _repository.isSkillVisibleToAi(skillName: skillName),
        ),
      );
      final visibleSkillNames = <String>{};
      var index = 0;
      for (final skillName in skills.keys) {
        if (visibilityResults[index]) {
          visibleSkillNames.add(skillName);
        }
        index += 1;
      }
      if (!mounted) {
        return;
      }
      setState(() {
        _skillsDirectory = skillsDirectory;
        _skills = skills;
        _loadErrors = loadErrors;
        _visibleSkillNames = visibleSkillNames;
        _moreSkills = moreSkills;
        _loading = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load skills: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
        _loading = false;
      });
    }
  }

  Future<void> _setSkillVisible(String skillName, bool visible) async {
    final previous = _visibleSkillNames.contains(skillName);
    setState(() {
      final next = Set<String>.from(_visibleSkillNames);
      if (visible) {
        next.add(skillName);
      } else {
        next.remove(skillName);
      }
      _visibleSkillNames = next;
    });
    try {
      await _repository.setSkillVisibleToAi(
        skillName: skillName,
        visible: visible,
      );
    } catch (error, stackTrace) {
      debugPrint('Failed to update skill visibility: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        final next = Set<String>.from(_visibleSkillNames);
        if (previous) {
          next.add(skillName);
        } else {
          next.remove(skillName);
        }
        _visibleSkillNames = next;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  Future<void> _showSkillDetails(core_proxy.SkillPackage skill) async {
    showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (context) {
        return const AlertDialog(
          content: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              M3LoadingIndicator(size: 18),
              SizedBox(width: 12),
              Text('加载技能详情'),
            ],
          ),
        );
      },
    );
    final content = await _repository.readSkillContent(skillName: skill.name);
    if (!mounted) {
      return;
    }
    Navigator.of(context).pop();
    showDialog<void>(
      context: context,
      builder: (context) {
        return _SkillDetailsDialog(
          skill: skill,
          content: content,
          onDelete: () async {
            Navigator.of(context).pop();
            final scaffoldMessenger = ScaffoldMessenger.of(this.context);
            final deleted = await _repository.deleteSkill(
              skillName: skill.name,
            );
            await _loadSkills();
            if (!mounted) {
              return;
            }
            if (!deleted) {
              scaffoldMessenger.showSnackBar(
                SnackBar(
                  content: Text('删除失败 ${skill.name}'),
                  behavior: SnackBarBehavior.floating,
                ),
              );
            }
          },
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    final error = _errorMessage;
    if (_loading && _skills.isEmpty && _moreSkills.isEmpty) {
      return const M3LoadingPane();
    }
    if (error != null && _skills.isEmpty && _moreSkills.isEmpty) {
      return EmptyState(
        icon: Icons.error_outline,
        title: '加载失败',
        message: error,
        action: TextButton.icon(
          onPressed: _loadSkills,
          icon: const Icon(Icons.refresh),
          label: const Text('刷新'),
        ),
      );
    }

    final displayedSkills = _filteredSkills;
    final displayedMoreSkills = _filteredMoreSkills;
    final searchActive = widget.searchQuery.trim().isNotEmpty;
    return Stack(
      children: <Widget>[
        RefreshIndicator(
          onRefresh: _loadSkills,
          child: ListView(
            physics: const AlwaysScrollableScrollPhysics(),
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 120),
            children: <Widget>[
              _SkillHeaderCard(
                directory: _skillsDirectory,
                errorCount: _loadErrors.length,
                onRefresh: _loadSkills,
                onShowErrors: _loadErrors.isEmpty
                    ? null
                    : () => _showLoadErrors(_loadErrors),
              ),
              const SizedBox(height: 12),
              if (displayedSkills.isEmpty && displayedMoreSkills.isEmpty)
                EmptyState(
                  icon: Icons.build_outlined,
                  title: '没有技能',
                  message: searchActive
                      ? '没有匹配的技能。'
                      : '当前没有可显示的技能。',
                  scrollable: false,
                )
              else ...<Widget>[
                const _SkillSectionHeader(title: '当前技能'),
                if (displayedSkills.isEmpty)
                  _SkillSectionEmpty(
                    message: searchActive ? '没有匹配的当前技能。' : '当前没有可显示的技能。',
                  )
                else
                  PackageInlineGrid(
                    itemCount: displayedSkills.length,
                    itemBuilder: (context, index) {
                      final skill = displayedSkills[index];
                      final visible = _visibleSkillNames.contains(skill.name);
                      return PackageListItem(
                        icon: Icons.build_outlined,
                        title: skill.name,
                        subtitle: skill.description,
                        metadata: <String>[visible ? 'AI 可见' : 'AI 隐藏'],
                        enabled: visible,
                        onTap: () => _showSkillDetails(skill),
                        onEnabledChanged: (value) =>
                            _setSkillVisible(skill.name, value),
                      );
                    },
                  ),
                if (displayedMoreSkills.isNotEmpty) ...<Widget>[
                  const SizedBox(height: 16),
                  const _SkillSectionHeader(
                    title: '更多技能',
                    subtitle: 'App 自带的官方额外技能，加载后进入当前技能。',
                  ),
                  PackageInlineGrid(
                    itemCount: displayedMoreSkills.length,
                    itemBuilder: (context, index) {
                      final skill = displayedMoreSkills[index];
                      return PackageListItem(
                        icon: Icons.build_outlined,
                        title: skill.name,
                        subtitle: skill.description,
                        metadata: const <String>['官方额外'],
                        enabled: false,
                        onEnabledChanged: (_) {},
                        showEnabledSwitch: false,
                        trailingActions: <Widget>[
                          FilledButton.tonalIcon(
                            onPressed: () => _loadBundledSkill(skill),
                            icon: const Icon(Icons.add, size: 18),
                            label: const Text('加载'),
                            style: FilledButton.styleFrom(
                              visualDensity: VisualDensity.compact,
                              padding: const EdgeInsets.symmetric(
                                horizontal: 10,
                              ),
                            ),
                          ),
                        ],
                      );
                    },
                  ),
                ],
              ],
            ],
          ),
        ),
        if (_loading && (_skills.isNotEmpty || _moreSkills.isNotEmpty))
          const Positioned.fill(child: M3LoadingOverlay()),
      ],
    );
  }

  List<core_proxy.SkillPackage> get _filteredSkills {
    final query = widget.searchQuery.trim().toLowerCase();
    final items = _skills.values.toList()
      ..sort((left, right) => left.name.compareTo(right.name));
    if (query.isEmpty) {
      return items;
    }
    return items
        .where(
          (skill) =>
              skill.name.toLowerCase().contains(query) ||
              skill.description.toLowerCase().contains(query) ||
              skill.directory.toString().toLowerCase().contains(query),
        )
        .toList(growable: false);
  }

  List<core_proxy.BundledExternalSkillCandidate> get _filteredMoreSkills {
    final query = widget.searchQuery.trim().toLowerCase();
    final items = _moreSkills.toList()
      ..sort((left, right) => left.name.compareTo(right.name));
    if (query.isEmpty) {
      return items;
    }
    return items
        .where(
          (skill) =>
              skill.name.toLowerCase().contains(query) ||
              skill.description.toLowerCase().contains(query),
        )
        .toList(growable: false);
  }

  Future<void> _loadBundledSkill(
    core_proxy.BundledExternalSkillCandidate skill,
  ) async {
    try {
      await _repository.importBundledExternalSkill(skillName: skill.name);
      await _loadSkills();
    } catch (error, stackTrace) {
      debugPrint('Failed to load bundled skill: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  void _showLoadErrors(Map<String, String> errors) {
    showDialog<void>(
      context: context,
      builder: (context) {
        return AlertDialog(
          icon: const Icon(Icons.error_outline),
          title: const Text('技能加载错误'),
          content: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 560, maxHeight: 420),
            child: SingleChildScrollView(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: errors.entries
                    .map(
                      (entry) => Padding(
                        padding: const EdgeInsets.only(bottom: 12),
                        child: Text('${entry.key}\n${entry.value}'),
                      ),
                    )
                    .toList(growable: false),
              ),
            ),
          ),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('关闭'),
            ),
          ],
        );
      },
    );
  }
}

class _SkillSectionHeader extends StatelessWidget {
  const _SkillSectionHeader({required this.title, this.subtitle});

  final String title;
  final String? subtitle;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final subtitle = this.subtitle;
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 4, 4, 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            title,
            style: Theme.of(
              context,
            ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
          ),
          if (subtitle != null) ...<Widget>[
            const SizedBox(height: 2),
            Text(
              subtitle,
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _SkillSectionEmpty extends StatelessWidget {
  const _SkillSectionEmpty({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 0, 4, 4),
      child: Text(
        message,
        style: Theme.of(context).textTheme.bodySmall?.copyWith(
          color: Theme.of(context).colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

class _SkillHeaderCard extends StatelessWidget {
  const _SkillHeaderCard({
    required this.directory,
    required this.errorCount,
    required this.onRefresh,
    required this.onShowErrors,
  });

  final String directory;
  final int errorCount;
  final VoidCallback onRefresh;
  final VoidCallback? onShowErrors;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return OperitGlassSurface(
      color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.30),
      layer: OperitGlassSurfaceLayer.card,
      borderRadius: BorderRadius.circular(16),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.16),
      ),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Row(
          children: <Widget>[
            const Icon(Icons.build_outlined),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    'Skills',
                    style: Theme.of(context).textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  if (directory.trim().isNotEmpty)
                    Text(
                      directory,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                ],
              ),
            ),
            if (errorCount > 0)
              IconButton(
                tooltip: '加载错误',
                onPressed: onShowErrors,
                icon: Badge(
                  label: Text(errorCount.toString()),
                  child: const Icon(Icons.error_outline),
                ),
              ),
            IconButton(
              tooltip: '刷新',
              onPressed: onRefresh,
              icon: const Icon(Icons.refresh),
            ),
          ],
        ),
      ),
    );
  }
}

class _SkillDetailsDialog extends StatelessWidget {
  const _SkillDetailsDialog({
    required this.skill,
    required this.content,
    required this.onDelete,
  });

  final core_proxy.SkillPackage skill;
  final String? content;
  final AsyncCallback onDelete;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      icon: const Icon(Icons.build_outlined),
      title: Text(skill.name),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 620, maxHeight: 520),
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              if (skill.description.trim().isNotEmpty) Text(skill.description),
              const SizedBox(height: 12),
              Text('目录: ${skill.directory}'),
              Text('入口: ${skill.skillFile}'),
              if (content != null) ...<Widget>[
                const SizedBox(height: 12),
                SelectableText(content!),
              ],
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(onPressed: onDelete, child: const Text('删除')),
        FilledButton.tonal(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('关闭'),
        ),
      ],
    );
  }
}
