/** Collects one exact record with atomic initial state and collector-owned cancellation. */
function __operitStorageWatchRecord(openRequest, collection, key, options) {
    var signal=options.signal;
    var after=null, opening=null, closing=null, stopped=false, pending=false;
    /** Opens one watcher-owned handle without changing the parent handle lifetime. */
    function open() {
        if (opening === null) opening=__operitStorageRequest(openRequest);
        return opening;
    }
    /** Cancels a pending wait by closing exactly this watcher's handle. */
    function close() {
        stopped=true;
        if (signal) signal.removeEventListener("abort", abort);
        if (opening === null) return Promise.resolve();
        if (closing === null) closing=opening.then(/** Closes the retained watch handle. */ function(value) {
            return __operitStorageRequest({op:"close",handle:value.handle});
        });
        return closing;
    }
    /** Cancels the retained wait when the standard signal is aborted. */
    function abort() { close().catch(/** Reports cancellation failures on the owning call. */ function(error) { console.error(error); }); }
    if (signal) {
        if (signal.aborted) stopped=true;
        else signal.addEventListener("abort", abort);
    }
    return {
        /** Returns this stream as its own asynchronous iterator. */
        [Symbol.asyncIterator]() {return this;},
        /** Reads the current record and waits for its next version without polling. */
        async next() {
            if (stopped) return {done:true,value:undefined};
            if (pending) throw new Error("Storage watch already has a pending read");
            pending=true;
            try {
                var owned=await open();
                if (stopped) return {done:true,value:undefined};
                var entry=await __operitInvokeHostAsync(__operitNativeWaitPluginStorageRecord,[owned.handle,collection,key,after]);
                if (stopped) return {done:true,value:undefined};
                after={version:entry === null ? null : entry.version};
                return {done:false,value:entry};
            } catch(error) {
                if (stopped) return {done:true,value:undefined};
                await close();throw error;
            } finally {pending=false;}
        },
        /** Stops an outstanding wait without closing the parent database. */
        async return() {await close();return {done:true,value:undefined};},
        /** Releases the watch and preserves the consumer's failure. */
        async throw(error) {await close();throw error;},
    };
}

/** Sends typed storage data through the same scoped host Promise transport as other SDK calls. */
function __operitStorageRequest(request) {
    return Tools.Storage.request(request).then(function(result) {
        switch (result.op) {
            case 'open': return { handle: result.handle, path: result.path };
            case 'close': case 'preferences_cancel_edit': return null;
            case 'preferences_data': case 'preferences_finish_edit': return result.snapshot;
            case 'preferences_begin_edit': return result.edit;
            case 'query': return result.rows;
            case 'execute': case 'transaction': return result.results;
            case 'define_table': return { changed: result.changed };
            case 'get': return result.entry;
            case 'get_many': return result.entries;
            case 'list': return result.entries;
            case 'changes': return result.page;
            case 'commit': return { version: result.version };
            default: throw new Error('Unknown Storage tool result operation');
        }
    });
}

/** Rejects unsupported structured data instead of silently dropping values. */
function __operitStorageValidate(value, ancestors) {
    if (value === null || typeof value === "string" || typeof value === "boolean") return;
    if (typeof value === "number" && Number.isFinite(value)) return;
    if (typeof value !== "object" || (!Array.isArray(value) && Object.getPrototypeOf(value) !== Object.prototype)) throw new Error("Storage values require plain structured data");
    if (ancestors.indexOf(value) >= 0) throw new Error("Storage values cannot contain cycles");
    ancestors.push(value);
    var keys = Reflect.ownKeys(value);
    for (var index = 0; index < keys.length; index++) {
        var key = keys[index];
        if (Array.isArray(value) && key === "length") continue;
        var descriptor = Object.getOwnPropertyDescriptor(value, key);
        if (typeof key !== "string" || !descriptor.enumerable || !Object.prototype.hasOwnProperty.call(descriptor, "value")) throw new Error("Storage values cannot contain hidden properties or accessors");
        __operitStorageValidate(descriptor.value, ancestors);
    }
    if (Array.isArray(value)) {
        for (var offset = 0; offset < value.length; offset++) if (!Object.prototype.hasOwnProperty.call(value, offset)) throw new Error("Storage arrays cannot contain holes");
        if (keys.length !== value.length + 1) throw new Error("Storage arrays cannot contain named properties");
    }
    ancestors.pop();
}

