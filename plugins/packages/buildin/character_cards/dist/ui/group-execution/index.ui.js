"use strict";
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);
var __async = (__this, __arguments, generator) => {
  return new Promise((resolve, reject) => {
    var fulfilled = (value) => {
      try {
        step(generator.next(value));
      } catch (e) {
        reject(e);
      }
    };
    var rejected = (value) => {
      try {
        step(generator.throw(value));
      } catch (e) {
        reject(e);
      }
    };
    var step = (x) => x.done ? resolve(x.value) : Promise.resolve(x.value).then(fulfilled, rejected);
    step((generator = generator.apply(__this, __arguments)).next());
  });
};

// src/ui/group-execution/index.ui.ts
var index_ui_exports = {};
__export(index_ui_exports, {
  default: () => Screen
});
module.exports = __toCommonJS(index_ui_exports);

// src/validation.ts
function assertObject(value, label) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error(`${label} \u5FC5\u987B\u662F\u5BF9\u8C61`);
}
function assertString(value, label) {
  if (typeof value !== "string") throw new Error(`${label} \u5FC5\u987B\u662F\u5B57\u7B26\u4E32`);
}
function requireId(value, label) {
  assertString(value, label);
  if (value.trim() === "" || value !== value.trim()) throw new Error(`${label} \u65E0\u6548`);
  return value;
}

