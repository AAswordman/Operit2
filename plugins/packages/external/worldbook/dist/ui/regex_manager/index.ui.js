"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.default = RegexManager;
const index_js_1 = require("../../i18n/index.js");
const layout_js_1 = require("../layout.js");
const worldbook_regex_js_1 = require("../../shared/worldbook_regex.js");
const worldbook_service_js_1 = require("../../shared/worldbook_service.js");
/** Manages ordered rules with ordinary fields, explicit imports, exports, and isolated previews. */
function RegexManager(ctx) {
    const { UI } = ctx;
    const layout = (0, layout_js_1.useWorldBookLayout)(ctx);
    const t = (0, index_js_1.currentWorldBookI18n)().regex;
    const [rules, setRules] = ctx.useState("regex.rules", []);
    const [loaded, setLoaded] = ctx.useState("regex.loaded", false);
    const [busy, setBusy] = ctx.useState("regex.busy", false);
    const [draft, setDraft] = ctx.useState("regex.draft", null);
    const [cards, setCards] = ctx.useState("regex.cards", []);
    const [editing, setEditing] = ctx.useState("regex.editing", false);
    const [importing, setImporting] = ctx.useState("regex.importing", false);
    const [importText, setImportText] = ctx.useState("regex.importText", "");
    const [preview, setPreview] = ctx.useState("regex.preview", "");
    const [previewResult, setPreviewResult] = ctx.useState("regex.previewResult", "");
    const [previewError, setPreviewError] = ctx.useState("regex.previewError", "");
    const [previewStage, setPreviewStage] = ctx.useState("regex.previewStage", "display");
    const [previewPlacement, setPreviewPlacement] = ctx.useState("regex.previewPlacement", 2);
    const [previewDepth, setPreviewDepth] = ctx.useState("regex.previewDepth", "0");
    const [previewEdit, setPreviewEdit] = ctx.useState("regex.previewEdit", false);
    const [deleteId, setDeleteId] = ctx.useState("regex.deleteId", "");
    const live = ctx.useRef("regex.live", { rules, draft, importText, preview, previewStage, previewPlacement, previewDepth, previewEdit, busy });
    live.current = { rules, draft, importText, preview, previewStage, previewPlacement, previewDepth, previewEdit, busy };
    /** Runs one user operation and keeps validation errors visible without losing form content. */
    async function act(operation) {
        if (live.current.busy)
            return;
        live.current.busy = true;
        setBusy(true);
        try {
            await operation();
        }
        catch (error) {
            ctx.showToast(String(error));
        }
        finally {
            live.current.busy = false;
            setBusy(false);
        }
    }
    /** Replaces form state synchronously so saving uses the latest typed value. */
    function change(patch) {
        if (!live.current.draft)
            return;
        const next = { ...live.current.draft, ...patch };
        live.current.draft = next;
        setDraft(next);
        setPreviewResult("");
        setPreviewError("");
    }
    /** Saves an ordered list only after the entire configuration validates. */
    async function persist(next) {
        await (0, worldbook_regex_js_1.writeRegexRules)(next);
        live.current.rules = next;
        setRules(next);
    }
    /** Opens a detached form without editing the persisted rule in place. */
    function open(rule, existing) {
        const copy = { ...rule, placement: [...rule.placement], trimStrings: [...rule.trimStrings] };
        live.current.draft = copy;
        setDraft(copy);
        setEditing(existing);
        setPreviewResult("");
        setPreviewError("");
    }
    /** Saves the current form while preserving its original execution position. */
    async function save() {
        const value = (0, worldbook_regex_js_1.validateRegexRule)(live.current.draft);
        const current = live.current.rules;
        const next = editing ? current.map(rule => rule.id === value.id ? value : rule) : [...current, value];
        await persist(next);
        setDraft(null);
        ctx.showToast(t.saved);
    }
    /** Moves one rule without changing IDs or otherwise reordering the list. */
    async function move(id, delta) {
        const next = [...live.current.rules];
        const index = next.findIndex(rule => rule.id === id);
        const target = index + delta;
        if (index < 0 || target < 0 || target >= next.length)
            return;
        [next[index], next[target]] = [next[target], next[index]];
        await persist(next);
    }
    /** Reads the host-selected JSON file into the import review field. */
    async function pickImport() {
        const result = await ctx.openFilePicker({ mimeTypes: ["application/json", "text/plain"], allowMultiple: false });
        if (result.cancelled || result.files.length === 0)
            return;
        const file = result.files[0];
        const path = file.path ?? file.uri;
        if (!path)
            throw new Error(t.pickerPathMissing);
        const read = await Tools.Files.read(path);
        if (typeof read.content !== "string")
            throw new Error(t.importReadFailed);
        live.current.importText = read.content;
        setImportText(read.content);
    }
    /** Exports the exact stored order through the host save selector. */
    async function exportRules(selected) {
        await ctx.saveFile({ suggestedName: "worldbook-regex.json", content: JSON.stringify(selected, null, 2), mimeType: "application/json", extensions: ["json"] });
    }
    /** Tests a detached rule without saving text, advancing variables, or touching chat history. */
    function testRule() {
        try {
            const state = live.current;
            const rule = (0, worldbook_regex_js_1.validateRegexRule)(state.draft);
            const depth = Number(state.previewDepth);
            if (!Number.isInteger(depth) || depth < 0)
                throw new Error(t.previewDepthInvalid);
            const output = (0, worldbook_regex_js_1.applyRegexRules)(state.preview, [rule], {
                stage: state.previewStage, placement: state.previewPlacement, depth,
                isEdit: state.previewEdit, participantId: rule.characterCardId, variables: null,
            });
            setPreviewResult(output);
            setPreviewError("");
        }
        catch (error) {
            setPreviewError(String(error));
        }
    }
    /** Creates a labelled action with a stable key and normal busy handling. */
    function button(key, title, onClick, enabled = true) {
        return UI.OutlinedButton({ key, onClick, enabled: enabled && !busy }, [UI.Text({ text: title })]);
    }
    /** Creates a checkbox with its explanatory label in the same row. */
    function toggle(key, label, value, update) {
        return UI.Row({ key, verticalAlignment: "center", spacing: 8 }, [
            UI.Checkbox({ checked: value, onCheckedChange: update }), UI.Text({ text: label, weight: 1 }),
        ]);
    }
    /** Creates an ordinary editable form field. */
    function field(key, label, value, update, multiline = false) {
        return UI.TextField({ key, label, value, onValueChange: update, fillMaxWidth: true, singleLine: !multiline, minLines: multiline ? 3 : 1 });
    }
    const nodes = [UI.Text({ text: t.title, style: "titleLarge" })];
    if (draft) {
        nodes.push(UI.Text({ text: t.modeHint, style: "bodySmall" }));
        nodes.push(field("name", t.name, draft.scriptName, value => change({ scriptName: value })));
        nodes.push(field("find", t.find, draft.findRegex, value => change({ findRegex: value }), true));
        nodes.push(field("replace", t.replace, draft.replaceString, value => change({ replaceString: value }), true));
        nodes.push(field("trim", t.trim, draft.trimStrings.join("\n"), value => change({ trimStrings: value ? value.split("\n") : [] }), true));
        for (const [placement, label] of [[1, t.user], [2, t.assistant], [5, t.worldInfo], [6, t.thinking]]) {
            nodes.push(toggle(`placement-${placement}`, label, draft.placement.includes(placement), checked => {
                const selected = live.current.draft.placement.filter(item => item !== placement);
                change({ placement: checked ? [...selected, placement] : selected });
            }));
        }
        nodes.push(toggle("enabled", t.enable, !draft.disabled, value => change({ disabled: !value })));
        nodes.push(toggle("display", t.displayOnly, draft.markdownOnly, value => change({ markdownOnly: value })));
        nodes.push(toggle("prompt", t.promptOnly, draft.promptOnly, value => change({ promptOnly: value })));
        nodes.push(toggle("edit", t.runOnEdit, draft.runOnEdit, value => change({ runOnEdit: value })));
        nodes.push(field("minDepth", t.minDepth, draft.minDepth === null ? "" : String(draft.minDepth), value => change({ minDepth: value.trim() ? Number(value) : null })));
        nodes.push(field("maxDepth", t.maxDepth, draft.maxDepth === null ? "" : String(draft.maxDepth), value => change({ maxDepth: value.trim() ? Number(value) : null })));
        nodes.push(UI.Text({ text: t.macros }));
        nodes.push(UI.FlowRow({ spacing: 8, runSpacing: 8 }, [[0, t.macroNone], [1, t.macroRaw], [2, t.macroEscaped]].map(([mode, label]) => button(`macro-${mode}`, `${draft.substituteRegex === mode ? "✓ " : ""}${label}`, () => change({ substituteRegex: mode })))));
        nodes.push(UI.Text({ text: t.character }));
        nodes.push(UI.FlowRow({ spacing: 8, runSpacing: 8 }, [
            button("card-all", `${draft.characterCardId === "" ? "✓ " : ""}${t.global}`, () => change({ characterCardId: "" })),
            ...cards.map(card => button(`card-${card.id}`, `${draft.characterCardId === card.id ? "✓ " : ""}${card.name}`, () => change({ characterCardId: card.id }))),
        ]));
        nodes.push(field("card-id", t.characterId, draft.characterCardId, value => change({ characterCardId: value })));
        const editor = nodes.splice(1);
        nodes.push(UI.Text({ text: t.previewTitle, style: "titleMedium" }));
        nodes.push(field("test-source", t.previewText, preview, value => { live.current.preview = value; setPreview(value); }, true));
        nodes.push(UI.FlowRow({ spacing: 8, runSpacing: 8 }, [["source", t.source], ["display", t.display], ["prompt", t.prompt]].map(([stage, label]) => button(`test-stage-${stage}`, `${previewStage === stage ? "✓ " : ""}${label}`, () => { live.current.previewStage = stage; setPreviewStage(stage); }))));
        nodes.push(UI.FlowRow({ spacing: 8, runSpacing: 8 }, [[1, t.user], [2, t.assistant], [5, t.worldInfo], [6, t.thinking]].map(([placement, label]) => button(`test-placement-${placement}`, `${previewPlacement === placement ? "✓ " : ""}${label}`, () => { live.current.previewPlacement = placement; setPreviewPlacement(placement); }))));
        nodes.push(field("test-depth", t.previewDepth, previewDepth, value => { live.current.previewDepth = value; setPreviewDepth(value); }));
        nodes.push(toggle("test-edit", t.previewEdit, previewEdit, value => { live.current.previewEdit = value; setPreviewEdit(value); }));
        nodes.push(button("test", t.previewRun, testRule));
        nodes.push(UI.Text({ text: previewError || t.previewResult(previewResult) }));
        const previewNodes = nodes.splice(1);
        nodes.push(layout.panes("regex-editor", [
            UI.Column({ key: "regex-fields", fillMaxWidth: true, spacing: 12 }, editor),
            UI.Column({ key: "regex-preview", fillMaxWidth: true, spacing: 12 }, previewNodes),
        ]));
        nodes.push(UI.FlowRow({ spacing: 8, runSpacing: 8 }, [
            button("save", t.save, () => void act(save)),
            button("cancel", t.back, () => setDraft(null)),
        ]));
    }
    else if (importing) {
        nodes.push(UI.Text({ text: t.importHint }));
        nodes.push(button("pick", t.pick, () => void act(pickImport)));
        nodes.push(field("import-json", t.importJson, importText, value => { live.current.importText = value; setImportText(value); }, true));
        nodes.push(button("import", t.import, () => void act(async () => {
            const imported = (0, worldbook_regex_js_1.importRegexRules)(live.current.importText);
            await persist([...live.current.rules, ...imported]);
            setImporting(false);
            ctx.showToast(t.imported(imported.length));
        })));
        nodes.push(button("import-cancel", t.back, () => setImporting(false)));
    }
    else {
        nodes.push(UI.Text({ text: t.listHint, style: "bodySmall" }));
        nodes.push(UI.FlowRow({ spacing: 8, runSpacing: 8 }, [
            button("new", t.add, () => open((0, worldbook_regex_js_1.newRegexRule)(), false)),
            button("open-import", t.importSillyTavern, () => setImporting(true)),
            button("export", t.exportAll, () => void act(() => exportRules(live.current.rules)), rules.length > 0),
        ]));
        if (loaded && rules.length === 0)
            nodes.push(UI.Text({ text: t.empty }));
        const cards = rules.map((rule, index) => {
            return UI.Card({ key: rule.id, fillMaxWidth: true }, [UI.Column({ padding: 12, spacing: 8 }, [
                    UI.Text({ text: `${index + 1}. ${rule.scriptName}`, style: "titleMedium" }),
                    UI.Text({ text: `${rule.disabled ? t.disabled : t.enabled} · ${[...(rule.markdownOnly ? [t.display] : []), ...(rule.promptOnly ? [t.prompt] : []), ...(!rule.markdownOnly && !rule.promptOnly ? [t.source] : [])].join(" / ")} · ${rule.characterCardId ? t.boundCharacter : t.global}` }),
                    button(`edit-${rule.id}`, t.edit, () => open(rule, true)),
                    button(`toggle-${rule.id}`, rule.disabled ? t.enable : t.disable, () => void act(() => persist(live.current.rules.map(item => item.id === rule.id ? { ...item, disabled: !item.disabled } : item)))),
                    UI.FlowRow({ spacing: 8, runSpacing: 8 }, [
                        button(`up-${rule.id}`, t.up, () => void act(() => move(rule.id, -1)), index > 0),
                        button(`down-${rule.id}`, t.down, () => void act(() => move(rule.id, 1)), index + 1 < rules.length),
                        button(`export-${rule.id}`, t.export, () => void act(() => exportRules([rule]))),
                    ]),
                    button(`delete-${rule.id}`, deleteId === rule.id ? t.confirmDelete : t.delete, () => {
                        if (deleteId !== rule.id) {
                            setDeleteId(rule.id);
                            return;
                        }
                        void act(async () => { await persist(live.current.rules.filter(item => item.id !== rule.id)); setDeleteId(""); });
                    }),
                ])]);
        });
        nodes.push(...layout.cardRows("regex-rules", cards));
    }
    return UI.LazyColumn({ fillMaxSize: true, modifier: layout.measure, padding: 12, spacing: 12, onLoad: () => {
            if (loaded)
                return;
            return act(async () => { setRules(await (0, worldbook_regex_js_1.readRegexRules)()); setCards(await (0, worldbook_service_js_1.listWorldBookCharacterCards)()); setLoaded(true); });
        } }, nodes);
}
