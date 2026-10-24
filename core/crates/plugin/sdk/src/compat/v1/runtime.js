/**
 * Installs the existing v1 dispatch family over the conditional runtime script package.
 * @param {Record<string, any>} runtime
 */
function __operitInstallV1Runtime(runtime) {
    var api = runtime.__operitToolPkgApi;
    var canonical = runtime.__operitCanonicalTools;
    var originalToolCall = runtime.toolCall;
    var routes = Object.freeze({
        'super_admin:terminal': 'v1_runtime:terminal',
        'code_runner:run_javascript_node': 'v1_runtime:run_javascript_node',
        'ffmpeg:ffmpeg_execute': 'v1_runtime:ffmpeg_execute',
        'ffmpeg:ffmpeg_info': 'v1_runtime:ffmpeg_info',
    });

    /** Decodes the script-package executor's JSON text without hiding provider failures.
     * @param {unknown} value
     * @returns {any}
     */
    function decoded(value) {
        if (typeof value !== 'string') throw new TypeError('V1 runtime package must return serialized JSON');
        return JSON.parse(value);
    }

    /** Calls one exact runtime tool through the original structured host transport.
     * @param {string} method
     * @param {Record<string, any>} params
     * @returns {Promise<any>}
     */
    function invoke(method, params) {
        return originalToolCall('v1_runtime:' + method, params).then(decoded);
    }

    // Both legacy file calls and shell commands use this existing managed filesystem.
    var cacheImports = new Map();

    /** Captures the current container before any asynchronous host work.
     * @returns {{source: string, destination: string, pattern: RegExp} | null}
     */
    function cacheAddress() {
        var state = runtime.__operitGetCallState(runtime.__operitCurrentCallId);
        var params = state && state.params || {};
        var owner = params.containerPackageName || params.__operit_ui_package_name || params.toolPkgId;
        if (typeof owner !== 'string' || !/^[A-Za-z0-9._-]+$/.test(owner)) return null;
        // Same Java UTF-16 hash used by the existing ToolPkg resource cache.
        var hash = 0;
        for (var i = 0; i < owner.length; i++) hash = (Math.imul(hash, 31) + owner.charCodeAt(i)) | 0;
        var directory = owner + '-' + (hash >>> 0).toString(16);
        var escaped = owner.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
        return {
            source: '/app/data/cache/toolpkg/' + directory,
            destination: '/tmp/operit-v1/toolpkg/' + directory,
            pattern: new RegExp('/data/(?:user/0|data)/com\\.ai\\.assistance\\.operit/files/toolpkg_cache/' +
                escaped + '-(?:\\*|[a-fA-F0-9]+)(?=/|$|[\\s\\x22\\x27;|&<>)])', 'g')
        };
    }

    /** Copies packaged resources through the existing VFS and bounded runtime-file transport.
     * Concurrent first accesses share one import; failures remain retryable.
     * @param {{source: string, destination: string}} address
     */
    async function importCache(address) {
        var pending = cacheImports.get(address.source);
        if (!pending) {
            /** Copies a cache tree without guessing native host paths or creating mounts.
             * @param {string} source
             * @param {string} destination
             * @param {number} depth
             */
            async function copyTree(source, destination, depth) {
                if (depth > 64) throw new Error('V1 package cache exceeds directory depth limit');
                var entries = (await canonical.Files.list(source)).entries;
                await invoke('file', { operation: 'mkdir', args: { path: destination, createParents: true } });
                for (var entry of entries) {
                    if (!entry.name || entry.name === '.' || entry.name === '..' || /[/\\\0]/.test(entry.name)) {
                        throw new Error('Invalid V1 package cache entry');
                    }
                    // Commit the existing host revision marker only after the whole import succeeds.
                    if (depth === 0 && entry.name === '.toolpkg-cache-signature') continue;
                    var from = source + '/' + entry.name, to = destination + '/' + entry.name;
                    if (entry.isDirectory) await copyTree(from, to, depth + 1);
                    else {
                        var data = await canonical.Files.readBinary(from);
                        await invoke('file', { operation: 'writeBinary', args: { path: to, contentBase64: data.contentBase64 } });
                    }
                }
            }
            pending = (async function() {
                var marker = '/.toolpkg-cache-signature';
                var revision = await canonical.Files.readBinary(address.source + marker);
                var existing = await invoke('file', { operation: 'exists', args: { path: address.destination + marker } });
                if (existing.exists) {
                    var previous = await invoke('file', { operation: 'readBinary', args: { path: address.destination + marker } });
                    if (previous.contentBase64 === revision.contentBase64) return;
                    await invoke('file', { operation: 'deleteFile', args: { path: address.destination + marker } });
                }
                await copyTree(address.source, address.destination, 0);
                await invoke('file', { operation: 'writeBinary', args: { path: address.destination + marker, contentBase64: revision.contentBase64 } });
            })();
            cacheImports.set(address.source, pending);
            pending.catch(function() { cacheImports.delete(address.source); });
        }
        await pending;
    }

    /** Translates only this container's old Android cache namespace.
     * The replacement contains shell-safe characters; quotes and suffix globs remain intact,
     * including in a quoted bash -lc script. This is namespace translation, not shell parsing.
     * @param {string} value
     * @returns {Promise<string>}
     */
    async function cachePath(value) {
        if (typeof value !== 'string' || value.indexOf('/toolpkg_cache/') === -1) return value;
        var address = cacheAddress();
        if (!address) throw new Error('V1 cache path requires a current ToolPkg container');
        var mapped = value.replace(address.pattern, address.destination);
        if (mapped !== value) await importCache(address);
        return mapped;
    }

    /** Rewrites only admitted legacy package identities while preserving every public overload.
     * @param {...any} args
     * @returns {Promise<any>}
     */
    async function legacyToolCall(...args) {
        var rewritten = args.slice();
        var name;
        var type = 'default';
        var index = 0;
        if (args[0] !== null && typeof args[0] === 'object') {
            name = args[0].name;
            type = args[0].type === undefined ? 'default' : args[0].type;
        } else {
            if (typeof args[1] === 'string') { type = args[0]; index = 1; }
            name = args[index];
        }
        if (type === 'default' && Object.prototype.hasOwnProperty.call(routes, name)) {
            var target = routes[/** @type {keyof typeof routes} */ (name)];
            if (args[0] !== null && typeof args[0] === 'object') rewritten[0] = Object.assign({}, args[0], { name: target });
            else rewritten[index] = target;
            if (name === 'super_admin:terminal') {
                var objectCall = args[0] !== null && typeof args[0] === 'object';
                var parameters = objectCall ? args[0].params : args[index + 1];
                if (parameters && typeof parameters.command === 'string') {
                    var mapped = Object.assign({}, parameters, { command: await cachePath(parameters.command) });
                    if (objectCall) rewritten[0].params = mapped;
                    else rewritten[index + 1] = mapped;
                }
            }
            return originalToolCall.apply(runtime, rewritten).then(decoded);
        }
        return originalToolCall.apply(runtime, args);
    }
    runtime.__operitExpose('toolCall', api.method()
        .between('1.0.0', '2.0.0', legacyToolCall)
        .since('2.0.0', originalToolCall)
        .build('toolCall'));

    var terminal = canonical.System.terminal;
    var currentTerminal = runtime.Tools.System.terminal;
    currentTerminal.create = api.method()
        .between('1.0.0', '2.0.0',
            /** Creates a legacy session using the package state selected by Rust.
             * @param {string} sessionName
             */
            function(sessionName) { return invoke('create_session', { sessionName: sessionName }); })
        .since('2.0.0', terminal.create).build('Tools.System.terminal.create');
    currentTerminal.exec = api.method()
        .between('1.0.0', '2.0.0',
            /** Executes in the exact compatibility session without selecting another interpreter.
             * @param {string} sessionId
             * @param {string} command
             * @param {number | string} [timeoutMs]
             */
            async function(sessionId, command, timeoutMs) {
                var params = { sessionId: sessionId, command: await cachePath(command) };
                if (timeoutMs !== undefined) /** @type {Record<string, any>} */ (params).timeoutMs = timeoutMs;
                return invoke('execute', params);
            })
        .since('2.0.0', terminal.exec).build('Tools.System.terminal.exec');

    currentTerminal.execStreaming = api.method()
        .between('1.0.0', '2.0.0',
            /** Streams the selected compatibility session through the canonical terminal transport.
             * @param {string} sessionId
             * @param {string} command
             * @param {{timeoutMs?: number | string, onIntermediateResult?: (event: any) => void}} [options]
             */
            async function(sessionId, command, options) {
                if (options !== undefined && (options === null || typeof options !== 'object' || Array.isArray(options))) throw new TypeError('Terminal streaming options must be an object');
                var controls = Object.assign({}, options);
                if (controls.timeoutMs === undefined) controls.timeoutMs = 15000;
                return terminal.execStreaming(sessionId, await cachePath(command), controls);
            })
        .since('2.0.0', terminal.execStreaming).build('Tools.System.terminal.execStreaming');

    /** Identifies the explicit legacy Linux VFS namespace by complete path segments.
     * @param {unknown} path
     * @returns {path is string}
     */
    function isLinux(path) { return typeof path === 'string' && (path === '/mnt/linux' || path.startsWith('/mnt/linux/')); }

    /** Converts one explicit Linux VFS address into the selected runtime's absolute path.
     * @param {string} path
     */
    function linuxPath(path) {
        if (!isLinux(path)) throw new Error('V1 runtime file operation requires a Linux VFS address');
        var suffix = path.slice('/mnt/linux'.length);
        return suffix === '' ? '/' : suffix;
    }

    /** Runs one file operation and restores its canonical VFS result path for the existing Files adapter.
     * @param {string} operation
     * @param {Record<string, any>} args
     * @returns {Promise<any>}
     */
    async function file(operation, args) {
        var value = await invoke('file', { operation: operation, args: args });
        value.path = '/mnt/linux' + (value.path === '/' ? '' : value.path);
        return value;
    }
    var files = Object.assign({}, canonical.Files);
    for (var method of ['read', 'readBinary', 'write', 'writeBinary', 'exists', 'mkdir', 'deleteFile', 'move', 'copy', 'unzip']) {
        /** Installs a single known filesystem operation without mutating the canonical namespace.
         * @param {string} name
         */
        (function(name) {
            /** Routes the explicit Linux namespace to its actual managed runtime filesystem.
             * @param {...any} args
             */
            files[name] = async function(...args) {
                var path = name === 'read' && typeof args[0] === 'object' && args[0] !== null ? args[0].path : args[0];
                var originalPath = path;
                var mappedPath = await cachePath(path);
                if (mappedPath !== path) {
                    path = '/mnt/linux' + mappedPath;
                    args[0] = typeof args[0] === 'object' ? Object.assign({}, args[0], { path: path }) : path;
                }
                if (['move', 'copy', 'unzip'].includes(name)) {
                    var mappedDestination = await cachePath(args[1]);
                    if (mappedDestination !== args[1]) args[1] = '/mnt/linux' + mappedDestination;
                }
                var sourceLinux = isLinux(path);
                var hasDestination = name === 'move' || name === 'copy' || name === 'unzip';
                var destinationLinux = hasDestination && isLinux(args[1]);
                if (!sourceLinux && !destinationLinux) return canonical.Files[name].apply(canonical.Files, args);
                if (name === 'copy' && sourceLinux !== destinationLinux) {
                    if (args[2] === true) throw new Error('V1 cross-environment directory copy requires an explicit archive');
                    var data = sourceLinux ? await file('readBinary', { path: linuxPath(path) }) : await canonical.Files.readBinary(path);
                    var copied = destinationLinux
                        ? await file('writeBinary', { path: linuxPath(args[1]), contentBase64: data.contentBase64 })
                        : await canonical.Files.writeBinary(args[1], data.contentBase64);
                    return Object.assign({}, copied, { operation: 'copy', path: originalPath });
                }
                if (hasDestination && (!sourceLinux || !destinationLinux)) throw new Error('V1 ' + name + ' requires both paths in the same runtime filesystem');
                var params = { path: linuxPath(path) };
                switch (name) {
                    case 'read':
                        if (typeof args[0] === 'object' && Object.keys(args[0]).some(
                            /** Identifies actual read options without inspecting text contents. */
                            function(key) { return key !== 'path'; })) throw new Error('V1 runtime read supports text paths; semantic and image reads require their host API');
                        break;
                    case 'write': Object.assign(params, { content: args[1], append: args[2] === true }); break;
                    case 'writeBinary': Object.assign(params, { contentBase64: args[1] }); break;
                    case 'mkdir': Object.assign(params, { createParents: args[1] === true }); break;
                    case 'deleteFile': Object.assign(params, { recursive: args[1] === true }); break;
                    case 'move': case 'copy': case 'unzip': Object.assign(params, { destination: linuxPath(args[1]), recursive: args[2] === true }); break;
                }
                var value = await file(name, params);
                return mappedPath === originalPath ? value : Object.assign({}, value, { path: originalPath });
            };
        })(method);
    }
    var legacyFiles = __operitCreateV1Files(files);
    for (var name of Object.keys(legacyFiles)) {
        runtime.Tools.Files[name] = api.method()
            .between('1.0.0', '2.0.0', legacyFiles[/** @type {keyof typeof legacyFiles} */ (name)])
            .since('2.0.0', canonical.Files[/** @type {keyof typeof canonical.Files} */ (name)])
            .build('Tools.Files.' + name);
    }
}