// src/group-execution/control.ts
function controlInput(value) {
  assertObject(value, "group control presentation");
  const requestId = requireId(value.requestId, "group control requestId");
  assertObject(value.input, "group control input");
  if (value.input.mode !== "group-execution" || Object.keys(value.input).length !== 2) throw new Error("Group control requires mode and chatId");
  return { requestId, chatId: requireId(value.input.chatId, "group control chatId") };
}
function checkedStatus(value, chatId, submissionId) {
  if (value === null) {
    if (submissionId !== null) throw new Error("Exact group submission returned no status");
    return null;
  }
  assertObject(value, "group execution status");
  requireId(value.submissionId, "group status submissionId");
  if (value.chatId !== chatId || submissionId !== null && value.submissionId !== submissionId) throw new Error("Group status changed its requested identity");
  if (value.status !== "running" && value.status !== "settled" && value.status !== "failed") throw new Error("Unknown group execution status");
  if (value.status === "settled" !== (value.outcome !== null)) throw new Error("Group status and settlement disagree");
  if (value.status === "failed" ? typeof value.error !== "string" : value.error !== null) throw new Error("Group failure status is malformed");
  if (value.outcome !== null && (value.outcome.chatId !== chatId || value.outcome.submissionId !== value.submissionId)) throw new Error("Group outcome changed its submission identity");
  return value;
}
function createGroupControl(requestId, chatId, publish, assertOwner) {
  let state = { value: null, busy: false, loaded: false, error: "", finished: false }, loading = null;
  function update(change) {
    state = { ...state, ...change };
    publish(state);
  }
  function idle() {
    assertOwner();
    if (state.finished || state.busy) throw new Error("Group control is closed or already processing an action");
  }
  function request(action, submissionId) {
    return __async(this, null, function* () {
      idle();
      update({ busy: true, error: "" });
      try {
        const payload = action === "current" ? { action, chatId } : { action, chatId, submissionId };
        const value = yield ToolPkg.ipc.call("character-memory.group-execution", payload, { targetRuntime: "main" });
        assertOwner();
        update({ value: checkedStatus(value, chatId, submissionId), loaded: true });
      } catch (failure) {
        update({ error: String(failure) });
        throw failure;
      } finally {
        update({ busy: false });
      }
    });
  }
  return {
    /** Retains the initial load Promise including its original rejection. */
    load() {
      if (loading === null) loading = request("current", null);
      return loading;
    },
    /** Reads current state only when the user explicitly asks to refresh. */
    refresh() {
      return request("current", null);
    },
    /** Requires the precise displayed state before issuing cancellation or resumption. */
    act(action) {
      idle();
      const value = state.value;
      if (state.error !== "" || value === null) throw new Error("Group controls require a successful status read");
      if (action === "cancel" && value.status !== "running") throw new Error("Only a running submission can be cancelled");
      if (action === "resume" && (value.status !== "settled" || value.outcome === null || value.outcome.status !== "cancelled")) throw new Error("Only a settled cancelled submission can resume");
      return request(action, value.submissionId);
    },
    /** Emits cancellation of the modal only, not cancellation of the group submission. */
    close() {
      idle();
      update({ finished: true });
      return { type: "toolpkg.presentation.cancel", requestId };
    }
  };
}
function renderGroupExecutionScreen(ctx) {
  const [presentation] = ctx.useState("presentation", null), input = controlInput(presentation);
  const [state, publish] = ctx.useState("group-control-state", { value: null, busy: false, loaded: false, error: "", finished: false });
  const owner = ctx.useRef("group-control-owner", JSON.stringify(presentation));
  const controller = ctx.useRef("group-control-controller", null);
  function assertOwner() {
    const [current] = ctx.useState("presentation", null);
    if (JSON.stringify(current) !== owner.current) throw new Error("Group control presentation owner changed");
  }
  assertOwner();
  if (controller.current === null) controller.current = createGroupControl(input.requestId, input.chatId, publish, assertOwner);
  const control = controller.current, value = state.value;
  const labels = { running: "\u6B63\u5728\u6267\u884C", settled: "\u5DF2\u7ED3\u675F", failed: "\u6267\u884C\u5931\u8D25" };
  const outcomes = { completed: "\u5DF2\u5B8C\u6210", cancelled: "\u5DF2\u53D6\u6D88\uFF0C\u53EF\u7EE7\u7EED", blocked: "\u53D1\u9001\u88AB\u963B\u6B62", consumed: "\u53D1\u9001\u88AB\u5176\u4ED6\u5904\u7406\u5668\u63A5\u7BA1", not_persisted: "\u6D88\u606F\u672A\u4FDD\u5B58" };
  const enabled = state.loaded && !state.busy && !state.finished && state.error === "";
  return ctx.UI.Dialog({
    key: "group-execution-dialog",
    closeOnDismissRequest: false,
    properties: { dismissOnBackPress: !state.busy, dismissOnClickOutside: !state.busy },
    /** Loads the actual retained state once when this modal mounts. */
    onLoad: () => control.load(),
    /** Closes this modal without changing the underlying submission. */
    onDismissRequest: () => control.close()
  }, ctx.UI.Column({ fillMaxWidth: true, paddingHorizontal: 20, paddingVertical: 16 }, [
    ctx.UI.Text({ text: "\u7FA4\u7EC4\u6267\u884C", style: "titleMedium" }),
    ctx.UI.Text({ key: "group-execution-status", text: !state.loaded ? "\u6B63\u5728\u8BFB\u53D6\u6267\u884C\u72B6\u6001\u2026" : value === null ? "\u5F53\u524D\u4F1A\u8BDD\u6CA1\u6709\u5DF2\u63D0\u4EA4\u7684\u7FA4\u7EC4\u4EFB\u52A1" : labels[value.status], paddingVertical: 12 }),
    ...value === null ? [] : [
      ctx.UI.Text({ text: "\u63D0\u4EA4\uFF1A" + value.submissionId }),
      ...value.outcome === null ? [] : [ctx.UI.Text({ text: outcomes[value.outcome.status] + " \xB7 " + value.outcome.cursor + "/" + value.outcome.plannedTurns })],
      ...value.error === null ? [] : [ctx.UI.Text({ text: value.error, color: ctx.MaterialTheme.colorScheme.error })]
    ],
    ...state.error === "" ? [] : [ctx.UI.Text({ key: "group-execution-error", text: state.error, color: ctx.MaterialTheme.colorScheme.error })],
    ...state.busy ? [ctx.UI.CircularProgressIndicator({ width: 20, height: 20 })] : [],
    ctx.UI.Row({ fillMaxWidth: true }, [
      ctx.UI.TextButton({
        key: "group-execution-refresh",
        enabled: !state.busy && !state.finished,
        /** Requests a new status snapshot only for this explicit refresh click. */
        onClick: () => control.refresh()
      }, ctx.UI.Text({ text: "\u5237\u65B0" })),
      ctx.UI.TextButton({
        key: "group-execution-cancel",
        enabled: enabled && value !== null && value.status === "running",
        /** Cancels only the exact running submission currently displayed. */
        onClick: () => control.act("cancel")
      }, ctx.UI.Text({ text: "\u53D6\u6D88\u6267\u884C" })),
      ctx.UI.TextButton({
        key: "group-execution-resume",
        enabled: enabled && value !== null && value.status === "settled" && value.outcome !== null && value.outcome.status === "cancelled",
        /** Resumes only the exact cancelled submission without resending its user input. */
        onClick: () => control.act("resume")
      }, ctx.UI.Text({ text: "\u7EE7\u7EED\u6267\u884C" }))
    ]),
    ctx.UI.IconButton({
      key: "group-execution-close",
      enabled: !state.busy && !state.finished,
      /** Dismisses this presentation and leaves execution unchanged. */
      onClick: () => control.close()
    }, ctx.UI.Icon({ name: "Close", contentDescription: "\u5173\u95ED" }))
  ]));
}

// src/ui/group-execution/index.ui.ts
function Screen(ctx) {
  return renderGroupExecutionScreen(ctx);
}
