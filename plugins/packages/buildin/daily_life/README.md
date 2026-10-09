# Daily Life Toolkit

`daily_life` is a standard ToolPkg. The container is `com.operit.daily_life`; the subpackage remains `daily_life`, so tool names stay `daily_life:<tool>`.

## Connected tools

- `get_current_date`: current date/time from the plugin runtime.
- `device_status`: existing system Host device information.
- `take_screenshot`: existing `capture_screenshot` Host tool.
- `search_weather`: readable weather information from a search page through the existing web-visit Host.
- `set_reminder`: persistent in-app notification reminder.
- `list_reminders`: stored reminder records and delivery state.
- `cancel_reminder`: cancel a pending reminder by the ID returned by `set_reminder`.

## Networking boundary

Weather requests use `Tools.Net.visit`, backed by the existing `visit_web` Host. The plugin searches Baidu for `<location> weather` and returns the Host's `VisitWebResultData` unchanged. It does not use a separate weather API, `fetch`, shell commands, platform networking clients or platform-specific branches.

Web-visit errors are reported directly, without switching routes or data sources. The regression tests only mock web-visit Host responses and do not contact the network. The earlier web-visit timeout has not been diagnosed or fixed by changing this plugin.

## Reminder semantics

`set_reminder` requires a future ISO 8601 date-time with an explicit `Z` or UTC offset:

```json
{
  "title": "喝水",
  "description": "记得喝一杯水",
  "due_date": "2030-01-01T08:01:00+08:00"
}
```

This creates an **in-app notification reminder**, not a system alarm or a record in a system reminder application. Its returned `pending` state means the record has been persisted, not that a notification has already been sent.

One main-runtime owner serializes tool requests and scheduled delivery. Records are stored through the existing `PluginConfig` API in the container-scoped `reminders.json`. The existing Host interval event checks due records every 60 seconds while the plugin is enabled and Host events are active. Application startup and resume events also check persisted overdue records. Delivery depends on Host event execution, notification permission and the host's background scheduling behavior; exact-time execution while the application is inactive or terminated is not promised.

States are `pending`, `delivering`, `delivered`, `cancelled`, and `failed`. A delivery claim is persisted before calling the notification Host, preventing concurrent clock events from sending the same record. `delivered` means the notification Host accepted the request; it does not prove the user saw the notification. A Host notification failure is stored and surfaced, without automatic resending. An interrupted `delivering` record is reported as a failed delivery with an unknown outcome after restart, also without resending. Persistent-storage failures stop delivery in that runtime; no uncommitted reminder is treated as successfully scheduled.

## Not connected

The previous natural-language UI-subagent implementations have been removed. System alarms, SMS, calls, WeChat Moments, flashlight, device volume, Wi-Fi, photo capture and system dark mode are not advertised as available tools. They require real Host or application-integration capabilities and must not be implemented by dispatching a natural-language subagent.

## Checks and packaging

From the repository root:

```powershell
node --test tools/tests/daily_life_plugin.test.mjs
node plugins/packages/buildin/workflow/node_modules/typescript/bin/tsc -p plugins/packages/buildin/daily_life/tsconfig.json --noEmit
```

The existing plugin sync process discovers `manifest.json`, emits `dist/*.js` using this package's `tsconfig.json`, and packages the directory as `daily_life.toolpkg`. The former single-file `daily_life.js` output is no longer part of the sync plan. No core or Flutter capability changes are required.

## ESP32 diagnostic exports

The existing main runtime exports `test_connection` for a no-side-effect
plugin round trip, and `test_tool_call` which uses `toolCall` to execute
`daily_life:get_current_date` through Core's existing tool permission/dispatch
chain. Tests return only `{passed, message?}`; their raw date payload is not
sent to the device. These exports do not register another plugin API.