/** Encodes SQL scalar arguments without losing int64 or binary identity. */
function __operitStorageSqlValue(value) {
    if (value === null) return {kind:"null"};
    if (typeof value === "string") return {kind:"text",value:value};
    if (typeof value === "bigint") {
        if (value < -9223372036854775808n || value > 9223372036854775807n) throw new Error("SQL integer exceeds int64");
        return {kind:"integer",value:String(value)};
    }
    if (typeof value === "number") {
        if (!Number.isFinite(value)) throw new Error("SQL number must be finite");
        if (Number.isInteger(value)) {
            if (!Number.isSafeInteger(value)) throw new Error("Use bigint for SQL integers outside the safe number range");
            return {kind:"integer",value:String(value)};
        }
        return {kind:"real",value:value};
    }
    if (value instanceof Uint8Array) return {kind:"blob",value:Array.from(value)};
    throw new Error("Unsupported SQL parameter type");
}

/** Decodes explicitly tagged SQL results, retaining all integer values as bigint. */
function __operitStorageSqlResult(value) {
    switch (value.kind) {
        case "null": return null;
        case "integer": return BigInt(value.value);
        case "text": case "real": return value.value;
        case "blob": return new Uint8Array(value.value);
        default: throw new Error("Unknown SQL result type");
    }
}

/** Builds one parameterized statement with explicit validation. */
function __operitStorageStatement(sql, params) {
    if (typeof sql !== "string" || sql.trim() === "") throw new Error("SQL text is required");
    if (!Array.isArray(params)) throw new Error("SQL params must be an array");
    return {sql:sql,params:params.map(__operitStorageSqlValue)};
}

