# Theme template renderer boundary

ToolPkg transport, script protection, installation provenance and package ownership belong to the shared plugin system. Theme validation adds no independent package type or installation lane.

Each declared template owns a bounded list of visual resources. The validator checks actual resource sizes, visual schema and relations, referenced asset closure, image dimensions, restricted static SVG structures, nine-slice pixel bounds, and static TrueType tables and glyph traversal. Resource lengths and cache digests are computed from actual bytes; authors only declare paths and media types. Legacy length and digest fields are ignored. Only referenced resources enter media decoding; unrelated scripts and resources are retained by the package system.

Definition and preview references are safe package-relative paths. Asset references inside the visual definition resolve relative to that definition. Validation returns normalized renderer paths and evidence for one template identified by `toolpkg_id:template_id`.

`ThemeTemplateTrust::Untrusted` explicitly makes no claim about publisher identity, licensing rights or curation. The optional package agreement reference is validated by the ToolPkg parser; it is not a template-specific legal or compatibility requirement.
