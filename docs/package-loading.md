# Package discovery and activation

`RuntimePackageManager::loadAvailablePackages` reads installation activation settings before scanning package sources. The initial scan uses the persisted extension owners directly; it does not depend on an already initialized runtime registry.

Disabled ToolPkg containers remain in the installed catalog. Catalog reads parse the manifest and child package metadata, retaining identity, version, dependencies, scope, environment declarations and logo references. They do not create a JavaScript engine, evaluate the main entry, run `registerToolPkg`, load theme payloads or inflate unrelated archive resources. Authenticated marketplace packages also decode their structured publisher provenance without evaluating JavaScript, so disabled packages can still be updated with ownership checks.

Enabled containers take the full validation and registration path. The same path runs when a user enables a previously disabled container, including when enabling it through a child package or replacing the enabled-package list. Disabling a container revokes its execution contexts and replaces executable declarations with catalog metadata. Child activation preferences remain owned by the container's installation record.

Built-in and installed external scan caches include the registration mode as well as the source signature. An unchanged enabled package reuses its registration result during a rescan; changing its activation invalidates that result. Standalone script packages retain tool script bodies only while enabled. Scanning bundled installation candidates never evaluates their registration scripts.

Startup progress includes enabled packages and attempted registrations. Disabled catalog entries do not publish loading or success rows. Catalog diagnostics remain available through the package manager.

Focused regression tests count actual registration-engine creations across repeated scans, disablement, a fresh package manager and reactivation for both built-in and external ToolPkg containers. They also cover metadata-only child scripts, activation defaults and marketplace provenance decoding.
