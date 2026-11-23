// ignore_for_file: file_names
import 'package:flutter/material.dart';

import '../../core/host/VoiceCallCamera.dart';

class VoiceCallCameraPanel extends StatelessWidget {
  const VoiceCallCameraPanel({super.key, required this.camera});

  final VoiceCallCamera camera;

  @override
  Widget build(BuildContext context) {
    final zh = Localizations.localeOf(context).languageCode == 'zh';
    final colors = Theme.of(context).colorScheme;
    final portrait = MediaQuery.orientationOf(context) == Orientation.portrait;
    return ListenableBuilder(
      listenable: camera,
      builder: (context, _) {
        final preview = camera.preview;
        return Material(
          color: colors.surface,
          borderRadius: BorderRadius.circular(20),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                if (preview != null)
                  ConstrainedBox(
                    constraints: const BoxConstraints(maxHeight: 128),
                    child: ClipRRect(
                      borderRadius: BorderRadius.circular(12),
                      child: AspectRatio(
                        aspectRatio: voiceCallCameraNeedsActivity && portrait
                            ? preview.previewHeight / preview.previewWidth
                            : preview.previewWidth / preview.previewHeight,
                        child: camera.platform.buildPreview(preview.cameraId),
                      ),
                    ),
                  ),
                Row(
                  children: [
                    IconButton(
                      isSelected: camera.enabled,
                      selectedIcon: const Icon(Icons.videocam_outlined),
                      icon: const Icon(Icons.videocam_off_outlined),
                      tooltip: camera.enabled
                          ? (zh ? '关闭摄像头' : 'Turn camera off')
                          : (zh ? '开启摄像头' : 'Turn camera on'),
                      onPressed: camera.enabled ? camera.stop : camera.start,
                    ),
                    Expanded(
                      child: Text(
                        camera.error != null
                            ? (zh
                                  ? '摄像头不可用，请检查设备和系统权限'
                                  : 'Camera unavailable. Check the device and system permissions.')
                            : preview != null
                            ? (zh
                                  ? '每轮发言共享一张画面'
                                  : 'Shares one frame per spoken turn')
                            : camera.enabled
                            ? (zh ? '正在打开摄像头…' : 'Opening camera…')
                            : (zh ? '摄像头已关闭' : 'Camera off'),
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}
