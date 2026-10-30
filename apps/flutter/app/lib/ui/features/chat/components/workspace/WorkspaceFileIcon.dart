// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';

const String _kSvgLangPython = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M11.91 2c-5.06 0-4.73 2.19-4.73 2.19l.01 2.27h4.81v.68H5.21S2 6.77 2 11.88s2.8 5 2.8 5h1.67v-2.33s-.09-2.8 2.76-2.8h4.74s2.66.04 2.66-2.6V4.6s.4-2.6-4.72-2.6zm-2.58 1.48a.86.86 0 1 1 0 1.72.86.86 0 0 1 0-1.72z" fill="#3776AB"/>
<path d="M12.09 22c5.06 0 4.73-2.19 4.73-2.19l-.01-2.27h-4.81v-.68h6.78s3.21.37 3.21-4.74-2.8-5-2.8-5h-1.67v2.33s.09 2.8-2.76 2.8H10.1s-2.66-.04-2.66 2.6V19.4s-.4 2.6 4.72 2.6zm2.58-1.48a.86.86 0 1 1 0-1.72.86.86 0 0 1 0 1.72z" fill="#FFD43B"/>
</svg>
''';

const String _kSvgLangRust = '''
<svg viewBox="0 0 24 24" fill="#DEA584" xmlns="http://www.w3.org/2000/svg">
<path d="M23.83 12c0-.29-.02-.57-.05-.84l-1.92-.32a9.98 9.98 0 0 0-.58-1.41l1.33-1.44a11.96 11.96 0 0 0-1.23-1.9l-1.84.66a9.97 9.97 0 0 0-1.08-1.08l.66-1.84a11.97 11.97 0 0 0-1.9-1.23l-1.44 1.33a10.03 10.03 0 0 0-1.41-.59l-.32-1.92A11.75 11.75 0 0 0 12 .17c-.29 0-.57.02-.84.05l-.32 1.92a9.98 9.98 0 0 0-1.41.58l-1.44-1.33a11.96 11.96 0 0 0-1.9 1.23l.66 1.84a9.98 9.98 0 0 0-1.08 1.08l-1.84-.66a11.97 11.97 0 0 0-1.23 1.9l1.33 1.44c-.23.45-.43.93-.59 1.41l-1.92.32a11.77 11.77 0 0 0-.05.85c0 .28.02.56.05.84l1.92.32c.16.49.36.96.59 1.41l-1.33 1.44a11.96 11.96 0 0 0 1.23 1.9l1.84-.66c.34.4.7.76 1.08 1.08l-.66 1.84a11.97 11.97 0 0 0 1.9 1.23l1.44-1.33c.45.23.93.43 1.41.59l.32 1.92c.28.03.56.05.84.05.29 0 .57-.02.84-.05l.32-1.92a10.02 10.02 0 0 0 1.41-.59l1.44 1.33a11.96 11.96 0 0 0 1.9-1.23l-.66-1.84a9.98 9.98 0 0 0 1.08-1.08l1.84.66a11.97 11.97 0 0 0 1.23-1.9l-1.33-1.44c.23-.45.43-.93.59-1.41l1.92-.32c.03-.28.05-.56.05-.84zm-11.83 6.64a6.64 6.64 0 1 1 0-13.28 6.64 6.64 0 0 1 0 13.28zm-2.02-9.61h2.24c1.17 0 1.91.56 1.91 1.48 0 .76-.48 1.28-1.24 1.43l1.43 2.7h-1.28l-1.28-2.48h-.6v2.48H9.98V9.03zm1.18 2.11h.96c.55 0 .9-.25.9-.66 0-.42-.35-.66-.9-.66h-.96v1.32z"/>
</svg>
''';

const String _kSvgLangC = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M12 2.5l8.23 4.75v9.5L12 21.5l-8.23-4.75v-9.5L12 2.5z" fill="#00599C"/>
<path d="M14.8 8.8a4.2 4.2 0 0 0-5.6 0 4 4 0 0 0 0 6.4 4.2 4.2 0 0 0 5.6 0" stroke="#FFFFFF" stroke-width="2.4" stroke-linecap="round"/>
</svg>
''';

