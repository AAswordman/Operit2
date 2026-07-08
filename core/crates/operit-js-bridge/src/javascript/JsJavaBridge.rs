#[allow(non_snake_case)]
pub fn buildJavaClassBridgeDefinition() -> String {
    r#"
        (function() {
            function hasNative(methodName) {
                return (
                    typeof NativeInterface !== 'undefined' &&
                    NativeInterface &&
                    typeof NativeInterface[methodName] === 'function'
                );
            }

            function normalizeBridgeBoolean(value) {
                if (value === true || value === false) {
                    return value;
                }
                if (typeof value === 'string') {
                    var normalized = value.trim().toLowerCase();
                    if (normalized === 'true') {
                        return true;
                    }
                    if (normalized === 'false' || normalized === '') {
                        return false;
                    }
                }
                if (typeof value === 'number') {
                    return value !== 0;
                }
                return !!value;
            }

            function classExistsRaw(className) {
                if (!hasNative('javaClassExists')) {
                    return false;
                }
                try {
                    return normalizeBridgeBoolean(
                        NativeInterface.javaClassExists(String(className || ''))
                    );
                } catch (_e) {
                    return false;
                }
            }

            function isPropertyKeyName(value) {
                return typeof value === 'string' && value.length > 0;
            }

            function bridgeUnavailable(methodName) {
                throw new Error('Java bridge native method is unavailable: ' + methodName);
            }

            var __operitJavaInstanceProxies = {};

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
                            JSON.stringify(normalizeArgs(args))
                        ]);
                    },
                    toJSON: function() {
                        return {
                            __javaHandle: handle,
                            __javaClass: className
                        };
                    },
                    toString: function() {
                        return invokeBridge('javaCallInstance', [handle, 'toString', '[]']);
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
                                JSON.stringify(normalizeArgs(args))
                            ]);
                        };
                    }
                });
                __operitJavaInstanceProxies[handle] = proxy;
                return proxy;
            }

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

            function parseBridgeResult(raw) {
                var result = raw;
                if (typeof raw === 'string') {
                    try {
                        result = JSON.parse(raw);
                    } catch (_parseError) {
                        return raw;
                    }
                }
                if (
                    result &&
                    typeof result === 'object' &&
                    Object.prototype.hasOwnProperty.call(result, 'success')
                ) {
                    if (result.success === false) {
                        throw new Error(String(result.message || 'Java bridge call failed'));
                    }
                    return normalizeBridgeValue(result.data);
                }
                return normalizeBridgeValue(result);
            }

            function invokeBridge(methodName, args) {
                if (!hasNative(methodName)) {
                    bridgeUnavailable(methodName);
                }
                return parseBridgeResult(NativeInterface[methodName].apply(NativeInterface, args || []));
            }

            function normalizeArgs(args) {
                return Array.prototype.slice.call(args || []);
            }

            function createClassProxy(className) {
                var target = function() {
                    return target.newInstance.apply(target, arguments);
                };
                target.className = className;
                target.exists = function() {
                    return hasNative('javaClassExists') &&
                        normalizeBridgeBoolean(NativeInterface.javaClassExists(className));
                };
                target.newInstance = function() {
                    return invokeBridge('javaNewInstance', [
                        className,
                        JSON.stringify(normalizeArgs(arguments))
                    ]);
                };
                target.callStatic = function(methodName) {
                    var args = Array.prototype.slice.call(arguments, 1);
                    return invokeBridge('javaCallStatic', [
                        className,
                        String(methodName || ''),
                        JSON.stringify(normalizeArgs(args))
                    ]);
                };
                target.callSuspend = function(methodName) {
                    var args = Array.prototype.slice.call(arguments, 1);
                    return invokeBridge('javaCallStaticSuspend', [
                        className,
                        String(methodName || ''),
                        JSON.stringify(normalizeArgs(args))
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
                        JSON.stringify(value)
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
                        try {
                            return invokeBridge('javaGetStaticField', [className, prop]);
                        } catch (_fieldError) {
                        }
                        var nestedClassName = className + '$' + prop;
                        if (classExistsRaw(nestedClassName)) {
                            return createClassProxy(nestedClassName);
                        }
                        var nestedUpperClassName = className + '$' + prop.toUpperCase();
                        if (
                            nestedUpperClassName !== nestedClassName &&
                            classExistsRaw(nestedUpperClassName)
                        ) {
                            return createClassProxy(nestedUpperClassName);
                        }
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
                            JSON.stringify(value)
                        ]);
                        return true;
                    }
                });
            }

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
                        JSON.stringify(normalizeArgs(args))
                    ]);
                },
                callSuspend: function(className, methodName) {
                    var args = Array.prototype.slice.call(arguments, 2);
                    return invokeBridge('javaCallStaticSuspend', [
                        String(className || '').trim(),
                        String(methodName || '').trim(),
                        JSON.stringify(normalizeArgs(args))
                    ]);
                },
                newInstance: function(className) {
                    var args = Array.prototype.slice.call(arguments, 1);
                    return invokeBridge('javaNewInstance', [
                        String(className || '').trim(),
                        JSON.stringify(normalizeArgs(args))
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
