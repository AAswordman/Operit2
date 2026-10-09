/// Builds public Java compatibility wrappers over the required structured host bindings.
#[allow(non_snake_case)]
pub fn buildJavaClassBridgeDefinition() -> String {
    r#"
        (function() {
            /** Queries exact class metadata through the structured host binding. */
            function classExistsRaw(className) {
                return __operitNativeJavaClassExists(String(className));
            }

            /** Recognizes an explicit Java package or class property key. */
            function isPropertyKeyName(value) {
                return typeof value === 'string' && value.length > 0;
            }

            var __operitJavaInstanceProxies = {};

            /** Wraps an explicit host-owned Java descriptor without changing its structured arguments. */
            function createInstanceProxy(className, handle) {
                if (__operitJavaInstanceProxies[handle]) {
                    return __operitJavaInstanceProxies[handle];
                }
                var target = {
                    __javaClass: className,
                    __javaHandle: handle,
                    className: className,
                    handle: handle,
                    call: function(methodName) {
                        var args = Array.prototype.slice.call(arguments, 1);
                        return invokeBridge('javaCallInstance', [
                            handle,
                            String(methodName || ''),
                            normalizeArgs(args)
                        ]);
                    },
                    toJSON: function() {
                        return {
                            __javaHandle: handle,
                            __javaClass: className
                        };
                    },
                    toString: function() {
                        return invokeBridge('javaCallInstance', [handle, 'toString', []]);
                    }
                };
                var proxy = new Proxy(target, {
                    get: function(obj, prop) {
                        if (prop in obj) {
                            return obj[prop];
                        }
                        if (prop === Symbol.toStringTag) {
                            return 'JavaObject';
                        }
                        if (prop === 'then') {
                            return undefined;
                        }
                        if (typeof prop !== 'string') {
                            return undefined;
                        }
                        return function() {
                            var args = Array.prototype.slice.call(arguments);
                            return invokeBridge('javaCallInstance', [
                                handle,
                                prop,
                                normalizeArgs(args)
                            ]);
                        };
                    }
                });
                __operitJavaInstanceProxies[handle] = proxy;
                return proxy;
            }

            /** Wraps only descriptors with the exact Java handle fields, preserving other application values. */
            function normalizeBridgeValue(value) {
                if (
                    value &&
                    typeof value === 'object' &&
                    typeof value.__javaHandle === 'string' &&
                    typeof value.__javaClass === 'string'
                ) {
                    return createInstanceProxy(value.__javaClass, value.__javaHandle);
                }
                return value;
            }

            /** Dispatches a named Java operation with structured values and direct host errors. */
            function invokeBridge(methodName, args) {
                var bindings = {
                    javaClassExists: __operitNativeJavaClassExists,
                    javaGetApplicationContext: __operitNativeJavaGetApplicationContext,
                    javaGetCurrentActivity: __operitNativeJavaGetCurrentActivity,
                    javaNewInstance: __operitNativeJavaNewInstance,
                    javaCallStatic: __operitNativeJavaCallStatic,
                    javaCallInstance: __operitNativeJavaCallInstance,
                    javaCallStaticSuspend: __operitNativeJavaCallStaticSuspend,
                    javaGetStaticField: __operitNativeJavaGetStaticField,
                    javaSetStaticField: __operitNativeJavaSetStaticField
                };
                if (!Object.prototype.hasOwnProperty.call(bindings, methodName)) {
                    throw new Error('Unknown Java host operation: ' + methodName);
                }
                return normalizeBridgeValue(bindings[methodName].apply(undefined, args));
            }

            /** Copies the public Java argument list into a structured array. */
            function normalizeArgs(args) {
                return Array.prototype.slice.call(args || []);
            }

            /** Exposes explicit class operations without guessing another member kind after an error. */
            function createClassProxy(className) {
                var target = function() {
                    return target.newInstance.apply(target, arguments);
                };
                target.className = className;
                target.exists = function() {
                    return classExistsRaw(className);
                };
                target.newInstance = function() {
                    return invokeBridge('javaNewInstance', [
                        className,
                        normalizeArgs(arguments)
                    ]);
                };
                target.callStatic = function(methodName) {
                    var args = Array.prototype.slice.call(arguments, 1);
                    return invokeBridge('javaCallStatic', [
                        className,
                        String(methodName || ''),
                        normalizeArgs(args)
                    ]);
                };
                target.callSuspend = function(methodName) {
                    var args = Array.prototype.slice.call(arguments, 1);
                    return invokeBridge('javaCallStaticSuspend', [
                        className,
                        String(methodName || ''),
                        normalizeArgs(args)
                    ]);
                };
                target.getStatic = function(fieldName) {
                    return invokeBridge('javaGetStaticField', [
                        className,
                        String(fieldName || '')
                    ]);
                };
                target.setStatic = function(fieldName, value) {
                    return invokeBridge('javaSetStaticField', [
                        className,
                        String(fieldName || ''),
                        value
                    ]);
                };
                target.toString = function() {
                    return '[JavaClass ' + className + ']';
                };

                return new Proxy(target, {
                    get: function(obj, prop) {
                        if (prop in obj) {
                            return obj[prop];
                        }
                        if (prop === Symbol.toStringTag) {
                            return 'JavaClass';
                        }
                        if (prop === 'then') {
                            return undefined;
                        }
                        if (typeof prop !== 'string') {
                            return undefined;
                        }
                        // Static fields use getStatic; nested classes use their explicit class name.
                        return function() {
                            var args = Array.prototype.slice.call(arguments);
                            return target.callStatic.apply(target, [prop].concat(args));
                        };
                    },
                    apply: function(obj, _thisArg, args) {
                        return obj.newInstance.apply(obj, args || []);
                    },
                    construct: function(obj, args) {
                        return obj.newInstance.apply(obj, args || []);
                    },
                    set: function(obj, prop, value) {
                        if (prop in obj) {
                            obj[prop] = value;
                            return true;
                        }
                        if (typeof prop !== 'string') {
                            return false;
                        }
                        invokeBridge('javaSetStaticField', [
                            className,
                            prop,
                            value
                        ]);
                        return true;
                    }
                });
            }

            /** Resolves package paths using explicit host class metadata. */
            function createPackageProxy(parts) {
                var pathParts = Array.isArray(parts) ? parts.slice() : [];
                var target = function() {
                    var fullName = pathParts.join('.');
                    if (!fullName) {
                        throw new Error('cannot instantiate empty package path');
                    }
                    if (!classExistsRaw(fullName)) {
                        throw new Error('class not found: ' + fullName);
                    }
                    var cls = createClassProxy(fullName);
                    return cls.newInstance.apply(cls, arguments);
                };
                target.path = pathParts.join('.');
                target.toString = function() {
                    return '[JavaPackage ' + target.path + ']';
                };

                return new Proxy(target, {
                    get: function(obj, prop) {
                        if (prop in obj) {
                            return obj[prop];
                        }
                        if (prop === Symbol.toStringTag) {
                            return 'JavaPackage';
                        }
                        if (prop === 'then') {
                            return undefined;
                        }
                        if (!isPropertyKeyName(prop)) {
                            return undefined;
                        }
                        var nextParts = pathParts.concat([prop]);
                        var candidate = nextParts.join('.');
                        if (classExistsRaw(candidate)) {
                            return createClassProxy(candidate);
                        }
                        return createPackageProxy(nextParts);
                    },
                    apply: function(_obj, _thisArg, args) {
                        var fullName = pathParts.join('.');
                        if (!fullName) {
                            throw new Error('cannot call empty package path');
                        }
                        if (!classExistsRaw(fullName)) {
                            throw new Error('class not found: ' + fullName);
                        }
                        var cls = createClassProxy(fullName);
                        return cls.newInstance.apply(cls, args || []);
                    },
                    construct: function(_obj, args) {
                        var fullName = pathParts.join('.');
                        if (!fullName) {
                            throw new Error('cannot construct empty package path');
                        }
                        if (!classExistsRaw(fullName)) {
                            throw new Error('class not found: ' + fullName);
                        }
                        var cls = createClassProxy(fullName);
                        return cls.newInstance.apply(cls, args || []);
                    }
                });
            }

            var JavaApi = {
                type: function(className) {
                    var normalized = String(className || '').trim();
                    if (!normalized) {
                        throw new Error('class name is required');
                    }
                    return createClassProxy(normalized);
                },
                use: function(className) {
                    return this.type(className);
                },
                importClass: function(className) {
                    return this.type(className);
                },
                package: function(packageName) {
                    var normalized = String(packageName || '').trim();
                    if (!normalized) {
                        throw new Error('package name is required');
                    }
                    return createPackageProxy(normalized.split('.').filter(Boolean));
                },
                implement: function(interfaceNameOrNames, impl) {
                    return {
                        __javaJsInterface: true,
                        __javaInterfaces: Array.isArray(interfaceNameOrNames)
                            ? interfaceNameOrNames
                            : (interfaceNameOrNames ? [String(interfaceNameOrNames)] : []),
                        __javaJsValue: impl
                    };
                },
                proxy: function(interfaceNameOrNames, impl) {
                    return this.implement(interfaceNameOrNames, impl);
                },
                classExists: function(className) {
                    var normalized = String(className || '').trim();
                    if (!normalized) {
                        return false;
                    }
                    return classExistsRaw(normalized);
                },
                callStatic: function(className, methodName) {
                    var args = Array.prototype.slice.call(arguments, 2);
                    return invokeBridge('javaCallStatic', [
                        String(className || '').trim(),
                        String(methodName || '').trim(),
                        normalizeArgs(args)
                    ]);
                },
                callSuspend: function(className, methodName) {
                    var args = Array.prototype.slice.call(arguments, 2);
                    return invokeBridge('javaCallStaticSuspend', [
                        String(className || '').trim(),
                        String(methodName || '').trim(),
                        normalizeArgs(args)
                    ]);
                },
                newInstance: function(className) {
                    var args = Array.prototype.slice.call(arguments, 1);
                    return invokeBridge('javaNewInstance', [
                        String(className || '').trim(),
                        normalizeArgs(args)
                    ]);
                },
                getApplicationContext: function() {
                    return invokeBridge('javaGetApplicationContext', []);
                },
                getContext: function() {
                    return this.getApplicationContext();
                },
                getCurrentActivity: function() {
                    return invokeBridge('javaGetCurrentActivity', []);
                },
                getActivity: function() {
                    return this.getCurrentActivity();
                },
                toString: function() {
                    return '[JavaBridge]';
                }
            };

            var JavaBridge = new Proxy(JavaApi, {
                get: function(obj, prop) {
                    if (prop in obj) {
                        return obj[prop];
                    }
                    if (prop === Symbol.toStringTag) {
                        return 'JavaBridge';
                    }
                    if (prop === 'then') {
                        return undefined;
                    }
                    if (!isPropertyKeyName(prop)) {
                        return undefined;
                    }
                    if (classExistsRaw(prop)) {
                        return createClassProxy(prop);
                    }
                    return createPackageProxy([prop]);
                }
            });

            globalThis.Java = JavaBridge;
            globalThis.Kotlin = JavaBridge;
            window.Java = JavaBridge;
            window.Kotlin = JavaBridge;
        })();
    "#
    .to_string()
}