const String _kSvgLangCpp = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M12 2.5l8.23 4.75v9.5L12 21.5l-8.23-4.75v-9.5L12 2.5z" fill="#004482"/>
<path d="M12 9.2a3.5 3.5 0 0 0-4.8 0 3.4 3.4 0 0 0 0 5.6 3.5 3.5 0 0 0 4.8 0" stroke="#FFFFFF" stroke-width="2.2" stroke-linecap="round"/>
<path d="M14 12h2.5M15.25 10.75v2.5M17.5 12h2.5M18.75 10.75v2.5" stroke="#659AD2" stroke-width="1.5" stroke-linecap="round"/>
</svg>
''';

const String _kSvgLangDart = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M4.1 3.5L3 13.2l5.7 5.7 8.2-8.2L4.1 3.5z" fill="#0175C2"/>
<path d="M16.9 10.7l-8.2 8.2 2.1 2.1c1.2 1.2 3.1 1.2 4.3 0l5.8-5.8-4-4.5z" fill="#02569B"/>
<path d="M12.9 2.5c-1.2-1.2-3.1-1.2-4.3 0L3 8.1l1.1 5.1 12.8-2.5-4-8.2z" fill="#00B4AB"/>
<path d="M16.9 10.7l4 4.5 1.1-5.1-5.1-7.6-4 8.2 4 0z" fill="#29B6F6"/>
</svg>
''';

const String _kSvgLangTs = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="22" height="22" x="1" y="1" rx="4" fill="#3178C6"/>
<path d="M11 9H5v2.2h2V18h2.3v-6.8H11V9zm3.8 4.7c.8.6 1.7.9 2.7.9 1 0 1.6-.4 1.6-1 0-.6-.5-.9-1.8-1.4-1.7-.6-2.8-1.5-2.8-3 0-1.8 1.4-3.2 3.6-3.2 1.3 0 2.3.4 3.1 1l-1 1.8c-.7-.5-1.4-.7-2.1-.7-.8 0-1.3.4-1.3.9 0 .6.5.8 1.7 1.3 1.9.7 2.9 1.6 2.9 3.2 0 1.9-1.4 3.3-3.8 3.3-1.6 0-2.8-.5-3.8-1.2l1-1.9z" fill="#FFFFFF"/>
</svg>
''';

const String _kSvgLangJs = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="22" height="22" x="1" y="1" rx="4" fill="#F7DF1E"/>
<path d="M7.5 17.5c1 .6 2.1.9 3.2.9 1.8 0 2.8-.9 2.8-2.5V9H11v6.9c0 .6-.3.9-.9.9-.4 0-.8-.1-1.2-.3l-1.4 1zM15 17.2c1 .6 2.1 1 3.4 1 2.2 0 3.6-1.1 3.6-2.8 0-1.6-1-2.4-2.6-3-1.1-.5-1.6-.8-1.6-1.3 0-.5.4-.8 1.2-.8.8 0 1.5.3 2.1.7l1.1-1.8c-.9-.6-2-1-3.2-1-2.1 0-3.5 1.2-3.5 2.8 0 1.5 1 2.4 2.5 3 1.1.5 1.7.8 1.7 1.4 0 .6-.5.9-1.4.9-.9 0-1.8-.3-2.5-.8l-1.3 1.9z" fill="#000000"/>
</svg>
''';

