/**
 * Adapts the handwritten v1 Files contract to the canonical v2 Files implementation.
 * @param {typeof import('../../../../../../../plugins/types/files').Files} current
 * @returns {typeof import('../../../../../../../plugins/types-v1/files').Files}
 */
/**
 * Converts a legacy environment/path pair to a host-owned VFS address.
 * The omitted environment is the documented v1 Android environment, not the host OS.
 * @param {string} path
 * @param {import('../../../../../../../plugins/types-v1/files').FileEnvironment} environment
 */
function __operitV1FileAddress(path, environment) {
    if (typeof path !== 'string' || path[0] !== '/' || path.indexOf('\\') !== -1) {
        throw new Error('ToolPkg v1 Files requires an absolute slash-separated path.');
    }
    if (environment !== 'android' && environment !== 'linux') {
        throw new Error('Unsupported ToolPkg v1 file environment: ' + environment);
    }
    var segments = [];
    for (var segment of path.split('/')) {
        if (segment === '' || segment === '.') continue;
        if (segment === '..') {
            if (segments.length === 0) throw new Error('ToolPkg v1 file path escapes its root.');
            segments.pop();
        } else {
            segments.push(segment);
        }
    }
    var normalized = '/' + segments.join('/');
    // Paths returned by the current resource/config APIs are already VFS addresses.
    if (segments[0] === 'app' || segments[0] === 'mnt') return normalized;
    if (environment === 'linux') return '/mnt/linux' + (normalized === '/' ? '' : normalized);
    if (segments[0] === 'sdcard' || segments[0] === 'data') return normalized;
    if (segments[0] === 'storage' && segments[1] === 'emulated' && segments[2] === '0') {
        return '/sdcard' + (segments.length === 3 ? '' : '/' + segments.slice(3).join('/'));
    }
    throw new Error('ToolPkg v1 Android path has no VFS mapping: ' + normalized);
}

