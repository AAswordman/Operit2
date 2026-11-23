# Built-In Plugins

ToolPkg sources in this folder are packaged into Operit2 built-in plugin assets.

`super_admin/` owns terminal commands and closes its idle sessions after ten
minutes using a plugin-owned Host clock and atomic conditional close.

`automatic_ui_base/` is the UI Automation ToolPkg (`com.operit.ui_automation`).
It is disabled by default and exposes the `Automatic_ui_base` subpackage.
It owns the automation indicator and its lifecycle. UI actions use the system
Host provider. Screenshots use the system capture tool and VFS media registration.