const String _kSvgLangGo = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="22" height="22" x="1" y="1" rx="5" fill="#00ADD8"/>
<path d="M10.8 12.3H8.3v1.8h4.5c-.4 2.3-2 3.6-4.5 3.6-2.9 0-5.1-2.1-5.1-5.4s2.2-5.4 5.1-5.4c1.8 0 3.1.8 3.9 1.8l-1.6 1.4c-.6-.7-1.3-1.2-2.3-1.2-1.7 0-3 1.4-3 3.4s1.3 3.4 3 3.4c1.2 0 2-.6 2.4-1.5H10.8v-1.9zm8.4-.3c0 3.3-2.3 5.7-5.5 5.7s-5.5-2.4-5.5-5.7 2.3-5.7 5.5-5.7 5.5 2.4 5.5 5.7zm-2.4 0c0-2-1.3-3.6-3.1-3.6s-3.1 1.6-3.1 3.6 1.3 3.6 3.1 3.6 3.1-1.6 3.1-3.6z" fill="#FFFFFF"/>
</svg>
''';

const String _kSvgLangJava = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M4 10h12v6a4 4 0 0 1-4 4H8a4 4 0 0 1-4-4v-6zm12 2h2a2 2 0 0 1 2 2v0a2 2 0 0 1-2 2h-2v-4z" stroke="#EA2D2E" stroke-width="2" stroke-linecap="round"/>
<path d="M7 6c0-1.5 1-2.5 1-4M10 6c0-1.5 1-2.5 1-4M13 6c0-1.5 1-2.5 1-4" stroke="#5382A1" stroke-width="1.8" stroke-linecap="round"/>
<path d="M2 21h18" stroke="#EA2D2E" stroke-width="2" stroke-linecap="round"/>
</svg>
''';

const String _kSvgLangKotlin = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="22" height="22" x="1" y="1" rx="4" fill="#7F52FF"/>
<path d="M4 20L12 12 4 4v16zM12 12l8 8H12v-8zM12 12l8-8H12v8z" fill="#FFFFFF" fill-opacity="0.85"/>
</svg>
''';

const String _kSvgLangSwift = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="22" height="22" x="1" y="1" rx="5" fill="#F05138"/>
<path d="M19.2 17.5c-3.1 2.2-7.3 1.8-10.4-.9 4.2 1.4 8.7-.3 10.4-.9-2.3-1.6-4.5-4-5.2-6.8 1.8 1.4 4.1 2 6.4 1.4-1.9-2.6-4.9-5-8.5-5.8 4.2.9 7.8 3.5 9.7 7.2-.6-1.5-1.5-2.8-2.6-3.9 3.2 2.7 4.1 7.1.2 9.7z" fill="#FFFFFF"/>
</svg>
''';

const String _kSvgLangHtml = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M4 3l1.5 16.5L12 21.5l6.5-2L20 3H4z" fill="#E34F26"/>
<path d="M12 4.5v15.2l5.1-1.5 1.2-13.7H12z" fill="#EF652A"/>
<path d="M12 7.8H8.2l.2 2.5h3.6V7.8zm0 5h-2.1l-.1-1.3H8.3l.3 3.6h3.4v-2.3zm0-5v2.5h3.4l-.3 3.5-3.1.8v2.4l5.3-1.5.6-7.7H12z" fill="#FFFFFF"/>
</svg>
''';

const String _kSvgLangCss = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M4 3l1.5 16.5L12 21.5l6.5-2L20 3H4z" fill="#1572B6"/>
<path d="M12 4.5v15.2l5.1-1.5 1.2-13.7H12z" fill="#33A9DC"/>
<path d="M12 7.8H8.2l.2 2.5h3.6V7.8zm0 5h-3.5l.3 3.6 3.2.9v-2.4l-1.4-.4-.1-1.7H12v-0zm0-5v2.5h3.4l-.3 3.5-3.1.8v2.4l5.3-1.5.6-7.7H12z" fill="#FFFFFF"/>
</svg>
''';

const String _kSvgLangJson = '''
<svg viewBox="0 0 24 24" fill="none" stroke="#CB85E8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">
<path d="M9 5c-1.5 0-2 1-2 2v3c0 1-1 2-2 2 1 0 2 1 2 2v3c0 1 .5 2 2 2M15 5c1.5 0 2 1 2 2v3c0 1 1 2 2 2-1 0-2 1-2 2v3c0 1-.5 2-2 2"/>
</svg>
''';

const String _kSvgLangYaml = '''
<svg viewBox="0 0 24 24" fill="none" stroke="#E5C07B" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">
<path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6"/>
</svg>
''';