function __operitCreateV1Files(current) {
    /**
     * Converts a legacy environment/path pair to a host-owned VFS address.
     * @param {string} path
     * @param {import('../../../../../../../plugins/types-v1/files').FileEnvironment} environment
     */
    function address(path, environment) {
        return __operitV1FileAddress(path, environment);
    }

    /**
     * Restores the caller's path vocabulary without changing unrelated VFS roots.
     * @param {string} path
     * @param {string} original
     * @param {string} mapped
     */
    function originalPath(path, original, mapped) {
        if (path === mapped) return original;
        var prefix = mapped.replace(/\/$/, '') + '/';
        if (path.startsWith(prefix)) return original.replace(/\/$/, '') + '/' + path.slice(prefix.length);
        return path;
    }

    /**
     * Adds the legacy environment to a path-bearing result without mutating v2 data.
     * @template {{path: string}} T
     * @param {T} value
     * @param {string} path
     * @param {import('../../../../../../../plugins/types-v1/files').FileEnvironment} environment
     */
    function result(value, path, environment) {
        return Object.assign({}, value, {
            env: environment,
            path: originalPath(value.path, path, address(path, environment))
        });
    }

    /**
     * Converts the nested operation returned by apply/create/edit.
     * @param {import('../../../../../../../plugins/types/results').FileApplyResultData} value
     * @param {string} path
     * @param {import('../../../../../../../plugins/types-v1/files').FileEnvironment} environment
     */
    function applied(value, path, environment) {
        return Object.assign({}, value, { operation: result(value.operation, path, environment) });
    }

    /**
     * Restores environment and per-match paths on a search result.
     * @param {import('../../../../../../../plugins/types/results').GrepResultData} value
     * @param {string} path
     * @param {import('../../../../../../../plugins/types-v1/files').FileEnvironment} environment
     * @param {string | undefined} filePattern
     * @returns {import('../../../../../../../plugins/types-v1/results').GrepResultData}
     */
    function searched(value, path, environment, filePattern) {
        var mapped = address(path, environment);
        return Object.assign({}, value, {
            env: environment,
            searchPath: originalPath(value.searchPath, path, mapped),
            filePattern: filePattern,
            /** Converts each known path field; search contents are never rewritten. */
            matches: value.matches.map(function(match) {
                return {
                    filePath: originalPath(match.filePath, path, mapped),
                    /** Converts nullable v2 context to the optional v1 field. */
                    lineMatches: match.lineMatches.map(function(line) {
                        var entry = { lineNumber: line.lineNumber, lineContent: line.lineContent };
                        if (line.matchContext === null) return entry;
                        return Object.assign(entry, { matchContext: line.matchContext });
                    })
                };
            })
        });
    }

    /** @type {typeof import('../../../../../../../plugins/types-v1/files').Files} */
    var legacy = {
        /** Lists a legacy environment directory using the current VFS. */
        async list(path, environment = 'android') {
            return result(await current.list(address(path, environment)), path, environment);
        },
        /** Reads either legacy path or options overload without modifying caller options. */
        async read(pathOrOptions) {
            var options = typeof pathOrOptions === 'string' ? { path: pathOrOptions } : pathOrOptions;
            var { path, environment = 'android', ...rest } = options;
            return result(await current.read(Object.assign({}, rest, { path: address(path, environment) })), path, environment);
        },
        /** Reads a line range with the legacy fourth environment parameter. */
        async readPart(path, startLine, endLine, environment = 'android') {
            return result(await current.readPart(address(path, environment), startLine, endLine), path, environment);
        },
        /** Writes text in the declared legacy environment. */
        async write(path, content, append, environment = 'android') {
            return result(await current.write(address(path, environment), content, append), path, environment);
        },
        /** Writes Base64 bytes in the declared legacy environment. */
        async writeBinary(path, base64Content, environment = 'android') {
            return result(await current.writeBinary(address(path, environment), base64Content), path, environment);
        },
        /** Reads binary data and restores the environment metadata. */
        async readBinary(path, environment = 'android') {
            return result(await current.readBinary(address(path, environment)), path, environment);
        },
        /** Deletes a file through the existing host capability. */
        async deleteFile(path, recursive, environment = 'android') {
            return result(await current.deleteFile(address(path, environment), recursive), path, environment);
        },
        /** Checks existence in the explicitly selected file environment. */
        async exists(path, environment = 'android') {
            return result(await current.exists(address(path, environment)), path, environment);
        },
        /** Moves two paths within one legacy environment. */
        async move(source, destination, environment = 'android') {
            return result(await current.move(address(source, environment), address(destination, environment)), source, environment);
        },
        /** Preserves independently selected source and destination environments. */
        async copy(source, destination, recursive, sourceEnvironment = 'android', destEnvironment = 'android') {
            return result(await current.copy(address(source, sourceEnvironment), address(destination, destEnvironment), recursive), source, sourceEnvironment);
        },
        /** Creates a directory with the legacy third environment argument. */
        async mkdir(path, createParents, environment = 'android') {
            return result(await current.mkdir(address(path, environment), createParents), path, environment);
        },
        /** Restores each found path as well as the root environment. */
        async find(path, pattern, options, environment = 'android') {
            var mapped = address(path, environment);
            var value = result(await current.find(mapped, pattern, options), path, environment);
            /** Converts known search path entries to their original root. */
            value.files = value.files.map(function(file) { return originalPath(file, path, mapped); });
            return value;
        },
        /** Removes the legacy environment from options and restores search metadata. */
        async grep(path, pattern, options = {}) {
            var { environment = 'android', ...rest } = options;
            return searched(await current.grep(address(path, environment), pattern, rest), path, environment, options.file_pattern);
        },
        /** Adapts intent-based searches with the same environment/path rules. */
        async grepContext(path, intent, options = {}) {
            var { environment = 'android', ...rest } = options;
            return searched(await current.grepContext(address(path, environment), intent, rest), path, environment, options.file_pattern);
        },
        /** Reads file metadata while preserving the legacy environment field. */
        async info(path, environment = 'android') {
            return result(await current.info(address(path, environment)), path, environment);
        },
        /** Adapts the nested result of an AI-assisted file operation. */
        async apply(path, type, oldContent, newContent, environment = 'android') {
            return applied(await current.apply(address(path, environment), type, oldContent, newContent), path, environment);
        },
        /** Creates a file and restores its nested operation environment. */
        async create(path, newContent, environment = 'android') {
            return applied(await current.create(address(path, environment), newContent), path, environment);
        },
        /** Edits a file and restores its nested operation environment. */
        async edit(path, oldContent, newContent, environment = 'android') {
            return applied(await current.edit(address(path, environment), oldContent, newContent), path, environment);
        },
        /** Maps the fourth legacy argument to the third current argument. */
        async zip(source, destination, environment = 'android', includeRootDirectory) {
            return result(await current.zip(address(source, environment), address(destination, environment), includeRootDirectory), source, environment);
        },
        /** Extracts an archive through the shared current implementation. */
        async unzip(source, destination, environment = 'android') {
            return result(await current.unzip(address(source, environment), address(destination, environment)), source, environment);
        },
        /** Opens an environment-qualified file through the host. */
        async open(path, environment = 'android') {
            return result(await current.open(address(path, environment)), path, environment);
        },
        /** Shares an environment-qualified file through the host. */
        async share(path, title, environment = 'android') {
            return result(await current.share(address(path, environment), title), path, environment);
        },
        /**
         * Preserves both download overloads and the legacy headers position.
         * @param {string | Parameters<typeof import('../../../../../../../plugins/types-v1/files').Files.download>[0]} urlOrOptions
         * @param {string} [destination]
         * @param {import('../../../../../../../plugins/types-v1/files').FileEnvironment} [environment]
         * @param {Record<string, string>} [headers]
         */
        async download(urlOrOptions, destination, environment = 'android', headers) {
            if (typeof urlOrOptions === 'string') {
                if (typeof destination !== 'string') throw new Error('ToolPkg v1 download destination is required.');
                return result(await current.download(urlOrOptions, address(destination, environment), headers), destination, environment);
            }
            var { destination: target, environment: targetEnvironment = 'android', ...rest } = urlOrOptions;
            return result(await current.download(Object.assign({}, rest, { destination: address(target, targetEnvironment) })), target, targetEnvironment);
        }
    };
    return legacy;
}
