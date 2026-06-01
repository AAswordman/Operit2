// ignore_for_file: file_names

import 'package:flutter/material.dart';

enum SettingsCategory {
  model,
  chat,
  role,
  memory,
  permission,
  appearance,
  data,
}

class SettingsCategorySpec {
  const SettingsCategorySpec({
    required this.title,
    required this.subtitle,
    required this.description,
    required this.icon,
    required this.sections,
  });

  final String title;
  final String subtitle;
  final String description;
  final IconData icon;
  final List<SettingsSectionSpec> sections;

  static SettingsCategorySpec of(SettingsCategory category) {
    return switch (category) {
      SettingsCategory.model => settingsModelSpec,
      SettingsCategory.chat => settingsChatSpec,
      SettingsCategory.role => settingsRoleSpec,
      SettingsCategory.memory => settingsMemorySpec,
      SettingsCategory.permission => settingsPermissionSpec,
      SettingsCategory.appearance => settingsAppearanceSpec,
      SettingsCategory.data => settingsDataSpec,
    };
  }
}

class SettingsSectionSpec {
  const SettingsSectionSpec({required this.title, required this.items});

  final String title;
  final List<SettingsItemSpec> items;
}

class SettingsItemSpec {
  const SettingsItemSpec({
    required this.title,
    required this.description,
    required this.icon,
  });

  final String title;
  final String description;
  final IconData icon;
}

const SettingsCategorySpec settingsModelSpec = SettingsCategorySpec(
  title: '模型',
  subtitle: '供应商、Key、模型选择',
  description: '接入 AI 服务，选择供应商，填写 Key，拉取并测试模型。',
  icon: Icons.hub_outlined,
  sections: <SettingsSectionSpec>[
    SettingsSectionSpec(
      title: '常用',
      items: <SettingsItemSpec>[
        SettingsItemSpec(
          title: '当前聊天模型',
          description: '查看当前聊天使用的模型档案和模型。',
          icon: Icons.chat_bubble_outline,
        ),
        SettingsItemSpec(
          title: '新建模型档案',
          description: '选择供应商，填写 Key，拉取模型并保存。',
          icon: Icons.add_circle_outline,
        ),
      ],
    ),
    SettingsSectionSpec(
      title: '更多',
      items: <SettingsItemSpec>[
        SettingsItemSpec(
          title: '模型档案',
          description: '管理所有已保存的模型连接。',
          icon: Icons.list_alt_outlined,
        ),
        SettingsItemSpec(
          title: '使用场景分配',
          description: '为聊天、总结、记忆和识别能力分配模型。',
          icon: Icons.account_tree_outlined,
        ),
      ],
    ),
  ],
);

const SettingsCategorySpec settingsChatSpec = SettingsCategorySpec(
  title: '聊天',
  subtitle: '输出、思考、上下文',
  description: '调整聊天过程里的输出方式、思考模式、媒体历史和总结策略。',
  icon: Icons.forum_outlined,
  sections: <SettingsSectionSpec>[
    SettingsSectionSpec(
      title: '体验',
      items: <SettingsItemSpec>[
        SettingsItemSpec(
          title: '流式输出',
          description: '控制回复是否逐步显示。',
          icon: Icons.notes_outlined,
        ),
        SettingsItemSpec(
          title: '思考模式',
          description: '控制模型思考内容和强度。',
          icon: Icons.psychology_outlined,
        ),
      ],
    ),
    SettingsSectionSpec(
      title: '上下文',
      items: <SettingsItemSpec>[
        SettingsItemSpec(
          title: '媒体历史',
          description: '控制图片、音频和视频在上下文里的保留轮数。',
          icon: Icons.perm_media_outlined,
        ),
        SettingsItemSpec(
          title: '上下文总结',
          description: '设置长对话自动总结的触发规则。',
          icon: Icons.summarize_outlined,
        ),
      ],
    ),
  ],
);

const SettingsCategorySpec settingsRoleSpec = SettingsCategorySpec(
  title: '角色',
  subtitle: '人设卡、角色组、绑定',
  description: '管理 AI 的角色、人设卡、角色组和默认绑定。',
  icon: Icons.badge_outlined,
  sections: <SettingsSectionSpec>[
    SettingsSectionSpec(
      title: '角色管理',
      items: <SettingsItemSpec>[
        SettingsItemSpec(
          title: '人设卡',
          description: '创建、编辑和选择默认角色。',
          icon: Icons.face_outlined,
        ),
        SettingsItemSpec(
          title: '角色组',
          description: '管理多人设对话和角色组合。',
          icon: Icons.groups_outlined,
        ),
      ],
    ),
    SettingsSectionSpec(
      title: '绑定',
      items: <SettingsItemSpec>[
        SettingsItemSpec(
          title: '角色模型绑定',
          description: '让指定角色使用固定模型档案。',
          icon: Icons.link_outlined,
        ),
      ],
    ),
  ],
);

