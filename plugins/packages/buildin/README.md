# Built-In Plugins

ToolPkg sources in this folder are packaged into Operit2 built-in plugin assets.

`image_workbench/` provides the unified image-generation plugin and native workbench.
Its single `image_workbench` tool namespace owns the six internal provider adapters;
the former standalone `*_draw.ts` entries have been removed from `external/`.
