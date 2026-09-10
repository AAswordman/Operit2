/// Builds the JavaScript runtime that gates ToolPkg APIs by manifest API version.
#[allow(non_snake_case)]
pub fn buildToolPkgApiRuntimeScript() -> String {
    r#"
    (function() {
        var root = globalThis;
        var expose = root.__operitExpose;
        if (typeof expose !== 'function') {
            throw new Error('__operitExpose is unavailable');
        }

        function currentCallId() {
            var callId = root.__operitCurrentCallId;
            if (typeof callId !== 'string' || !callId.trim()) {
                throw new Error('ToolPkg API requires an active execution call id.');
            }
            return callId.trim();
        }

        function currentCallState() {
            var callId = currentCallId();
            if (typeof root.__operitGetCallState !== 'function') {
                throw new Error('ToolPkg API execution state reader is unavailable.');
            }
            var state = root.__operitGetCallState(callId);
            if (!state || typeof state !== 'object') {
                throw new Error('ToolPkg API execution state is unavailable for call ' + callId + '.');
            }
            return state;
        }

        function currentApiVersion() {
            var state = currentCallState();
            if (!state.params || typeof state.params !== 'object' || Array.isArray(state.params)) {
                throw new Error('ToolPkg API execution context is missing parameters.');
            }
            var version = state.params.__operit_toolpkg_api_version;
            if (typeof version !== 'string' || !version.trim()) {
                throw new Error('ToolPkg API execution context is missing manifest.api_version.');
            }
            return version.trim();
        }

        function parseVersion(value) {
            var text = typeof value === 'string' ? value.trim() : '';
            var match = /^([0-9]+)\.([0-9]+)\.([0-9]+)$/.exec(text);
            if (!match) {
                return null;
            }
            return {
                text: text,
                parts: [Number(match[1]), Number(match[2]), Number(match[3])]
            };
        }

        function requireVersion(value, label) {
            var parsed = parseVersion(value);
            if (!parsed) {
                throw new Error(label + ' must use major.minor.patch format');
            }
            return parsed;
        }

        function compareVersions(left, right) {
            for (var index = 0; index < 3; index += 1) {
                if (left.parts[index] > right.parts[index]) return 1;
                if (left.parts[index] < right.parts[index]) return -1;
            }
            return 0;
        }

        // Parses an optional exclusive upper bound for a method implementation.
        function optionalVersion(value, label) {
            if (value === undefined || value === null) {
                return null;
            }
            return requireVersion(value, label);
        }

        function unsupported(apiName, requiredVersion, currentVersion) {
            throw new Error(
                apiName + ' requires ToolPkg API ' + requiredVersion +
                ', but manifest.api_version is ' + currentVersion + '.'
            );
        }

        function requireCurrentVersion(apiName) {
            var currentText = currentApiVersion();
            var current = parseVersion(currentText);
            if (!current) {
                throw new Error(apiName + ' requires a valid manifest.api_version.');
            }
            return current;
        }

        function cleanNamespace(value) {
            if (typeof value !== 'string' || value.trim() !== value || !value) {
                throw new Error('ToolPkg API namespace must be a non-empty trimmed string.');
            }
            var segments = value.split('.');
            for (var index = 0; index < segments.length; index += 1) {
                if (!segments[index]) {
                    throw new Error('ToolPkg API namespace must not contain empty path segments.');
                }
            }
            return value;
        }

        // Formats a normalized method implementation range for diagnostics.
        function variantRangeText(variant) {
            return variant.until
                ? '>= ' + variant.since.text + ' and < ' + variant.until.text
                : '>= ' + variant.since.text;
        }

        // Checks whether a parsed ToolPkg API version lands in one implementation range.
        function variantMatches(variant, current) {
            return compareVersions(current, variant.since) >= 0 &&
                (!variant.until || compareVersions(current, variant.until) < 0);
        }

        function normalizeVariants(apiName, variants) {
            if (!Array.isArray(variants) || variants.length === 0) {
                throw new Error(apiName + ' must declare at least one API version variant.');
            }
            var normalized = [];
            var seenRange = {};
            var seenSince = {};
            for (var index = 0; index < variants.length; index += 1) {
                var variant = variants[index];
                if (!variant || typeof variant !== 'object' || Array.isArray(variant)) {
                    throw new Error(apiName + ' variant must be an object.');
                }
                if (typeof variant.invoke !== 'function') {
                    throw new Error(apiName + ' variant must declare an implementation function.');
                }
                var since = requireVersion(variant.since, apiName + ' variant since');
                var until = optionalVersion(variant.until, apiName + ' variant until');
                if (until && compareVersions(since, until) >= 0) {
                    throw new Error(
                        apiName + ' variant until must be greater than since: ' +
                        since.text + ' .. ' + until.text + '.'
                    );
                }
                if (seenSince[since.text]) {
                    throw new Error(apiName + ' declares duplicate variant since ' + since.text + '.');
                }
                seenSince[since.text] = true;
                var rangeKey = since.text + '..' + (until ? until.text : '');
                if (seenRange[rangeKey]) {
                    throw new Error(apiName + ' declares duplicate variant range ' + rangeKey + '.');
                }
                seenRange[rangeKey] = true;
                normalized.push({
                    since: since,
                    until: until,
                    invoke: variant.invoke
                });
            }
            normalized.sort(function(left, right) {
                var order = compareVersions(left.since, right.since);
                if (order !== 0) {
                    return order;
                }
                if (!left.until && right.until) {
                    return 1;
                }
                if (left.until && !right.until) {
                    return -1;
                }
                if (!left.until && !right.until) {
                    return 0;
                }
                return compareVersions(left.until, right.until);
            });
            for (var rangeIndex = 0; rangeIndex + 1 < normalized.length; rangeIndex += 1) {
                var current = normalized[rangeIndex];
                var next = normalized[rangeIndex + 1];
                if (!current.until) {
                    current.until = next.since;
                }
                if (compareVersions(current.until, next.since) > 0) {
                    throw new Error(
                        apiName + ' declares overlapping variant ranges: ' +
                        variantRangeText(current) + ' and ' + variantRangeText(next) + '.'
                    );
                }
            }
            return normalized;
        }

        function selectVariant(apiName, introduced, normalizedVariants) {
            var current = requireCurrentVersion(apiName);
            if (compareVersions(current, introduced) < 0) {
                unsupported(apiName, introduced.text, current.text);
            }
            for (var index = 0; index < normalizedVariants.length; index += 1) {
                var variant = normalizedVariants[index];
                if (variantMatches(variant, current)) {
                    return variant.invoke;
                }
            }
            var ranges = normalizedVariants.map(variantRangeText).join('; ');
            throw new Error(
                apiName + ' has no implementation for ToolPkg API ' + current.text +
                '. Supported ranges: ' + ranges + '.'
            );
        }

        function versionedMethod(apiName, variants) {
            var normalizedName = cleanNamespace(apiName);
            var normalizedVariants = normalizeVariants(normalizedName, variants);
            var introduced = normalizedVariants[0].since;
            return function() {
                var invoke = selectVariant(normalizedName, introduced, normalizedVariants);
                return invoke.apply(this, arguments);
            };
        }

        function method() {
            var variants = [];
            var builder = {
                __operitToolPkgApiMethod: true,
                since: function(apiVersion, invoke) {
                    return builder.range(apiVersion, null, invoke);
                },
                range: function(apiVersion, untilApiVersion, invoke) {
                    variants.push({
                        since: apiVersion,
                        until: untilApiVersion,
                        invoke: invoke
                    });
                    return builder;
                },
                build: function(apiName) {
                    return versionedMethod(apiName, variants);
                }
            };
            return builder;
        }

        function namespace(publicName, members) {
            var normalizedNamespace = cleanNamespace(publicName);
            if (!members || typeof members !== 'object' || Array.isArray(members)) {
                throw new Error(normalizedNamespace + ' members must be an object.');
            }
            var output = {};
            Object.keys(members).forEach(function(memberName) {
                var member = members[memberName];
                output[memberName] =
                    member &&
                    member.__operitToolPkgApiMethod === true &&
                    typeof member.build === 'function'
                        ? member.build(normalizedNamespace + '.' + memberName)
                        : member;
            });
            return output;
        }

        expose('__operitToolPkgApi', {
            currentCallId: currentCallId,
            currentVersion: currentApiVersion,
            namespace: namespace,
            method: method
        });
    })();
    "#
    .to_string()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::buildToolPkgApiRuntimeScript;
    use rquickjs::{Context, Runtime};

    /// Installs the ToolPkg API runtime into a QuickJS test context.
    fn install_runtime(context: &Context) {
        context.with(|context| {
            context
                .eval::<(), _>(
                    r#"
                    globalThis.__operitExpose = function(name, value) {
                        globalThis[name] = value;
                    };
                    globalThis.__operitCurrentCallId = 'test-call';
                    globalThis.__operitGetCallState = function() {
                        return {
                            params: {
                                __operit_toolpkg_api_version: globalThis.__testToolPkgApiVersion
                            }
                        };
                    };
                    "#,
                )
                .expect("ToolPkg API runtime globals should evaluate");
            context
                .eval::<(), _>(buildToolPkgApiRuntimeScript())
                .expect("ToolPkg API runtime should evaluate");
        });
    }

    /// Verifies method implementations can be selected by explicit API version ranges.
    #[test]
    fn selects_explicit_api_version_ranges() {
        let runtime = Runtime::new().expect("QuickJS runtime should start");
        let context = Context::full(&runtime).expect("QuickJS context should start");
        install_runtime(&context);

        context.with(|context| {
            context
                .eval::<(), _>(
                    r#"
                    globalThis.TestApi = __operitToolPkgApi.namespace('Test.Api', {
                        call: __operitToolPkgApi.method()
                            .range('2.0.0', '2.1.0', function() { return 'v200'; })
                            .range('2.1.0', '2.2.0', function() { return 'v210'; })
                            .since('2.3.0', function() { return 'v230'; })
                    });
                    "#,
                )
                .expect("versioned API fixture should evaluate");

            context
                .eval::<(), _>("globalThis.__testToolPkgApiVersion = '2.0.5';")
                .expect("test API version should set");
            assert_eq!(
                context
                    .eval::<String, _>("TestApi.call();")
                    .expect("2.0 range should invoke"),
                "v200"
            );

            context
                .eval::<(), _>("globalThis.__testToolPkgApiVersion = '2.1.0';")
                .expect("test API version should set");
            assert_eq!(
                context
                    .eval::<String, _>("TestApi.call();")
                    .expect("2.1 range should invoke"),
                "v210"
            );

            context
                .eval::<(), _>("globalThis.__testToolPkgApiVersion = '2.3.0';")
                .expect("test API version should set");
            assert_eq!(
                context
                    .eval::<String, _>("TestApi.call();")
                    .expect("2.3 range should invoke"),
                "v230"
            );
        });
    }

    /// Verifies version gaps remain unavailable instead of selecting a neighboring implementation.
    #[test]
    fn rejects_api_version_range_gaps() {
        let runtime = Runtime::new().expect("QuickJS runtime should start");
        let context = Context::full(&runtime).expect("QuickJS context should start");
        install_runtime(&context);

        context.with(|context| {
            context
                .eval::<(), _>(
                    r#"
                    globalThis.TestApi = __operitToolPkgApi.namespace('Test.Api', {
                        call: __operitToolPkgApi.method()
                            .range('2.0.0', '2.1.0', function() { return 'v200'; })
                            .since('2.2.0', function() { return 'v220'; })
                    });
                    globalThis.__testToolPkgApiVersion = '2.1.5';
                    "#,
                )
                .expect("version gap fixture should evaluate");

            assert!(context.eval::<String, _>("TestApi.call();").is_err());
        });
    }

    /// Verifies overlapping version ranges are rejected when a method is created.
    #[test]
    fn rejects_overlapping_api_version_ranges() {
        let runtime = Runtime::new().expect("QuickJS runtime should start");
        let context = Context::full(&runtime).expect("QuickJS context should start");
        install_runtime(&context);

        context.with(|context| {
            assert!(context
                .eval::<(), _>(
                    r#"
                        __operitToolPkgApi.namespace('Test.Api', {
                            call: __operitToolPkgApi.method()
                                .range('2.0.0', '2.2.0', function() { return 'v200'; })
                                .range('2.1.0', '2.3.0', function() { return 'v210'; })
                        });
                        "#,
                )
                .is_err());
        });
    }
}
