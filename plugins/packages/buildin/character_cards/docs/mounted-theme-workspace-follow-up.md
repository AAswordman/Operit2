# Confirmed mounted theme/workspace consumers and minimal follow-up

This is a source-level review of actual mounted call chains, not Flutter execution
or an implemented replacement contract. No Flutter production file was changed
for this review. The metadata/sidebar/snapshot slices are **not** whole-feature
extraction acceptance.

## Live paths that block deleting Core manager exports

Paths below are relative to `apps/flutter/app/lib/`.

| Mounted chain | Confirmed live old-domain dependency | Status |
| --- | --- | --- |
| `ui/main/OperitApp.dart:87` → `OperitTheme(...)` → `ui/theme/OperitTheme.dart:145` `_controller.start()` | `start()` reads `preferencesActivePromptManager.getActivePrompt` and subscribes to `activePromptFlow` at 463/470; resolves names through card/group managers at 852/857 | **Live application-startup blocker**, not an unmounted migration-reference file |
| Settings category → `ui/features/settings/components/SettingsDetailView.dart:44` → `AppearanceSettingsPanel` → target picker | `appearance/AppearanceSettingsPanel.dart:1553–1554` calls `loadThemeCharacterCards` / `loadThemeCharacterGroups`; controller calls `getAllCharacterCards` / `getAllCharacterGroupCards` at 715–722 and writes active prompt at 744 | **Live settings consumer**. The current panel is named `AppearanceSettingsPanel`, not `ThemeSettingsPanel` |
| Appearance panel → `appearance/ChatAppearancePreview.dart` | `didChangeDependencies` interprets active character target and calls `loadThemeCharacterCards`; preview resolves a `CharacterCard` avatar | **Live preview consumer**, not merely an old settings filename |
| `ui/features/chat/screens/AIChatScreen.dart:2339,2552` → `components/WorkspaceShell.dart:84` → `workspace/WorkspacePanel.dart` | `WorkspacePanel.initState` starts card-list subscriptions and a card load at 134–135; loaders use `preferencesCharacterCardManager` at 730–809; workspace aggregation interprets `ChatHistory.characterCardName` at 1038 | **Live workspace consumer**, including startup of the panel before its overview is interacted with |
| Theme controller → `data/preferences/UserPreferencesManager.dart` | Public theme write/delete/copy paths still take `characterCardId` / `characterGroupId` and domain-specific prefixes | **Scope-shape blocker**; removing the manager alone does not make the preference boundary generic |

`SettingsDetailView` does not mount the old `MemorySettingsPanel` simply because
that file still exists. The same distinction must be used for other abandoned
card/memory settings files: require an actual import plus constructor/callback
path from the settings route before reporting a live consumer.

The current sidebar checker follows Main → Phone/Tablet → Drawer →
`ChatSidebarTabHost` and its registered route catalog. Its passing result must not
be generalized to these separate theme/workspace chains. This review does not
remove or weaken its existing negative fixtures.

## Minimal follow-up proposal for main-thread approval

The following are scope proposals, **not locked API names or shipped SDK methods**.
They deliberately reuse `registerApi`, the enabled package catalog and registered
UI routes. They add no CharacterCards Host/DTO, universal provider bus, or new
plugin storage framework.

### 1. Theme: separate global appearance from plugin-owned scope selection

- Host keeps the application's ordinary global Material theme and appearance
  preferences. Starting it must not require a selected card/group or an old
  active-prompt manager. Global appearance is its own explicit scope, not a
  recovery value for an unsuccessful plugin request.
- The plugin owns card/group theme targets and their association with its active
  namespace. Host must not infer them from `ActivePrompt.tag` or card/group IDs.
- A necessary reusable theme scope reference would be narrowly typed as an
  authenticated package owner plus an opaque scope key; displayed target metadata
  is generic title/icon and an owned registered route with opaque input. Exact
  field/API naming requires main-thread agreement before implementation.
- Existing `chat.context.actions` identity/background projections can supply
  per-chat display identity. They do not currently implement scoped Material
  preference editing or a scope-change subscription; those are explicit gaps,
  not reasons to keep the old manager active.
- Prefer the plugin's registered management/presentation route for its target
  selector and editor. Use the existing completion/cancel protocol; never write
  active selection on cancellation, silently choose a default role, or swallow a
  disabled/unknown owner error.
- `OperitThemeController`, the appearance picker/preview and
  `UserPreferencesManager` scope arguments must change together **before** old
  Core manager/DTO exports are unmounted. No task here edits Linnaeus's files.

### 2. Workspace: retain built-in workspace UI, delegate domain summaries

- Built-in workspace overview owns generic workspace state, folders/files and
  conversation counts. It must stop loading all cards and interpreting
  `ChatHistory.characterCardName`.
- Plugin-specific grouping/usage/identity statistics belong to the plugin. An
  enabled-package `registerApi` projection can take a workspace ID and generic
  chat summaries, returning narrow display items (opaque ID, title, optional
  avatar/icon, count, owned route descriptor). The plugin obtains associations
  through its own namespace/service; Host never assembles character fields.
- If those domain summaries are interactive panels, use the existing registered
  Compose route embedding and opaque params rather than rendering a fake
  card-specific section in Core. No new surface is necessary merely to move
  grouping into the already registered sidebar route.
- Main must decide the minimum actual reusable workspace contribution contract
  before implementation. A generic projection proposal does not mean a runtime
  workspace API has already been registered or consumed.
- Unknown/disabled ownership, duplicate/missing routes and provider failures must
  be explicit errors. Absence of registered participants is a genuine empty
  catalog; an invoked provider failure is not an empty catalog.

## Acceptance still required

1. Remove old live calls from all listed controllers, picker/preview and workspace
   loaders; do not just delete a dialog file or Core re-export.
2. Verify two arbitrary enabled plugins, ordering/localization, owner/route
   validation, immutable params, disabled/reload behavior and error propagation.
3. Verify legitimate app restart/runtime rebind and presentation cancellation do
   not create a default plugin association.
4. Imported Operit1 chats start with empty extension namespaces. Later explicit
   user binding happens through the plugin; theme/workspace must not restore a
   legacy/default marker merely to display those conversations.
5. Re-run the actual mounted-chain check and Flutter integration when execution
   is permitted. No Flutter compilation or runtime test has been performed here.
