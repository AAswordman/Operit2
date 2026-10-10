/// Compiles existing UI calls into internal generation scopes without changing plugin source APIs.
var OperitComposeCompiler = (function() {
    var childNodes = new WeakMap();
    /// Visits direct ESTree child nodes without traversing locations or primitive properties.
    function children(node) {
        if (childNodes.has(node)) return childNodes.get(node);
        var result = [];
        Object.keys(node).forEach(function(key) {
            var value = node[key];
            if (Array.isArray(value)) value.forEach(function(item) { if (item && typeof item.type === 'string') result.push(item); });
            else if (value && typeof value.type === 'string') result.push(value);
        });
        result.sort(function(a, b) { return a.start - b.start; });
        childNodes.set(node, result);
        return result;
    }

    /// Recognizes the existing context.UI.Type call using its parsed member structure.
    function uiCall(node) {
        if (node.type !== 'CallExpression' || node.optional) return null;
        var member = node.callee;
        if (member.type !== 'MemberExpression' || member.computed || member.optional) return null;
        var registry = member.object;
        if (registry.type !== 'MemberExpression' || registry.computed || registry.optional || registry.property.name !== 'UI') return null;
        return registry.object.type === 'Identifier' ? registry.object : null;
    }

    /// Classifies side-effect-free value expressions and records their lexical value dependencies.
    function dependencies(node, names) {
        switch (node.type) {
            case 'Literal': return true;
            case 'Identifier': names.add(node.name); return true;
            case 'ArrayExpression': return node.elements.every(function(item) { return item === null || dependencies(item, names); });
            case 'ObjectExpression': return node.properties.every(function(property) {
                return property.type === 'Property' && property.kind === 'init' && !property.method &&
                    (!property.computed || dependencies(property.key, names)) && dependencies(property.value, names);
            });
            case 'UnaryExpression': return node.operator !== 'delete' && dependencies(node.argument, names);
            case 'BinaryExpression':
            case 'LogicalExpression': return dependencies(node.left, names) && dependencies(node.right, names);
            case 'ConditionalExpression': return dependencies(node.test, names) && dependencies(node.consequent, names) && dependencies(node.alternate, names);
            case 'TemplateLiteral': return node.expressions.every(function(item) { return dependencies(item, names); });
            default: return false;
        }
    }

    /// Transforms only parsed UI construction calls and leaves user control flow and side effects intact.
    function compile(source, identity) {
        var ast = __operitAcorn.parse(source, { ecmaVersion: 2022, sourceType: 'script', allowReturnOutsideFunction: true });
        /// Recognizes a statically keyed state binding whose lexical value can be refreshed independently.
        function stateBinding(declaration, context) {
            var call = declaration.init;
            return declaration.id.type === 'ArrayPattern' && declaration.id.elements.length <= 2 &&
                declaration.id.elements.every(function(item) { return item === null || item.type === 'Identifier'; }) &&
                call && call.type === 'CallExpression' && call.callee.type === 'MemberExpression' && !call.callee.computed &&
                call.callee.object.type === 'Identifier' && call.callee.object.name === context && call.callee.property.name === 'useState' &&
                call.arguments[0] && call.arguments[0].type === 'Literal' && typeof call.arguments[0].value === 'string';
        }

        /// Keeps observable calls, mutations and control-flow effects inside their original execution unit.
        function separable(node) {
            if (!node) return true;
            if (node.type === 'FunctionExpression' || node.type === 'ArrowFunctionExpression') return true;
            if (node.type === 'AssignmentExpression' || node.type === 'UpdateExpression' || node.type === 'AwaitExpression' ||
                node.type === 'YieldExpression' || node.type === 'NewExpression' || node.type === 'TaggedTemplateExpression') return false;
            if (node.type === 'UnaryExpression' && node.operator === 'delete') return false;
            if (node.type === 'CallExpression') {
                if (!uiCall(node) && !(node.callee.type === 'MemberExpression' && !node.callee.computed &&
                    node.callee.property.name === 'map')) return false;
            }
            return children(node).every(separable);
        }

        /// Rejects captured variable assignments that lack an observable state-write boundary.
        function capturedWrites(node, names) {
            if (!node) return false;
            if (node.type === 'AssignmentExpression' || node.type === 'UpdateExpression') {
                var target = node.type === 'AssignmentExpression' ? node.left : node.argument;
                if (target.type === 'Identifier' && names.has(target.name)) return true;
            }
            return children(node).some(function(child) { return capturedWrites(child, names); });
        }

        /// Partitions straight-line synchronous render functions into lexical binding and expression units.
        function renderFrame(fn) {
            if (fn.async || fn.generator || fn.params.length !== 1 || fn.params[0].type !== 'Identifier' || fn.body.type !== 'BlockStatement') return null;
            var context = fn.params[0].name;
            var statements = fn.body.body;
            if (!statements.length || statements[statements.length - 1].type !== 'ReturnStatement') return null;
            var hasState = false;
            var variables = new Set();
            for (var index = 0; index < statements.length - 1; index += 1) {
                var statement = statements[index];
                if (statement.type === 'FunctionDeclaration') continue;
                if (statement.type !== 'VariableDeclaration') return null;
                for (var declaration of statement.declarations) {
                    if (stateBinding(declaration, context)) {
                        hasState = true;
                        if (declaration.id.elements[0]) variables.add(declaration.id.elements[0].name);
                    } else if (declaration.id.type === 'Identifier' && declaration.init && separable(declaration.init)) {
                        if (declaration.init.type !== 'ArrowFunctionExpression' && declaration.init.type !== 'FunctionExpression') variables.add(declaration.id.name);
                    } else return null;
                }
            }
            if (!hasState || !uiCall(statements[statements.length - 1].argument) || !separable(statements[statements.length - 1].argument)) return null;
            if (capturedWrites(fn.body, variables)) return null;
            return { context: context, variables: variables, statements: statements, frame: '__operitFrame_' + fn.start };
        }

        /// Finds lexical reads used by this execution unit, excluding independently registered UI expressions and callbacks.
        function reads(node, plan, output) {
            if (!node || uiCall(node) || node.type === 'FunctionExpression' || node.type === 'ArrowFunctionExpression') return;
            if (node.type === 'Identifier') { if (plan.variables.has(node.name)) output.add(node.name); return; }
            if (node.type === 'MemberExpression') {
                reads(node.object, plan, output);
                if (node.computed) reads(node.property, plan, output);
                return;
            }
            if (node.type === 'Property') {
                if (node.computed) reads(node.key, plan, output);
                reads(node.value, plan, output);
                return;
            }
            children(node).forEach(function(child) { reads(child, plan, output); });
        }

        /// Emits dependency keys bound to the exact captured lexical frame.
        function readKeys(node, plan) {
            var names = new Set();
            reads(node, plan, names);
            return '[' + Array.from(names).map(function(name) { return plan.frame + '.key+' + JSON.stringify(':value:' + name); }).join(',') + ']';
        }

        /// Preserves captured lexical variables while registering their independent refresh functions.
        function emitFrame(fn, plan) {
            var output = source.slice(fn.start, fn.body.start) + '{\nconst ' + plan.frame + '=OperitComposeReactive.frame(' + plan.context + ',' + JSON.stringify(identity + ':' + fn.start) + ');\n';
            for (var index = 0; index < plan.statements.length - 1; index += 1) {
                var statement = plan.statements[index];
                if (statement.type === 'FunctionDeclaration') { output += emit(statement) + '\n'; continue; }
                for (var declaration of statement.declarations) {
                    if (stateBinding(declaration, plan.context)) {
                        var name = declaration.id.elements[0];
                        output += 'let ' + source.slice(declaration.id.start, declaration.id.end) + '=' + source.slice(declaration.init.start, declaration.init.end) + ';\n';
                        if (name) output += 'OperitComposeReactive.state(' + plan.frame + ',' + JSON.stringify(name.name) + ',' + source.slice(declaration.init.arguments[0].start, declaration.init.arguments[0].end) + ',(__value)=>{' + name.name + '=__value;});\n';
                    } else if (declaration.init.type === 'ArrowFunctionExpression' || declaration.init.type === 'FunctionExpression') {
                        output += 'const ' + declaration.id.name + '=' + emit(declaration.init) + ';\n';
                    } else {
                        output += 'let ' + declaration.id.name + ';OperitComposeReactive.derived(' + plan.frame + ',' + JSON.stringify(declaration.id.name) + ',' + readKeys(declaration.init, plan) + ',()=>(' + emit(declaration.init, plan) + '),(__value)=>{' + declaration.id.name + '=__value;});\n';
                    }
                }
            }
            return output + 'return ' + emit(plan.statements[plan.statements.length - 1].argument, plan) + ';\n}';
        }

        /// Materializes a source range only when an enclosing rewrite needs its complete text.
        function emit(node, plan) {
            var rewritten = rewrite(node, plan);
            return rewritten === null ? source.slice(node.start, node.end) : rewritten;
        }

        /// Produces replacements only for AST ranges that contain an actual DSL transformation.
        function rewrite(node, plan) {
            if (node.type === 'FunctionDeclaration' || node.type === 'FunctionExpression' || node.type === 'ArrowFunctionExpression') {
                var partition = renderFrame(node);
                if (partition) return emitFrame(node, partition);
            }
            var receiver = uiCall(node);
            if (receiver) {
                var context = source.slice(receiver.start, receiver.end);
                var site = JSON.stringify(identity + ':' + node.start);
                var callee = source.slice(node.callee.start, node.callee.end);
                var args = '[' + node.arguments.map(function(argument) { return emit(argument, plan); }).join(',') + ']';
                var factory = '(...__operitArgs)=>' + callee + '(...__operitArgs)';
                if (plan) {
                    var names = new Set();
                    node.arguments.forEach(function(argument) { reads(argument, plan, names); });
                    var keys = '[' + Array.from(names).map(function(name) { return plan.frame + '.key+' + JSON.stringify(':value:' + name); }).join(',') + ']';
                    return 'OperitComposeReactive.expression(' + plan.frame + ',' + site + ',' + keys + ',()=>'+ args + ',' + factory + ')';
                }
                var names = new Set();
                var pure = node.arguments.every(function(arg) { return dependencies(arg, names); });
                if (pure) return 'OperitComposeGeneration.leaf(' + context + ',' + site + ',[' + Array.from(names).join(',') + '],()=>'+ args + ',' + factory + ')';
                return 'OperitComposeGeneration.node(' + context + ',' + site + ',' + args + ',' + factory + ')';
            }
            var cursor = node.start;
            var visitedEnd = node.start;
            var output = null;
            children(node).forEach(function(child) {
                if (child.start < visitedEnd) return;
                visitedEnd = child.end;
                var replacement = rewrite(child, plan);
                if (replacement === null) return;
                if (output === null) output = '';
                output += source.slice(cursor, child.start) + replacement;
                cursor = child.end;
            });
            return output === null ? null : output + source.slice(cursor, node.end);
        }
        return emit(ast);
    }
    return { compile: compile };
})();