/** Opens one explicitly owned database and constructs a retained JS handle facade. */
async function __operitStorageOpen(kind, options) {
    if (!options || typeof options.path !== "string" || options.path.trim() === "") throw new Error("Storage path is required");
    for (const key of Object.keys(options)) {
        if (key !== "path" && key !== "merge") throw new Error("Unknown storage open option: " + key);
    }
    var openRequest = {op:"open",path:options.path,kind:kind};
    if (Object.prototype.hasOwnProperty.call(options, "merge")) openRequest.merge = options.merge;
    var openResult = await __operitStorageRequest(openRequest);
    var closed = false;
    /** Rejects operations after explicit disposal and retains no second durable state. */
    function request(op, data) {
        if (closed) return Promise.reject(new Error("Storage handle is closed"));
        return __operitStorageRequest(Object.assign({op:op,handle:openResult.handle},data));
    }
    var base = {
        path:openResult.path,
        /** Returns exact typed columns from caller-selected SQL. */
        async query(sql, params=[]) {
            var rows = await request("query",{statement:__operitStorageStatement(sql,params)});
            return rows.map(/** Decodes one typed SQL row. */ function(row) {
                var object = Object.create(null);
                row.columns.forEach(/** Assigns one uniquely named result column. */ function(column,index) {
                    if (Object.prototype.hasOwnProperty.call(object,column)) throw new Error("SQL result contains duplicate column names; use aliases");
                    object[column]=__operitStorageSqlResult(row.values[index]);
                });
                return object;
            });
        },

        /** Disposes this engine-owned handle without deleting persisted data. */
        async close() { if (closed) return; await request("close",{}); closed=true; },

    };
    if (kind === "sqlite") {
        return Object.assign(base, {
            /** Executes one write and its change journal inside one Rust transaction. */
            async execute(sql, params=[]) { return (await request("execute",{statement:__operitStorageStatement(sql,params)}))[0]; },
            /** Commits a complete statement batch without holding a transaction across JS awaits. */
            transaction(statements, options = {}) {
                if (!Array.isArray(statements)) throw new Error("SQL transaction requires statements");
                var data = {statements:statements.map(/** Encodes one member of an atomic statement batch with its write-count precondition. */ function(statement) {
                    var encoded = __operitStorageStatement(statement.sql,statement.params === undefined ? [] : statement.params);
                    if (statement.expectedAffectedRows !== undefined) {
                        if (!Number.isSafeInteger(statement.expectedAffectedRows) || statement.expectedAffectedRows < 0) throw new Error("expectedAffectedRows must be a nonnegative safe integer");
                        encoded.expectedAffectedRows = statement.expectedAffectedRows;
                    }
                    return encoded;
                })};
                if (options.whenAbsent !== undefined) data.whenAbsent = options.whenAbsent;
                return request("transaction",data);
            },
            /** Declares a typed synchronized table and installs primary-key dirty tracking. */
            defineTable(table) { return request("define_table",{table:table}); },
        });
    }
    /** Creates one explicit mutation with a distinct null value and deletion operation. */
    function mutation(collection,key,value,deleted,options) {
        if (typeof collection !== "string" || typeof key !== "string") throw new Error("Collection and key must be strings");
        __operitStorageValidate(value,[]);
        var checked=options !== undefined && Object.prototype.hasOwnProperty.call(options,"expectedVersion");
        var version=checked ? options.expectedVersion : null;
        if (checked && version !== null && typeof version !== "string") throw new Error("Expected version must be a string or null");
        return {collection:collection,key:key,value:value,deleted:deleted,checkVersion:checked,expectedVersion:version};
    }
    /** Captures changes to a versioned record and flushes only after explicit caller commitment. */
    async function edit(collection,key) {
        var entry=await request("get",{collection:collection,key:key});
        if (entry === null || entry.value === null || typeof entry.value !== "object" || Array.isArray(entry.value)) throw new Error("Editable record must be an existing object");
        var value=entry.value, dirty=false, saving=false, version=entry.version;
        var proxies=new WeakMap();
        /** Recursively tracks mutations without JSON cloning or implicit asynchronous writes. */
        function proxy(object) {
            if (proxies.has(object)) return proxies.get(object);
            var wrapped=new Proxy(object, {
                /** Wraps actual nested object fields without materializing a second record tree. */
                get(target,property) {var next=Reflect.get(target,property); return next !== null && typeof next === "object" ? proxy(next) : next;},
                /** Validates assignments before changing the pending local object. */
                set(target,property,next) {
                    if (saving) throw new Error("Record is being committed");
                    if (typeof property !== "string") throw new Error("Storage property must be a string");
                    __operitStorageValidate(next,[]);
                    if (property === "__proto__") throw new Error("Storage proxy prototypes cannot change");
                    if (!Reflect.set(target,property,next)) throw new Error("Storage property assignment was rejected"); dirty=true; return true;
                },
                /** Tracks explicit removals and rejects mutation during a commit. */
                deleteProperty(target,property) {if(saving) throw new Error("Record is being committed"); var removed=Reflect.deleteProperty(target,property); dirty=true;return removed;},
                /** Rejects hidden or accessor fields that cannot be persisted faithfully. */
                defineProperty() {throw new Error("Storage proxies require ordinary property assignment");},
                /** Keeps records plain and serializable. */
                setPrototypeOf() {throw new Error("Storage record prototypes cannot change");},
            });
            proxies.set(object,wrapped);return wrapped;
        }
        return {
            value:proxy(value),
            /** Publishes this record with its exact read version, or keeps the dirty state on failure. */
            async flush() {
                if(saving) throw new Error("Record commit is already running");
                if(!dirty) return {version:version};
                __operitStorageValidate(value,[]);saving=true;
                try {var receipt=await request("commit",{mutations:[mutation(collection,key,value,false,{expectedVersion:version})]});version=receipt.version;dirty=false;return receipt;}
                finally {saving=false;}
            },
        };
    }
    /** Selects a collection without opening a second database connection. */
    function collection(name) {
        return {
            /** Reads one exact key and its compare-and-set version. */
            get(key) {return request("get",{collection:name,key:key});},
            /** Reads up to 1000 ordered identities in one consistent host snapshot. */
            getMany(keys) {
                if (!Array.isArray(keys) || keys.length > 1000 || keys.some(/** Rejects nonstring identities before dispatch. */ function(key) {return typeof key !== "string";}))
                    return Promise.reject(new Error("Object batch read requires up to 1000 string keys"));
                if (keys.length === 0) {
                    if (closed) return Promise.reject(new Error("Storage handle is closed"));
                    return Promise.resolve([]);
                }
                return request("get_many",{collection:name,keys:keys});
            },
            /** Pages through primary-key order without exporting the entire collection. */
            list(after=null,limit=100) {return request("list",{collection:name,after:after,limit:limit});},
            /** Writes one record, including a genuine null value. */
            put(key,value,options) {return request("commit",{mutations:[mutation(name,key,value,false,options)]});},
            /** Removes one exact record with an optional version check. */
            remove(key,options) {return request("commit",{mutations:[mutation(name,key,null,true,options)]});},
            /** Emits only this exact record, including initial absence and subsequent deletion. */
            watch(key,options={}) {
                if (closed) throw new Error("Storage handle is closed");
                return __operitStorageWatchRecord(openRequest,name,key,options);
            },
            /** Reads a mutable record proxy with explicit atomic persistence. */
            edit(key) {return edit(name,key);},
        };
    }
    if (kind === "objects") {
        return Object.assign(base, {
            collection:collection,
            /** Reads all committed object changes, including writes by other engines and sync replay. */
            changes(after, limit = 100) { return request("changes", {after: after, limit: limit}); },
            /** Publishes changes across collections in one atomic versioned transaction. */
            commit(changes, guards = []) {
                if(!Array.isArray(changes)) throw new Error("Object commit requires mutations");
                return request("commit",{guards:guards,mutations:changes.map(/** Validates one explicit collection mutation. */ function(change) {
                    if(change.op !== "put" && change.op !== "delete") throw new Error("Unknown object mutation");
                    return mutation(change.collection,change.key,change.op === "delete" ? null : change.value,change.op === "delete",change);
                })});
            },
        });
    }
    throw new Error("Unknown object database kind");
}

