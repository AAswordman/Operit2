// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';

/// Maps provider type IDs to their bundled brand logo assets.
const Map<String, String> providerLogoAssets = <String, String>{
  'ALIPAY_BAILING': 'assets/model_logos/ALIPAY_BAILING.svg',
  'ALIYUN': 'assets/model_logos/ALIYUN.svg',
  'ANTHROPIC': 'assets/model_logos/ANTHROPIC.svg',
  'ANTHROPIC_GENERIC': 'assets/model_logos/ANTHROPIC_GENERIC.svg',
  'BAICHUAN': 'assets/model_logos/BAICHUAN.svg',
  'BAIDU': 'assets/model_logos/BAIDU.svg',
  'DEEPSEEK': 'assets/model_logos/DEEPSEEK.svg',
  'DOUBAO': 'assets/model_logos/DOUBAO.svg',
  'FOUR_ROUTER': 'assets/model_logos/FOUR_ROUTER.svg',
  'GEMINI_GENERIC': 'assets/model_logos/GEMINI_GENERIC.svg',
  'GOOGLE': 'assets/model_logos/GOOGLE.svg',
  'IFLOW': 'assets/model_logos/IFLOW.png',
  'INFINIAI': 'assets/model_logos/INFINIAI.png',
  'LMSTUDIO': 'assets/model_logos/LMSTUDIO.svg',
  'MIMO': 'assets/model_logos/MIMO.svg',
  'MINIMAX': 'assets/model_logos/MINIMAX.svg',
  'MISTRAL': 'assets/model_logos/MISTRAL.svg',
  'MOONSHOT': 'assets/model_logos/MOONSHOT.svg',
  'NOUS_PORTAL': 'assets/model_logos/NOUS_PORTAL.svg',
  'NOVITA': 'assets/model_logos/NOVITA.svg',
  'NVIDIA': 'assets/model_logos/NVIDIA.svg',
  'OLLAMA': 'assets/model_logos/OLLAMA.svg',
  'OPENAI': 'assets/model_logos/OPENAI.svg',
  'OPENAI_CODEX': 'assets/model_logos/OPENAI_CODEX.svg',
  'OPENAI_GENERIC': 'assets/model_logos/OPENAI_GENERIC.svg',
  'OPENAI_LOCAL': 'assets/model_logos/OPENAI_LOCAL.svg',
  'OPENAI_RESPONSES': 'assets/model_logos/OPENAI_RESPONSES.svg',
  'OPENAI_RESPONSES_GENERIC': 'assets/model_logos/OPENAI_RESPONSES_GENERIC.svg',
  'OPENCODE': 'assets/model_logos/OPENCODE.svg',
  'OPENROUTER': 'assets/model_logos/OPENROUTER.svg',
  'PPINFRA': 'assets/model_logos/PPINFRA.svg',
  'SILICONFLOW': 'assets/model_logos/SILICONFLOW.svg',
  'XAI': 'assets/model_logos/XAI.svg',
  'XUNFEI': 'assets/model_logos/XUNFEI.svg',
  'ZHIPU': 'assets/model_logos/ZHIPU.svg',
};

/// Logos that carry brand colors and keep them in both themes.
const Set<String> _coloredLogoProviderTypeIds = <String>{
  'FOUR_ROUTER',
  'IFLOW',
  'INFINIAI',
};

/// Renders a provider logo with an optional circular container and initial fallback.
class ProviderLogo extends StatelessWidget {
  const ProviderLogo({
    super.key,
    required this.providerTypeId,
    required this.fallbackName,
    this.size = 42,
    this.contentScale = 0.62,
    this.showBackdrop = true,
  });

  final String providerTypeId;
  final String fallbackName;
  final double size;
  final double contentScale;
  final bool showBackdrop;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final asset = providerLogoAssets[providerTypeId];
    final glyphSize = showBackdrop ? size * contentScale : size;
    final child = asset == null
        ? _ProviderLogoInitial(name: fallbackName, size: glyphSize)
        : _ProviderLogoAsset(
            asset: asset,
            providerTypeId: providerTypeId,
            size: glyphSize,
          );
    if (!showBackdrop) {
      return SizedBox(
        width: size,
        height: size,
        child: Center(child: child),
      );
    }
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.7),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.42),
        ),
      ),
      alignment: Alignment.center,
      child: child,
    );
  }
}

class _ProviderLogoAsset extends StatelessWidget {
  const _ProviderLogoAsset({
    required this.asset,
    required this.providerTypeId,
    required this.size,
  });

  final String asset;
  final String providerTypeId;
  final double size;

  @override
  Widget build(BuildContext context) {
    final monochromeTint = _coloredLogoProviderTypeIds.contains(providerTypeId)
        ? null
        : ColorFilter.mode(
            Theme.of(context).colorScheme.onSurface,
            BlendMode.srcIn,
          );
    if (asset.endsWith('.svg')) {
      return SvgPicture.asset(
        asset,
        width: size,
        height: size,
        fit: BoxFit.contain,
        alignment: Alignment.center,
        colorFilter: monochromeTint,
      );
    }
    final image = Image.asset(
      asset,
      width: size,
      height: size,
      fit: BoxFit.contain,
      alignment: Alignment.center,
    );
    return monochromeTint == null
        ? image
        : ColorFiltered(colorFilter: monochromeTint, child: image);
  }
}

class _ProviderLogoInitial extends StatelessWidget {
  const _ProviderLogoInitial({required this.name, required this.size});

  final String name;
  final double size;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final trimmedName = name.trim();
    final initial = trimmedName.isEmpty ? '?' : trimmedName.substring(0, 1);
    return Text(
      initial.toUpperCase(),
      textAlign: TextAlign.center,
      style: Theme.of(context).textTheme.titleSmall?.copyWith(
        fontWeight: FontWeight.w700,
        fontSize: size * 0.78,
        height: 1,
        color: colorScheme.onSurfaceVariant,
      ),
    );
  }
}
