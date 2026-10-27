if (typeof module !== 'undefined' && module && module.exports) {
  module.exports.__operit_render_compose_dsl = renderComposeDsl;
  module.exports.__operit_dispatch_compose_dsl_action = dispatchComposeDslAction;
}
var root = typeof globalThis !== 'undefined' ? globalThis : typeof window !== 'undefined' ? window : this;
root.__operit_render_compose_dsl = renderComposeDsl;
root.__operit_dispatch_compose_dsl_action = dispatchComposeDslAction;
