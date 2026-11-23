# Built-In Plugins

ToolPkg sources in this folder are packaged into Operit2 built-in plugin assets.

`automatic_ui_base/` is the UI Automation ToolPkg (`com.operit.ui_automation`).
It is disabled by default and exposes the `Automatic_ui_base` subpackage.
It owns the automation indicator and its lifecycle. UI actions use the system
Host provider. Screenshots use the system capture tool and VFS media registration.

`ai_chat_presentation/` provides the opt-in AI Reply Presentation mode.
