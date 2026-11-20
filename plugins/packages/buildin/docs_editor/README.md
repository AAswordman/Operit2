# Office documents

This built-in plugin is packaged from `plugins/packages/buildin/docs_editor` and is disabled by default.

Enable Office documents in the current space and open a `.docx`, `.pptx` or `.pdf` in the workspace. The `office` tool subpackage is enabled by default when the plugin is enabled. Use the fixed three-dot button at the right of a file tab, or right-click the tab, to choose **Open with** and switch between Office and the built-in preview. Office edits the file in the tab, and saves back to the original workspace path. Working copies are limited to 128 MiB. Reading failures report their actual cause separately from size-limit errors.

Save, quote selection, and save status share the editor's toolbar. The plugin does not add a separate filename header above it; the workspace file tab identifies the document.

Documents support two-finger touch, Ctrl+wheel, trackpad pinch, zoom buttons and fit width. Word centers its pages in the pane and keeps fitting as the pane width changes, until a manual zoom is chosen. Zoom changes the view without marking the file as edited.

The quote button adds the selection as an additional-information attachment to the current conversation draft, without changing the message text or sending it. To edit through the conversation, call `office:list`, `office:read`, and then `office:edit` using the exact revision and a structured edit. `office:save` explicitly persists the result. The plugin never invokes a model and does not configure or restrict the host model. The editors and their assets run offline.

- DOCX: paginated editing with SuperDoc, built-in formatting controls, headers and footers, and DOCX export. The tools can read the body, selected text, header/footer parts and embedded images. Find-and-replace targets exact text; duplicate matches require a zero-based `occurrence`. Image parts are returned in base64 chunks, not submitted automatically as vision inputs.
- PPTX: slide and shape editing via the Office Kit editor. Read `body`, `shapes:<zero-based index>` or `tools` for the supported structured commands.
- PDF: offline rendering, text selection and extraction, annotations, and supported form filling. The tool creates notes; it does not rewrite text painted into an existing page. This is not an OCR engine.

Legacy `.doc` / `.ppt` files and exact rendering parity for every Office feature are not supported. Font availability affects layout. Save fails rather than silently overwriting a file changed by another editor or tool.

Build with `pnpm install --frozen-lockfile` and `pnpm pack:toolpkg`.
