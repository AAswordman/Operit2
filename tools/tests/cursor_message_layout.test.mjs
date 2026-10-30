import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../apps/flutter/app/lib/ui/', import.meta.url);

/** Reads the production Flutter source without starting a native runtime. */
function source(path) {
  return readFileSync(new URL(path, root), 'utf8');
}

/** Keeps avatar settings beside the message styles rather than input behavior. */
test('avatar visibility and shape belong to messages and bubbles', () => {
  const panel = source('features/settings/appearance/AppearanceSettingsPanel.dart');
  const bubblesStart = panel.indexOf('Widget _buildBubblesTab(');
  const interactionStart = panel.indexOf('Widget _buildInteractionTab(');
  const panelEnd = panel.indexOf('\nclass ', interactionStart);
  assert.ok(bubblesStart >= 0 && interactionStart > bubblesStart && panelEnd > interactionStart);
  const bubbles = panel.slice(bubblesStart, interactionStart);
  const interaction = panel.slice(interactionStart, panelEnd);
  for (const setting of ['settingsAppearanceShowAvatars', 'settingsAppearanceAvatarShape']) {
    assert.ok(bubbles.includes(setting), `Missing message setting: ${setting}`);
    assert.ok(!interaction.includes(setting), `Misplaced interaction setting: ${setting}`);
  }
  assert.match(bubbles, /saveThemeSettings\(bubbleShowAvatar: value\)/);
});

/** Keeps command-style messages avatar-free regardless of bubble preferences. */
test('command user and assistant rows do not render avatars', () => {
  for (const name of ['AiMessageComposable', 'UserMessageComposable']) {
    const message = source(`features/chat/components/style/cursor/${name}.dart`);
    assert.match(message, /return (?:MouseRegion|CursorMessageLayout)\(/);
    assert.match(message, /CursorMessageLayout\(/);
    assert.doesNotMatch(message, /bubbleShowAvatar|CursorMessageAvatar|ChatAvatarImage|ThemeAssetImage/);
  }
});

/** Gives both command-style roles the original inset without an avatar column. */
test('command rows use one inset with no avatar slot or gap', () => {
  const layout = source('features/chat/components/style/cursor/CursorMessageLayout.dart');
  assert.match(layout, /EdgeInsets\.symmetric\(horizontal: 16, vertical: verticalPadding\)/);
  assert.match(layout, /child: child/);
  assert.doesNotMatch(layout, /avatar|Avatar|Row\(|SizedBox\(/);
  const assistant = source('features/chat/components/style/cursor/AiMessageComposable.dart');
  assert.match(assistant, /padding: const EdgeInsets\.only\(top: 6\),\s*child: _CursorAiMessageHeader\(/);
});