const String _kSvgLangMarkdown = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="22" height="16" x="1" y="4" rx="3" stroke="#79B8FF" stroke-width="2"/>
<path d="M4 15V9l2.5 3L9 9v6M13 12h3M14.5 9.5v5.5m0 0l-2-2m2 2l2-2" stroke="#79B8FF" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
''';

const String _kSvgLangShell = '''
<svg viewBox="0 0 24 24" fill="none" stroke="#5391FE" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">
<polyline points="4 17 10 11 4 5"/>
<line x1="12" y1="19" x2="20" y2="19"/>
</svg>
''';

const String _kSvgLangSql = '''
<svg viewBox="0 0 24 24" fill="none" stroke="#E5C07B" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">
<ellipse cx="12" cy="5" rx="9" ry="3"/>
<path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/>
</svg>
''';

const String _kSvgLangDocker = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M2.5 12.5h19c0 4.5-3.5 8-8 8-4 0-7.2-2.8-7.9-6.5l-3.1-1.5z" fill="#0db7ed"/>
<rect x="5" y="9" width="2" height="2" fill="#0db7ed"/>
<rect x="8" y="9" width="2" height="2" fill="#0db7ed"/>
<rect x="11" y="9" width="2" height="2" fill="#0db7ed"/>
<rect x="8" y="6" width="2" height="2" fill="#0db7ed"/>
<rect x="11" y="6" width="2" height="2" fill="#0db7ed"/>
<rect x="14" y="9" width="2" height="2" fill="#0db7ed"/>
</svg>
''';

const String _kSvgLangGit = '''
<svg viewBox="0 0 24 24" fill="none" stroke="#F05032" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">
<circle cx="6" cy="6" r="3"/>
<circle cx="6" cy="18" r="3"/>
<circle cx="18" cy="9" r="3"/>
<line x1="6" y1="9" x2="6" y2="15"/>
<path d="M6 12a9 9 0 0 0 9-3"/>
</svg>
''';

const String _kSvgLangLock = '''
<svg viewBox="0 0 24 24" fill="none" stroke="#8B949E" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">
<rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
<path d="M7 11V7a5 5 0 0 1 10 0v4"/>
</svg>
''';

const String _kSvgLangDefault = '''
<svg viewBox="0 0 24 24" fill="none" stroke="#8B949E" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">
<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
<polyline points="14 2 14 8 20 8"/>
</svg>
''';

/// Builds a language icon shared by ordinary workspace and tool file previews.
Widget buildWorkspaceFileIcon(String name, {double size = 14}) {
  final ext = name.contains('.') ? name.split('.').last.toLowerCase() : '';
  final lowerName = name.toLowerCase();

  if (lowerName == 'dockerfile' || lowerName.startsWith('dockerfile.')) {
    return SvgPicture.string(_kSvgLangDocker, width: size, height: size);
  }
  if (lowerName.startsWith('.git') ||
      lowerName == '.gitignore' ||
      lowerName == '.gitmodules') {
    return SvgPicture.string(_kSvgLangGit, width: size, height: size);
  }

  final svgString = switch (ext) {
    'c' || 'h' => _kSvgLangC,
    'cpp' || 'cc' || 'cxx' || 'hpp' || 'hxx' => _kSvgLangCpp,
    'py' || 'pyw' || 'ipynb' => _kSvgLangPython,
    'rs' => _kSvgLangRust,
    'dart' => _kSvgLangDart,
    'ts' || 'tsx' || 'mts' || 'cts' => _kSvgLangTs,
    'js' || 'jsx' || 'mjs' || 'cjs' => _kSvgLangJs,
    'go' => _kSvgLangGo,
    'java' || 'class' || 'jar' => _kSvgLangJava,
    'kt' || 'kts' => _kSvgLangKotlin,
    'swift' => _kSvgLangSwift,
    'html' || 'htm' => _kSvgLangHtml,
    'css' || 'scss' || 'less' => _kSvgLangCss,
    'json' || 'jsonc' => _kSvgLangJson,
    'yaml' || 'yml' || 'toml' => _kSvgLangYaml,
    'md' || 'markdown' => _kSvgLangMarkdown,
    'sh' || 'bash' || 'zsh' || 'ps1' || 'bat' || 'cmd' => _kSvgLangShell,
    'sql' => _kSvgLangSql,
    'lock' => _kSvgLangLock,
    _ => _kSvgLangDefault,
  };

  return SvgPicture.string(svgString, width: size, height: size);
}