/// Retains generation scopes separately from transport records and updates captured action closures.
var OperitComposeGeneration = (function() {
    var contexts = new WeakMap();
    var generated = new WeakSet();
    var observers = new WeakMap();
    var actionBinding = false;

    /// Identifies nodes whose construction inputs are owned by the generation registry.
    function owns(node) { return generated.has(node); }

    /// Attaches a retained node owner without retaining detached node graphs globally.
    function watch(node, listener) {
        var listeners = observers.get(node);
        if (!listeners) throw new Error('Compose node must be created by the DSL node factory');
        listeners.add(listener);
        /// Releases one retained occurrence when it leaves the composition.
        return function() { listeners.delete(listener); };
    }

    /// Updates the internal callback reference without treating ownership binding as a plugin mutation.
    function rebind(reference, id) {
        actionBinding = true;
        try { reference.__actionId = id; } finally { actionBinding = false; }
    }

    /// Invalidates a cached transport record when a plugin mutates its public node object.
    function observeNode(node) {
        var seen = new WeakMap();
        var root;
        /// Publishes the exact mutation path without invalidating unrelated retained occurrences.
        function changed(path) {
            if (actionBinding) return;
            observers.get(root).forEach(
                /// Enqueues the affected retained occurrence instead of walking its ancestors.
                function(listener) { listener(path); }
            );
        }
        /// Wraps owned containers while retaining child node identity and public object shape.
        function observe(value, path) {
            if (value === null || typeof value !== 'object') return value;
            if (value !== node && value.__composeNode === true) return value;
            if (seen.has(value)) return seen.get(value);
            var proxy = new Proxy(value, {
                /// Reads nested containers through the same mutation observer.
                get(target, key) {
                    if (path.length === 0 && key === 'type') return Reflect.get(target, key);
                    return observe(Reflect.get(target, key), path.concat(key));
                },
                /// Invalidates record caching before an explicit plugin assignment.
                set(target, key, next) {
                    var previous = Reflect.get(target, key);
                    var changedKey = path.length === 0 && key === 'props' && previous && previous.key !== next.key;
                    if (Object.is(previous, next)) return true;
                    if (changedKey) changed(['props', 'key']);
                    else changed(path.concat(key));
                    return Reflect.set(target, key, next);
                },
                /// Invalidates record caching before a plugin removes a property.
                deleteProperty(target, key) {
                    if (!Reflect.has(target, key)) return true;
                    changed(path.concat(key));
                    return Reflect.deleteProperty(target, key);
                },
                /// Invalidates record caching when a plugin defines a property descriptor.
                defineProperty(target, key, descriptor) {
                    var previous = Reflect.getOwnPropertyDescriptor(target, key);
                    if (previous && previous.configurable === descriptor.configurable && previous.enumerable === descriptor.enumerable &&
                        previous.writable === descriptor.writable && previous.value === descriptor.value && previous.get === descriptor.get && previous.set === descriptor.set) return true;
                    changed(path.concat(key));
                    return Reflect.defineProperty(target, key, descriptor);
                },
            });
            seen.set(value, proxy);
            return proxy;
        }
        root = observe(node, []);
        generated.add(root);
        observers.set(root, new Set());
        return root;
    }

    /// Creates generation state owned by exactly one public DSL context.
    function attach(ctx) { contexts.set(ctx, { scopes: new Map(), counts: new Map(), live: new Set() }); }

    /// Starts one render traversal without discarding retained expression results.
    function begin(ctx) { var state = contexts.get(ctx); state.counts.clear(); state.live.clear(); }

    /// Releases scopes belonging to branches and loop entries that left the current composition.
    function end(ctx) {
        var state = contexts.get(ctx);
        state.scopes.forEach(function(value, key) { if (!state.live.has(key)) state.scopes.delete(key); });
    }

    /// Selects a call-site occurrence while preserving independent loop instances.
    function scope(ctx, site) {
        var state = contexts.get(ctx);
        var index = state.counts.get(site) || 0;
        state.counts.set(site, index + 1);
        var key = site + '/' + index;
        state.live.add(key);
        if (!state.scopes.has(key)) state.scopes.set(key, { callbacks: new Map(), initialized: false });
        return state.scopes.get(key);
    }

    /// Retains immutable argument snapshots and action trampolines with replaceable captured closures.
    function snapshot(value, slot, callbacks, seen) {
        if (typeof value === 'function') {
            if (!callbacks.has(slot)) {
                var cell = { handler: value };
                /// Invokes the latest closure while keeping the public callback identity stable.
                cell.invoke = function() { return cell.handler.apply(this, arguments); };
                callbacks.set(slot, cell);
            }
            var current = callbacks.get(slot);
            current.handler = value;
            return current.invoke;
        }
        if (value === null || typeof value !== 'object') return value;
        if (value.__composeNode === true) return value;
        if (seen.has(value)) throw new TypeError('Compose arguments must not contain cycles');
        seen.add(value);
        var result = Array.isArray(value) ? [] : {};
        Object.keys(value).forEach(function(key) { result[key] = snapshot(value[key], slot + '/' + key.length + ':' + key, callbacks, seen); });
        seen.delete(value);
        return result;
    }

    /// Builds a node only when its normalized construction inputs actually changed.
    function evaluate(entry, args, factory) {
        var values = snapshot(args, '', entry.callbacks, new Set());
        if (!entry.initialized || !OperitComposeRetained.equal(entry.args, values)) {
            var next = factory.apply(undefined, values);
            if (entry.initialized && entry.value.type === next.type && entry.value.props.key === next.props.key) {
                // Preserve the occurrence handle so a changed leaf does not replace every ancestor.
                if (!OperitComposeRetained.equal(entry.value.props, next.props)) entry.value.props = next.props;
                if (!OperitComposeRetained.equal(entry.value.children, next.children)) entry.value.children = next.children;
                if (!OperitComposeRetained.equal(entry.value.slots, next.slots)) {
                    if (Object.prototype.hasOwnProperty.call(next, 'slots')) entry.value.slots = next.slots;
                    else delete entry.value.slots;
                }
            } else entry.value = next;
            entry.args = values;
            entry.initialized = true;
        }
        return entry.value;
    }

    /// Evaluates effectful arguments in source order while retaining unchanged node construction.
    function node(ctx, site, args, factory) { return evaluate(scope(ctx, site), args, factory); }

    /// Skips a pure leaf expression before argument construction when its lexical inputs are unchanged.
    function leaf(ctx, site, deps, argumentsFactory, factory) {
        var entry = scope(ctx, site);
        var primitiveInputs = deps.every(function(value) { return value === null || (typeof value !== 'object' && typeof value !== 'function'); });
        if (primitiveInputs && entry.initialized && OperitComposeRetained.equal(entry.deps, deps)) return entry.value;
        entry.deps = snapshot(deps, 'deps', new Map(), new Set());
        return evaluate(entry, argumentsFactory(), factory);
    }
    return { evaluate: evaluate, observeNode: observeNode, watch: watch, rebind: rebind, owns: owns, attach: attach, begin: begin, end: end, node: node, leaf: leaf };
})();