const SettingsCategorySpec settingsMemorySpec = SettingsCategorySpec(
  title: '记忆',
  subtitle: '用户偏好和长期记忆',
  description: '管理用户偏好、记忆档案、自动记忆和搜索设置。',
  icon: Icons.memory_outlined,
  sections: <SettingsSectionSpec>[
    SettingsSectionSpec(
      title: '档案',
      items: <SettingsItemSpec>[
        SettingsItemSpec(
          title: '记忆档案',
          description: '切换和编辑用户长期记忆档案。',
          icon: Icons.folder_shared_outlined,
        ),
        SettingsItemSpec(
          title: '用户偏好',
          description: '管理生日、身份、风格等可复用偏好。',
          icon: Icons.person_outline,
        ),
      ],
    ),
    SettingsSectionSpec(
      title: '自动化',
      items: <SettingsItemSpec>[
        SettingsItemSpec(
          title: '自动记忆',
          description: '控制对话中是否自动整理记忆。',
          icon: Icons.auto_awesome_outlined,
        ),
        SettingsItemSpec(
          title: '记忆搜索',
          description: '调整搜索权重、嵌入和保存策略。',
          icon: Icons.search_outlined,
        ),
      ],
    ),
  ],
);

const SettingsCategorySpec settingsPermissionSpec = SettingsCategorySpec(
  title: '权限',
  subtitle: '工具、工作区、外部访问',
  description: '管理 Operit 可以调用什么工具、访问什么资源，以及危险操作确认。',
  icon: Icons.admin_panel_settings_outlined,
  sections: <SettingsSectionSpec>[
    SettingsSectionSpec(
      title: '工具',
      items: <SettingsItemSpec>[
        SettingsItemSpec(
          title: '工具权限',
          description: '为工具设置允许、询问或禁止。',
          icon: Icons.construction_outlined,
        ),
        SettingsItemSpec(
          title: 'MCP 启动超时',
          description: '控制 MCP 服务启动等待时间。',
          icon: Icons.timer_outlined,
        ),
      ],
    ),
    SettingsSectionSpec(
      title: '边界',
      items: <SettingsItemSpec>[
        SettingsItemSpec(
          title: '工作区访问',
          description: '管理文件和项目工作区的访问边界。',
          icon: Icons.folder_open_outlined,
        ),
        SettingsItemSpec(
          title: '外部 HTTP',
          description: '配置外部程序调用 Operit 的接口。',
          icon: Icons.http_outlined,
        ),
      ],
    ),
  ],
);

const SettingsCategorySpec settingsAppearanceSpec = SettingsCategorySpec(
  title: '外观',
  subtitle: '主题、语言、布局',
  description: '调整界面主题、语言、显示密度、布局和 Markdown 呈现。',
  icon: Icons.palette_outlined,
  sections: <SettingsSectionSpec>[
    SettingsSectionSpec(
      title: '界面',
      items: <SettingsItemSpec>[
        SettingsItemSpec(
          title: '主题',
          description: '选择颜色、深浅模式和背景。',
          icon: Icons.color_lens_outlined,
        ),
        SettingsItemSpec(
          title: '语言',
          description: '切换应用显示语言。',
          icon: Icons.language_outlined,
        ),
      ],
    ),
    SettingsSectionSpec(
      title: '阅读',
      items: <SettingsItemSpec>[
        SettingsItemSpec(
          title: '布局与密度',
          description: '调整界面紧凑程度和内容宽度。',
          icon: Icons.view_compact_outlined,
        ),
        SettingsItemSpec(
          title: 'Markdown 显示',
          description: '控制代码块、公式、表格和动画。',
          icon: Icons.article_outlined,
        ),
      ],
    ),
  ],
);

const SettingsCategorySpec settingsDataSpec = SettingsCategorySpec(
  title: '数据与关于',
  subtitle: '历史、备份、诊断',
  description: '管理聊天历史、备份恢复、统计信息、版本更新和诊断日志。',
  icon: Icons.storage_outlined,
  sections: <SettingsSectionSpec>[
    SettingsSectionSpec(
      title: '数据',
      items: <SettingsItemSpec>[
        SettingsItemSpec(
          title: '聊天历史',
          description: '管理、清理和迁移聊天记录。',
          icon: Icons.manage_history_outlined,
        ),
        SettingsItemSpec(
          title: '备份与恢复',
          description: '导出、导入和恢复应用数据。',
          icon: Icons.settings_backup_restore_outlined,
        ),
        SettingsItemSpec(
          title: 'Token 统计',
          description: '查看模型调用和消耗统计。',
          icon: Icons.analytics_outlined,
        ),
      ],
    ),
    SettingsSectionSpec(
      title: '诊断',
      items: <SettingsItemSpec>[
        SettingsItemSpec(
          title: '版本与更新',
          description: '查看版本信息并检查更新。',
          icon: Icons.system_update_alt_outlined,
        ),
        SettingsItemSpec(
          title: 'Host 与日志',
          description: '查看运行路径、能力和诊断日志。',
          icon: Icons.terminal_outlined,
        ),
      ],
    ),
  ],
);