/** Installs ergonomic facades after the generated typed namespace has been created. */
function __operitInstallStorageFacade() {
    Tools.Storage.sqlite={
        /** Opens a plugin-owned parameterized SQL database. */
        open(options) {return __operitStorageOpen("sqlite",options);},
    };
    Tools.Storage.objects={
        /** Opens plugin-owned SQLite object collections on the transaction engine. */
        open(options) {return __operitStorageOpen("objects",options);},
    };
    Tools.Storage.dataStore={
        /** Opens native JSON preferences with snapshot, edit and ordered schema upgrades. */
        open(options) {return __operitPreferencesOpen(options);},
    };
}

/** Copies supported JSON data without preserving aliases to a callback's mutable draft. */
function __operitPreferencesClone(value) {
    if (Array.isArray(value)) return value.map(__operitPreferencesClone);
    if (value !== null && typeof value === "object") {
        return Object.fromEntries(Object.entries(value).map(/** Copies one own JSON property. */ function(entry) {
            return [entry[0], __operitPreferencesClone(entry[1])];
        }));
    }
    return value;
}

/** Freezes a complete read snapshot, including nested arrays and records. */
function __operitPreferencesFreeze(value) {
    if (value !== null && typeof value === "object") {
        Object.values(value).forEach(__operitPreferencesFreeze);
        Object.freeze(value);
    }
    return value;
}

