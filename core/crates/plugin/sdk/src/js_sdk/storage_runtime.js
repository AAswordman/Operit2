/** Sends typed storage data through the same scoped host Promise transport as other SDK calls. */
function __operitStorageRequest(request) {
    return __operitInvokeHostAsync(__operitNativeStorageRequestAsync, [request]);
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
        if (key !== "path") throw new Error("Unknown storage open option: " + key);
    }
    var result = await __operitStorageRequest({op:"open",path:options.path,kind:kind});
    var closed = false;
    /** Rejects operations after explicit disposal and retains no second durable state. */
    function request(op, data) {
        if (closed) return Promise.reject(new Error("Storage handle is closed"));
        return __operitStorageRequest(Object.assign({op:op,handle:result.handle},data));
    }
    var base = {
        path:result.path,
        /** Disposes this engine-owned handle without deleting persisted data. */
        async close() { if (closed) return; await request("close",{}); closed=true; },
        /** Reads bounded transaction deltas using an opaque decimal cursor. */
        changes(after="0", limit=100) { return request("changes",{after:after,limit:limit}); },
    };
    if (kind === "sqlite") {
        return Object.assign(base, {
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
            /** Executes one write and its change journal inside one Rust transaction. */
            async execute(sql, params=[]) { return (await request("execute",{statement:__operitStorageStatement(sql,params)}))[0]; },
            /** Commits a complete statement batch without holding a transaction across JS awaits. */
            transaction(statements) {
                if (!Array.isArray(statements)) throw new Error("SQL transaction requires statements");
                return request("transaction",{statements:statements.map(/** Encodes one member of an atomic statement batch. */ function(statement) {return __operitStorageStatement(statement.sql,statement.params === undefined ? [] : statement.params);})});
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
            /** Pages through primary-key order without exporting the entire collection. */
            list(after=null,limit=100) {return request("list",{collection:name,after:after,limit:limit});},
            /** Writes one record, including a genuine null value. */
            put(key,value,options) {return request("commit",{mutations:[mutation(name,key,value,false,options)]});},
            /** Removes one exact record with an optional version check. */
            remove(key,options) {return request("commit",{mutations:[mutation(name,key,null,true,options)]});},
            /** Reads a mutable record proxy with explicit atomic persistence. */
            edit(key) {return edit(name,key);},
        };
    }
    if (kind === "objects") {
        return Object.assign(base, {
            collection:collection,
            /** Publishes changes across collections in one atomic versioned transaction. */
            commit(changes) {
                if(!Array.isArray(changes)) throw new Error("Object commit requires mutations");
                return request("commit",{mutations:changes.map(/** Validates one explicit collection mutation. */ function(change) {
                    if(change.op !== "put" && change.op !== "delete") throw new Error("Unknown object mutation");
                    return mutation(change.collection,change.key,change.op === "delete" ? null : change.value,change.op === "delete",change);
                })});
            },
        });
    }
    return Object.assign(base,collection("keys"),{
        /** Publishes set/remove operations as one key-level transaction with optional per-key versions. */
        commit(batch) {
            if(!batch || typeof batch !== "object") throw new Error("DataStore commit requires a batch");
            var changes=[];
            if(batch.set !== undefined) {
                __operitStorageValidate(batch.set,[]);
                if(batch.set === null || Array.isArray(batch.set) || typeof batch.set !== "object") throw new Error("DataStore set must be an object");
                Object.keys(batch.set).forEach(/** Stages one named key write. */ function(key) {changes.push(mutation("keys",key,batch.set[key],false,precondition(key)));});
            }
            if(batch.remove !== undefined) {
                if(!Array.isArray(batch.remove)) throw new Error("DataStore remove must be an array");
                batch.remove.forEach(/** Stages one explicit key deletion. */ function(key) {changes.push(mutation("keys",key,null,true,precondition(key)));});
            }
            /** Uses only explicitly supplied key versions and preserves null as an absence check. */
            function precondition(key) {
                if(batch.expectedVersions !== undefined && Object.prototype.hasOwnProperty.call(batch.expectedVersions,key)) return {expectedVersion:batch.expectedVersions[key]};
                return undefined;
            }
            return request("commit",{mutations:changes});
        },
    });
}

/** Installs ergonomic facades after the generated typed namespace has been created. */
function __operitInstallStorageFacade() {
    Tools.Storage.sqlite={
        /** Opens a plugin-owned parameterized SQL database. */
        open(options) {return __operitStorageOpen("sqlite",options);},
    };
    Tools.Storage.objects={
        /** Opens plugin-owned ObjectBox-style collections on the same transaction engine. */
        open(options) {return __operitStorageOpen("objects",options);},
    };
    Tools.Storage.dataStore={
        /** Opens an atomic key-value store at the explicitly selected path. */
        open(options) {return __operitStorageOpen("data_store",options);},
    };
}
