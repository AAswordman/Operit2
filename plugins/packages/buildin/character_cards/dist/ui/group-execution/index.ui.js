// @operit-compose-artifact 4
"use strict";
var __operitUiPrograms_=[{"id":"com.operit.character_cards:dist/ui/group-execution/index.ui.js:10bb96f:7693","root":0,"nodes":[{"type":"Text","props":[2,["text","style"],[[0,"群组执行"],[0,"titleMedium"]]],"children":[0,null],"explicitChildren":false}]},{"id":"com.operit.character_cards:dist/ui/group-execution/index.ui.js:10bb96f:7770","root":0,"nodes":[{"type":"Text","props":[2,["key","text","paddingVertical"],[[0,"group-execution-status"],[1,0],[0,12]]],"children":[0,null],"explicitChildren":false}]},{"id":"com.operit.character_cards:dist/ui/group-execution/index.ui.js:10bb96f:8085","root":0,"nodes":[{"type":"Text","props":[2,["text"],[[1,0]]],"children":[0,null],"explicitChildren":false}]},{"id":"com.operit.character_cards:dist/ui/group-execution/index.ui.js:10bb96f:8191","root":0,"nodes":[{"type":"Text","props":[2,["text"],[[1,0]]],"children":[0,null],"explicitChildren":false}]},{"id":"com.operit.character_cards:dist/ui/group-execution/index.ui.js:10bb96f:8354","root":0,"nodes":[{"type":"Text","props":[2,["text","color"],[[1,0],[1,1]]],"children":[0,null],"explicitChildren":false}]},{"id":"com.operit.character_cards:dist/ui/group-execution/index.ui.js:10bb96f:8475","root":0,"nodes":[{"type":"Text","props":[2,["key","text","color"],[[0,"group-execution-error"],[1,0],[1,1]]],"children":[0,null],"explicitChildren":false}]},{"id":"com.operit.character_cards:dist/ui/group-execution/index.ui.js:10bb96f:8607","root":0,"nodes":[{"type":"CircularProgressIndicator","props":[2,["width","height"],[[0,20],[0,20]]],"children":[0,null],"explicitChildren":false}]},{"id":"com.operit.character_cards:dist/ui/group-execution/index.ui.js:10bb96f:8678","root":0,"nodes":[{"type":"Row","props":[2,["fillMaxWidth"],[[0,true]]],"children":[3,[[4,1],[4,3],[4,5]]],"explicitChildren":true},{"type":"TextButton","props":[2,["key","enabled","onClick"],[[0,"group-execution-refresh"],[1,0],[1,1]]],"children":[4,2],"explicitChildren":true},{"type":"Text","props":[2,["text"],[[0,"刷新"]]],"children":[0,null],"explicitChildren":false},{"type":"TextButton","props":[2,["key","enabled","onClick"],[[0,"group-execution-cancel"],[1,2],[1,3]]],"children":[4,4],"explicitChildren":true},{"type":"Text","props":[2,["text"],[[0,"取消执行"]]],"children":[0,null],"explicitChildren":false},{"type":"TextButton","props":[2,["key","enabled","onClick"],[[0,"group-execution-resume"],[1,4],[1,5]]],"children":[4,6],"explicitChildren":true},{"type":"Text","props":[2,["text"],[[0,"继续执行"]]],"children":[0,null],"explicitChildren":false}]},{"id":"com.operit.character_cards:dist/ui/group-execution/index.ui.js:10bb96f:9742","root":0,"nodes":[{"type":"IconButton","props":[2,["key","enabled","onClick"],[[0,"group-execution-close"],[1,0],[1,1]]],"children":[4,1],"explicitChildren":true},{"type":"Icon","props":[2,["name","contentDescription"],[[0,"Close"],[0,"关闭"]]],"children":[0,null],"explicitChildren":false}]},{"id":"com.operit.character_cards:dist/ui/group-execution/index.ui.js:10bb96f:7203","root":0,"nodes":[{"type":"Dialog","props":[2,["key","closeOnDismissRequest","properties","onLoad","onDismissRequest"],[[0,"group-execution-dialog"],[0,false],[2,["dismissOnBackPress","dismissOnClickOutside"],[[1,0],[1,1]]],[1,2],[1,3]]],"children":[4,1],"explicitChildren":true},{"type":"Column","props":[2,["fillMaxWidth","paddingHorizontal","paddingVertical"],[[0,true],[0,20],[0,16]]],"children":[1,4],"explicitChildren":true}]}];

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
  return OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/group-execution/index.ui.js:7203",__operitUiPrograms_[9],[!state.busy,!state.busy,() => control.load(),() => control.close(),[
    OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/group-execution/index.ui.js:7693",__operitUiPrograms_[0],[]),
    OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/group-execution/index.ui.js:7770",__operitUiPrograms_[1],[!state.loaded ? "\u6B63\u5728\u8BFB\u53D6\u6267\u884C\u72B6\u6001\u2026" : value === null ? "\u5F53\u524D\u4F1A\u8BDD\u6CA1\u6709\u5DF2\u63D0\u4EA4\u7684\u7FA4\u7EC4\u4EFB\u52A1" : labels[value.status]]),
    ...value === null ? [] : [
      OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/group-execution/index.ui.js:8085",__operitUiPrograms_[2],["\u63D0\u4EA4\uFF1A" + value.submissionId]),
      ...value.outcome === null ? [] : [OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/group-execution/index.ui.js:8191",__operitUiPrograms_[3],[outcomes[value.outcome.status] + " \xB7 " + value.outcome.cursor + "/" + value.outcome.plannedTurns])],
      ...value.error === null ? [] : [OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/group-execution/index.ui.js:8354",__operitUiPrograms_[4],[value.error,ctx.MaterialTheme.colorScheme.error])]
    ],
    ...state.error === "" ? [] : [OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/group-execution/index.ui.js:8475",__operitUiPrograms_[5],[state.error,ctx.MaterialTheme.colorScheme.error])],
    ...state.busy ? [OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/group-execution/index.ui.js:8607",__operitUiPrograms_[6],[])] : [],
    OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/group-execution/index.ui.js:8678",__operitUiPrograms_[7],[!state.busy && !state.finished,() => control.refresh(),enabled && value !== null && value.status === "running",() => control.act("cancel"),enabled && value !== null && value.status === "settled" && value.outcome !== null && value.outcome.status === "cancelled",() => control.act("resume")]),
    OperitComposeComposition.evaluate(ctx,"com.operit.character_cards:dist/ui/group-execution/index.ui.js:9742",__operitUiPrograms_[8],[!state.busy && !state.finished,() => control.close()])
  ]]);
}

// src/ui/group-execution/index.ui.ts
function Screen(ctx) {
  return renderGroupExecutionScreen(ctx);
}