/** Creates the same explicit get/set/remove preference vocabulary as the Rust store. */
function __operitPreferencesView(source, mutable) {
    var values=__operitPreferencesClone(source), active=true, edits=[];
    Object.values(values).forEach(__operitPreferencesFreeze);
    /** Protects the native schema metadata and validates explicit preference identities. */
    function keyAllowed(key) {
        if (typeof key !== "string" || key.length === 0) throw new Error("Preference key must be a nonempty string");
        if (key === "__operit_preferences_schema_version" || key === "__operit_preferences_merge") throw new Error("Preference schema metadata is managed by withSchema");
    }
    /** Rejects retained mutation methods after their callback has completed. */
    function writable() {if (!active) throw new Error("Preference edit callback has completed");}
    var view={
        /** Reads a typed JSON value while preserving missing keys as undefined. */
        get(key) {keyAllowed(key);return Object.prototype.hasOwnProperty.call(values,key) ? values[key] : undefined;},
        /** Reports key presence independently of the stored value. */
        contains(key) {keyAllowed(key);return Object.prototype.hasOwnProperty.call(values,key);},
        /** Enumerates application preferences from this complete snapshot. */
        entries() {return Object.entries(values);},
    };
    if (mutable) {
        /** Stages one structured preference value in this callback. */
        view.set=function(key,value) {
            writable();keyAllowed(key);__operitStorageValidate(value,[]);
            Object.defineProperty(values,key,{value:__operitPreferencesFreeze(__operitPreferencesClone(value)),enumerable:true,writable:true,configurable:true});
            edits.push({op:"set",key:key,value:values[key]});
        };
        /** Stages explicit deletion of one preference key. */
        view.remove=function(key) {writable();keyAllowed(key);delete values[key];edits.push({op:"remove",key:key});};
        /** Stages a safe integer delta while retaining native merge semantics. */
        view.incrementInteger=function(key,delta) {
            writable();keyAllowed(key);
            if (!Number.isSafeInteger(delta)) throw new Error("Preference integer delta must be a safe integer");
            var current=Object.prototype.hasOwnProperty.call(values,key) ? values[key] : 0;
            if (!Number.isSafeInteger(current) || !Number.isSafeInteger(current+delta)) throw new Error("Preference integer increment requires a safe integer result");
            Object.defineProperty(values,key,{value:current+delta,enumerable:true,writable:true,configurable:true});
            edits.push({op:"increment_integer",key:key,delta:delta});
        };
        /** Stages one explicit field assignment while retaining native patch semantics. */
        view.setObjectField=function(key,field,value) {
            writable();keyAllowed(key);
            if (typeof field !== "string") throw new Error("Preference object field must be a string");
            __operitStorageValidate(value,[]);
            var current=Object.prototype.hasOwnProperty.call(values,key) ? values[key] : {};
            if (current === null || Array.isArray(current) || typeof current !== "object") throw new Error("Preference object field update requires an object entry");
            var object=__operitPreferencesClone(current);
            Object.defineProperty(object,field,{value:__operitPreferencesClone(value),enumerable:true,writable:true,configurable:true});
            Object.defineProperty(values,key,{value:__operitPreferencesFreeze(object),enumerable:true,writable:true,configurable:true});
            edits.push({op:"set_object_field",key:key,field:field,value:object[field]});
        };
    } else __operitPreferencesFreeze(values);
    return {view:Object.freeze(view),values:values,edits:edits,
        /** Closes the mutable lifetime before publishing or discarding this draft. */
        seal() {active=false;__operitPreferencesFreeze(values);},
    };
}

