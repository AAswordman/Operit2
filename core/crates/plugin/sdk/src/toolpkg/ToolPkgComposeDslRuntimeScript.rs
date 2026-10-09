/// Wraps a Compose DSL screen script with render, rerender, and action entry points.
#[allow(non_snake_case)]
pub fn buildComposeDslRuntimeWrappedScript(script: &str) -> String {
    format!(
        r#"
        {script}

        (function() {{
            function __operit_is_promise(__value) {{
                return !!(__value && typeof __value.then === 'function');
            }}

            /// Keeps navigation queued during intermediate renders until the action completes.
            function __operit_wrap_compose_response(__bundle, __tree, __actionResult, __includeNavigation = true) {{
                var __response = {{
                    state: __bundle.state,
                    memo: __bundle.memo,
                    navigationCommands: __includeNavigation ? __bundle.takeNavigationCommands() : []
                }};
                if (typeof __tree !== 'undefined') {{
                    var __update = __bundle.composition.commit(__tree);
                    if (__bundle.retainedDelivery) {{
                        __response.update = __update;
                    }} else {{
                        __response.tree = __tree;
                    }}
                }}
                if (typeof __actionResult !== 'undefined') {{
                    __response.actionResult = __actionResult;
                }}
                return __response;
            }}

            /// Builds a response while preserving the caller's navigation delivery policy.
            function __operit_build_compose_response(__bundle, __entry, __actionResult, __includeNavigation = true) {{
                var __tree = OperitComposeReactive.render(__bundle.ctx, __entry);
                if (__operit_is_promise(__tree)) {{
                    return __tree.then(function(__resolvedTree) {{
                        return __operit_wrap_compose_response(
                            __bundle,
                            __resolvedTree,
                            __actionResult,
                            __includeNavigation
                        );
                    }});
                }}
                return __operit_wrap_compose_response(__bundle, __tree, __actionResult, __includeNavigation);
            }}

            function __operitResolveComposeEntry() {{
                try {{
                    if (typeof module !== 'undefined' && module && module.exports) {{
                        if (typeof module.exports.default === 'function') {{
                            return module.exports.default;
                        }}
                        if (typeof module.exports.Screen === 'function') {{
                            return module.exports.Screen;
                        }}
                    }}
                    if (typeof exports !== 'undefined' && exports) {{
                        if (typeof exports.default === 'function') {{
                            return exports.default;
                        }}
                        if (typeof exports.Screen === 'function') {{
                            return exports.Screen;
                        }}
                    }}
                    if (typeof window !== 'undefined') {{
                        if (typeof window.default === 'function') {{
                            return window.default;
                        }}
                        if (typeof window.Screen === 'function') {{
                            return window.Screen;
                        }}
                    }}
                }} catch (e) {{
                    console.error('resolve compose entry failed:', e);
                }}
                return null;
            }}

            function __operit_render_compose_dsl(__runtimeOptions) {{
                if (typeof OperitComposeDslRuntime === 'undefined') {{
                    throw new Error('OperitComposeDslRuntime bridge is not initialized');
                }}
                var __root = typeof globalThis !== 'undefined'
                    ? globalThis
                    : (typeof window !== 'undefined' ? window : this);
                var __activeCallRuntime =
                    typeof __root.__operit_call_runtime_ref === 'object' && __root.__operit_call_runtime_ref
                        ? __root.__operit_call_runtime_ref
                        : null;
                var __options = __runtimeOptions && typeof __runtimeOptions === 'object'
                    ? Object.assign({{}}, __runtimeOptions)
                    : {{}};
                if (__activeCallRuntime) {{
                    __options.__operit_call_runtime = __activeCallRuntime;
                }}
                var __bundle = OperitComposeDslRuntime.createContext(__options);
                var __entry = __operitResolveComposeEntry();
                if (typeof __entry !== 'function') {{
                    throw new Error(
                        'compose_dsl entry function not found, expected default export or Screen function'
                    );
                }}
                if (__activeCallRuntime && typeof __bundle.setCallRuntime === 'function') {{
                    __bundle.setCallRuntime(__activeCallRuntime);
                }}
                __root.__operit_compose_bundle = __bundle;
                __root.__operit_compose_entry = __entry;
                return __operit_build_compose_response(__bundle, __entry);
            }}

            function __operit_rerender_compose_dsl(__runtimeOptions) {{
                var __root = typeof globalThis !== 'undefined'
                    ? globalThis
                    : (typeof window !== 'undefined' ? window : this);
                var __bundle = __root.__operit_compose_bundle;
                var __entry = __root.__operit_compose_entry;
                if (!__bundle || typeof __entry !== 'function') {{
                    throw new Error('compose_dsl runtime is not initialized, render first');
                }}
                var __activeCallRuntime =
                    typeof __root.__operit_call_runtime_ref === 'object' && __root.__operit_call_runtime_ref
                        ? __root.__operit_call_runtime_ref
                        : null;
                var __options = __runtimeOptions && typeof __runtimeOptions === 'object'
                    ? Object.assign({{}}, __runtimeOptions)
                    : {{}};
                if (__activeCallRuntime) {{
                    __options.__operit_call_runtime = __activeCallRuntime;
                }}
                if (typeof __bundle.updateRuntimeOptions === 'function') {{
                    __bundle.updateRuntimeOptions(__options);
                }}
                if (__activeCallRuntime && typeof __bundle.setCallRuntime === 'function') {{
                    __bundle.setCallRuntime(__activeCallRuntime);
                }}
                return __operit_build_compose_response(__bundle, __entry);
            }}

            function __operit_dispatch_compose_dsl_action(__actionRequest) {{
                var __root = typeof globalThis !== 'undefined'
                    ? globalThis
                    : (typeof window !== 'undefined' ? window : this);
                var __bundle = __root.__operit_compose_bundle;
                var __entry = __root.__operit_compose_entry;
                if (!__bundle || typeof __entry !== 'function') {{
                    throw new Error('compose_dsl runtime is not initialized, render first');
                }}
                if (typeof __bundle.invokeAction !== 'function') {{
                    throw new Error('compose_dsl runtime action bridge is not available');
                }}
                var __activeCallRuntime =
                    typeof __root.__operit_call_runtime_ref === 'object' && __root.__operit_call_runtime_ref
                        ? __root.__operit_call_runtime_ref
                        : null;

                var __request =
                    __actionRequest && typeof __actionRequest === 'object'
                        ? __actionRequest
                        : {{}};
                var __runtimeOptions = Object.assign({{}}, __request);
                if (__activeCallRuntime) {{
                    __runtimeOptions.__operit_call_runtime = __activeCallRuntime;
                }}
                if (typeof __bundle.updateRuntimeOptions === 'function') {{
                    __bundle.updateRuntimeOptions(__runtimeOptions);
                }}
                if (__activeCallRuntime && typeof __bundle.setCallRuntime === 'function') {{
                    __bundle.setCallRuntime(__activeCallRuntime);
                }}
                var __actionId = String(
                    __request.__action_id || __request.actionId || ''
                ).trim();
                if (!__actionId) {{
                    throw new Error('compose action id is required');
                }}

                var __payload =
                    Object.prototype.hasOwnProperty.call(__request, '__action_payload')
                        ? __request.__action_payload
                        : __request.payload;
                var __noRender =
                    __payload &&
                    typeof __payload === 'object' &&
                    (__payload.__no_render === true ||
                        __payload.__noRender === true ||
                        __payload.__local === true);

                /// Serializes after the render callback stack unwinds.
                function __operit_send_intermediate_result(__value) {{
                    if (
                        !__composeActionCallRuntime ||
                        typeof __composeActionCallRuntime.sendIntermediateResult !== 'function'
                    ) {{
                        return;
                    }}
                    return Promise.resolve().then(function() {{
                        __composeActionCallRuntime.sendIntermediateResult(__value);
                    }});
                }}

                var __actionSettled = false;
                var __intermediateRenderQueued = false;
                var __intermediateRenderInFlight = false;
                var __unsubscribeStateChange = null;
                var __composeActionCallRuntime =
                    typeof __root.__operit_call_runtime_ref === 'object' &&
                    __root.__operit_call_runtime_ref
                        ? __root.__operit_call_runtime_ref
                        : null;
                var __composeActionCallId = String(
                    typeof __operitCurrentCallId === 'string' ? __operitCurrentCallId : ''
                );

                function __operit_finalize_detached_state_listener() {{
                    if (typeof __unsubscribeStateChange === 'function') {{
                        try {{
                            __unsubscribeStateChange();
                        }} catch (__unsubscribeError) {{
                        }}
                        __unsubscribeStateChange = null;
                    }}
                }}

                function __operit_can_render_intermediate() {{
                    if (!__actionSettled) {{
                        return true;
                    }}
                    var __callId = __composeActionCallId;
                    var __callState =
                        typeof __operitGetCallState === 'function'
                            ? __operitGetCallState(__callId)
                            : null;
                    return !!(__callState && __callState.detached);
                }}

                function __operit_finalize_action() {{
                    __actionSettled = true;
                    var __callId = String(
                        typeof __operitCurrentCallId === 'string' ? __operitCurrentCallId : ''
                    );
                    var __callState =
                        typeof __operitGetCallState === 'function'
                            ? __operitGetCallState(__callId)
                            : null;
                    if (__callState && Number(__callState.pendingReferences || 0) > 0) {{
                        __callState.detachedCleanup = __operit_finalize_detached_state_listener;
                    }} else {{
                        __operit_finalize_detached_state_listener();
                    }}
                }}

                function __operit_render_and_send_intermediate() {{
                    if (!__operit_can_render_intermediate()) {{
                        return null;
                    }}
                    try {{
                        var __intermediateResponse = __operit_build_compose_response(__bundle, __entry, undefined, false);
                        if (__operit_is_promise(__intermediateResponse)) {{
                            return __intermediateResponse.then(function(__resolvedIntermediate) {{
                                if (__operit_can_render_intermediate()) {{
                                    return __operit_send_intermediate_result(__resolvedIntermediate);
                                }}
                            }});
                        }}
                        return __operit_send_intermediate_result(__intermediateResponse);
                    }} catch (__intermediateError) {{
                        try {{
                            console.warn('compose intermediate render failed:', __intermediateError);
                        }} catch (__ignore) {{
                        }}
                    }}
                    return null;
                }}

                function __operit_process_intermediate_queue() {{
                    if (!__operit_can_render_intermediate() || __intermediateRenderInFlight || !__intermediateRenderQueued) {{
                        return;
                    }}
                    __intermediateRenderQueued = false;
                    __intermediateRenderInFlight = true;
                    var __renderResult = __operit_render_and_send_intermediate();
                    if (__operit_is_promise(__renderResult)) {{
                        __renderResult.then(
                            function() {{}},
                            function() {{}}
                        ).then(function() {{
                            __intermediateRenderInFlight = false;
                            if (__intermediateRenderQueued && __operit_can_render_intermediate()) {{
                                __operit_process_intermediate_queue();
                            }}
                        }});
                        return;
                    }}
                    __intermediateRenderInFlight = false;
                    if (__intermediateRenderQueued && __operit_can_render_intermediate()) {{
                        __operit_process_intermediate_queue();
                    }}
                }}

                function __operit_schedule_intermediate_render() {{
                    if (!__operit_can_render_intermediate()) {{
                        return;
                    }}
                    __intermediateRenderQueued = true;
                    Promise.resolve().then(function() {{
                        __operit_process_intermediate_queue();
                    }});
                }}

                function __operit_flush_state_changes() {{
                    if (!__bundle || typeof __bundle.flushStateChanges !== 'function') {{
                        return Promise.resolve();
                    }}
                    try {{
                        var __flushResult = __bundle.flushStateChanges();
                        if (__operit_is_promise(__flushResult)) {{
                            return __flushResult;
                        }}
                    }} catch (__flushError) {{
                        try {{
                            console.warn('compose state flush failed:', __flushError);
                        }} catch (__ignore) {{
                        }}
                    }}
                    return Promise.resolve();
                }}

                function __operit_build_final_response(__actionResult) {{
                    return __operit_flush_state_changes().then(function() {{
                        if (__noRender) {{
                            return __operit_wrap_compose_response(
                                __bundle,
                                undefined,
                                __actionResult
                            );
                        }}
                        return __operit_build_compose_response(
                            __bundle,
                            __entry,
                            __actionResult
                        );
                    }});
                }}

                if (typeof __bundle.subscribeStateChange === 'function') {{
                    if (!__noRender) {{
                        __unsubscribeStateChange = __bundle.subscribeStateChange(function() {{
                            __operit_schedule_intermediate_render();
                        }});
                    }}
                }}

                var __maybePromise;
                try {{
                    __maybePromise = __bundle.invokeAction(__actionId, __payload);
                }} catch (__actionError) {{
                    __operit_finalize_action();
                    throw __actionError;
                }}
                if (__maybePromise && typeof __maybePromise.then === 'function') {{
                    if (!__noRender) {{
                        __operit_schedule_intermediate_render();
                    }}
                    return __maybePromise.then(function(__resolvedActionResult) {{
                        return __operit_build_final_response(__resolvedActionResult).then(function(__finalResponse) {{
                            __operit_finalize_action();
                            return __finalResponse;
                        }});
                    }}, function(__actionError) {{
                        __operit_finalize_action();
                        throw __actionError;
                    }});
                }}
                return __operit_build_final_response(__maybePromise).then(function(__finalResponse) {{
                    __operit_finalize_action();
                    return __finalResponse;
                }});
            }}

            if (typeof exports !== 'undefined' && exports) {{
                exports.__operit_render_compose_dsl = __operit_render_compose_dsl;
                exports.__operit_rerender_compose_dsl = __operit_rerender_compose_dsl;
                exports.__operit_dispatch_compose_dsl_action =
                    __operit_dispatch_compose_dsl_action;
            }}
            if (typeof module !== 'undefined' && module && module.exports) {{
                module.exports.__operit_render_compose_dsl = __operit_render_compose_dsl;
                module.exports.__operit_rerender_compose_dsl = __operit_rerender_compose_dsl;
                module.exports.__operit_dispatch_compose_dsl_action =
                    __operit_dispatch_compose_dsl_action;
            }}
            var __root = typeof globalThis !== 'undefined'
                ? globalThis
                : (typeof window !== 'undefined' ? window : this);
            __root.__operit_render_compose_dsl = __operit_render_compose_dsl;
            __root.__operit_rerender_compose_dsl = __operit_rerender_compose_dsl;
            __root.__operit_dispatch_compose_dsl_action =
                __operit_dispatch_compose_dsl_action;
        }})();
    "#
    )
}
