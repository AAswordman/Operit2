# Plugin Docs

Notes for Operit2 plugin authors live here.

- `package_builder/`: documentation resources included into the bundled `PackageBuilder` Skill.
- [Plugin creation workflow](package_builder/PLUGIN_CREATION_WORKFLOW.md): current authoring, path validation, installation, same-id updates, testing, and publication.
- [Theme API](package_builder/THEME_API.md): read and observe UI themes; plugins control their own WebView updates.

- [ToolPkg v1 compatibility](V1_COMPATIBILITY.md): contract ownership, runtime adapters, checks, and remaining capability gaps.

- [Prerequisite public APIs](PUBLIC_API.md): manifest-declared single-file clients, explicit public handlers, dependency calls, and the v1 workflow adapter.

- [Device tools (v2)](DEVICE_TOOLS.md): execute canonical tools and plugin tools on an explicit Core device, with discovery, deadlines and cancellation.