/** Opens native preferences with a serialized local callback queue and guarded native publication. */
async function __operitPreferencesOpen(options) {
    if (!options || typeof options.path !== "string" || options.path.trim() === "") throw new Error("DataStore path is required");
    for (const key of Object.keys(options)) if (key !== "path" && key !== "merge") throw new Error("Unknown storage open option: "+key);
    var openRequest={op:"open_preferences",path:options.path};
    if (Object.prototype.hasOwnProperty.call(options,"merge")) openRequest.merge=options.merge;
    var opened=await __operitStorageRequest(openRequest), closed=false, schema=null, started=false;
    var queue=Promise.resolve();
    /** Dispatches only preference operations on the retained authenticated handle. */
    function request(op,data) {
        if (closed) return Promise.reject(new Error("DataStore handle is closed"));
        return __operitStorageRequest(Object.assign({op:op,handle:opened.handle},data));
    }
    /** Serializes this handle's operations while allowing later calls to observe an earlier failure. */
    function enqueue(action) {
        started=true;
        var result=queue.then(action);
        queue=result.then(/** Releases the queue after a successful operation. */ function(){},
            /** Releases the queue after propagating a rejected operation to its caller. */ function(){});
        return result;
    }
    /** Rejects unsupported future data before any user callback executes. */
    function checkVersion(version) {
        if (schema !== null && version > schema.version) throw new Error("Preference schema version "+version+" is newer than supported version "+schema.version);
    }
    /** Applies ordered one-version migrations and one edit in a single native publication. */
    async function edit(transform) {
        var edit=await request("preferences_begin_edit",{}), submitted=false;
        var draft=__operitPreferencesView(edit.snapshot.values,true), version=edit.snapshot.schemaVersion;
        try {
            checkVersion(version);
            if (schema !== null) {
                while (version < schema.version) {await schema.migrate(version,draft.view);version+=1;}
            }
            if (transform !== null) await transform(draft.view);
            __operitStorageValidate(draft.values,[]);
            draft.seal();submitted=true;
            return await request("preferences_finish_edit",{token:edit.token,edits:draft.edits,schemaVersion:version});
        } catch(error) {
            draft.seal();
            if (!submitted) {
                try {await request("preferences_cancel_edit",{token:edit.token});}
                catch(cancelError) {throw new AggregateError([error,cancelError],"Preference callback failed and its edit could not be released");}
            }
            throw error;
        }
    }
    /** Reads a validated snapshot, applying declared upgrades atomically when required. */
    async function data() {
        var snapshot=await request("preferences_data",{});
        checkVersion(snapshot.schemaVersion);
        if (schema !== null && snapshot.schemaVersion < schema.version) return edit(null);
        return snapshot;
    }
    var facade={path:opened.path,
        /** Declares the supported version before this handle performs any operation. */
        withSchema(currentVersion,migrate) {
            if (started || closed || schema !== null) throw new Error("withSchema must be declared once before using a DataStore handle");
            if (!Number.isInteger(currentVersion) || currentVersion < 0 || currentVersion > 4294967295 || typeof migrate !== "function") throw new Error("withSchema requires a uint32 version and migration callback");
            schema={version:currentVersion,migrate:migrate};return facade;
        },
        /** Reads the complete immutable application preferences. */
        data() {return enqueue(async function(){return __operitPreferencesView((await data()).values,false).view;});},
        /** Executes the callback once and returns the complete committed immutable snapshot. */
        edit(transform) {
            if (typeof transform !== "function") return Promise.reject(new Error("DataStore edit requires a callback"));
            return enqueue(async function(){return __operitPreferencesView((await edit(transform)).values,false).view;});
        },
        /** Returns the native schema version after pending numbered upgrades complete. */
        schemaVersion() {return enqueue(async function(){return (await data()).schemaVersion;});},
        /** Observes complete snapshots using the native journal notification and cancellation bridge. */
        dataFlow(options={}) {
            started=true;
            if (closed) throw new Error("DataStore handle is closed");
            var stream=null, stopped=options.signal !== undefined && options.signal.aborted, pending=false;
            return {
                /** Returns this collector as its own asynchronous iterator. */
                [Symbol.asyncIterator]() {return this;},
                /** Initializes the schema before observing this store's complete snapshots. */
                async next() {
                    if (stopped) return {done:true,value:undefined};
                    if (pending) throw new Error("DataStore flow already has a pending read");
                    pending=true;
                    try {
                        if (stream === null) {
                            await enqueue(data);
                            if (stopped) return {done:true,value:undefined};
                            stream=__operitStorageWatchRecord(openRequest,"__preferences","snapshot",options);
                        }
                        for (;;) {
                            var item=await stream.next();
                            if (item.done || stopped) return {done:true,value:undefined};
                            checkVersion(item.value.value.schemaVersion);
                            if (schema !== null && item.value.value.schemaVersion < schema.version) {
                                await enqueue(data);
                                continue;
                            }
                            return {done:false,value:__operitPreferencesView(item.value.value.values,false).view};
                        }
                    } catch(error) {
                        stopped=true;
                        if (stream !== null) await stream.return();
                        throw error;
                    } finally {pending=false;}
                },
                /** Disposes this collector without closing the owning DataStore. */
                async return() {stopped=true;if(stream !== null) await stream.return();return {done:true,value:undefined};},
                /** Releases the collector and preserves its consumer's failure. */
                async throw(error) {await this.return();throw error;},
            };
        },
        /** Releases the handle after its accepted callbacks finish. */
        close() {return enqueue(async function(){if(closed)return;await request("close",{});closed=true;});},
    };
    return Object.freeze(facade);
}
