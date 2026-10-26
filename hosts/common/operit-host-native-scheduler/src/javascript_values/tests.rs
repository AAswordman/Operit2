use super::*;
use crate::javascript_engine::NativeHostJavaScriptRuntime;
use operit_host_api::{HostJavaScriptBootstrap, HostJavaScriptCodeCache, HostJavaScriptRuntime};
use std::sync::Arc;

/// Runs a conversion assertion in a real affine Host runtime.
pub(super) fn with_values(
    operation: impl FnOnce(&mut v8::PinScope<'_, '_>, &JsonIntrinsics) -> HostResult<()>,
) {
    let mut runtime = NativeHostJavaScriptRuntime::new(
        Arc::new(HostJavaScriptBootstrap::default()),
        Arc::new(HostJavaScriptCodeCache::default()),
    )
    .unwrap();
    runtime
        .with_scope(|scope| {
            let intrinsics = JsonIntrinsics::new(scope)?;
            operation(scope, &intrinsics)
        })
        .unwrap();
}

/// Retains a stable Rust allocation behind an unforgeable private V8 External.
#[test]
fn owned_snapshot_uses_a_private_address_and_transfers_ownership_once() {
    with_values(|scope, intrinsics| {
        let value = evaluate(
            scope,
            "({nested:{text:'before'},values:[1,2,3],['__proto__']:{safe:true}})",
        )?;
        let capsule = intrinsics.snapshot_owned(scope, value)?;
        let object = v8::Local::<v8::Object>::try_from(capsule).unwrap();
        let key = v8::Local::new(scope, &intrinsics.capsule_key);
        let external = object.get_private(scope, key).unwrap();
        let address = v8::Local::<v8::External>::try_from(external)
            .unwrap()
            .value();
        assert!(!address.is_null());
        assert!(intrinsics
            .snapshots
            .borrow()
            .contains_key(&(address as usize)));
        assert_eq!(
            intrinsics.from_js(scope, capsule)?.unwrap(),
            serde_json::json!({"nested":{"text":"before"},"values":[1,2,3],"__proto__":{"safe":true}})
        );
        {
            v8::tc_scope!(let scope, scope);
            assert!(intrinsics.from_js(scope, capsule).is_err());
            assert!(scope.has_caught());
        }
        Ok(())
    });
}

/// Rejects accessors without invoking them and preserves the captured request after mutation.
#[test]
fn owned_snapshot_is_immutable_and_rejects_non_plain_data() {
    let mut runtime = NativeHostJavaScriptRuntime::new(
        Arc::new(HostJavaScriptBootstrap::default()),
        Arc::new(HostJavaScriptCodeCache::default()),
    )
    .unwrap();
    runtime.evaluateHostJavaScriptVoid("snapshot", r#"
        var request={nested:{text:'before'},values:[1,2,3]};
        var captured=__operitHostSnapshotJsonObject(request);
        request.nested.text='after';request.values.push(4);
        var getterCalls=0,rejected=0;
        for(var value of [
            {get value(){getterCalls++;return 1;}}, {array:[,1]},
            {array:Object.assign([1],{extra:2})}, {hidden:Object.defineProperty({},'value',{value:1})},
            {value:undefined}, {value:NaN}, {value:new Date()}, {value:Symbol('invalid')}, {value:1n}
        ]) {try{__operitHostSnapshotJsonObject(value);}catch(_){rejected++;}}
        if(getterCalls!==0||rejected!==9)throw Error('Strict snapshot contract changed');
        function readCaptured(){return captured;}
    "#).unwrap();
    assert_eq!(
        runtime
            .callHostJavaScriptFunction("readCaptured", &[])
            .unwrap(),
        serde_json::json!({"nested":{"text":"before"},"values":[1,2,3]})
    );
    assert!(runtime
        .callHostJavaScriptFunction("readCaptured", &[])
        .is_err());
}

/// Releases abandoned address capsules when V8 collects their owning objects.
#[test]
fn snapshots_replace_data_and_release_abandoned_allocations() {
    with_values(|scope, intrinsics| {
        for expression in [
            "({old:1,rows:[{id:1,name:'first'}]})",
            "({next:2,rows:[]})",
            "({rows:[{},[],null,'',true],next:{other:3}})",
        ] {
            v8::scope!(let scope, scope);
            let value = evaluate(scope, expression)?;
            let expected = intrinsics.from_js(scope, value)?;
            let capsule = intrinsics.snapshot_owned(scope, value)?;
            assert_eq!(intrinsics.from_js(scope, capsule)?, expected);
        }
        {
            v8::scope!(let scope, scope);
            let abandoned = evaluate(scope, "({rows:['abandoned allocation']})")?;
            intrinsics.snapshot_owned(scope, abandoned)?;
        }
        scope.low_memory_notification();
        assert!(intrinsics.snapshots.borrow().is_empty());
        Ok(())
    });
}

/// Preserves direct data fields, proxy arrays and decimal number semantics.
#[test]
fn structured_parameters_preserve_data_fields() {
    with_values(|scope, intrinsics| {
        for expression in [
            "({text:'引号\\\"\\n\\u2028',flag:true,nil:null,integer:42,large:1e16})",
            "({rounded:18446744073709550000,negative:-9223372036854775808,exp:1e21,huge:1.79e308})",
            "({drop:undefined,fn:function(){},symbol:Symbol('x'),nan:NaN,inf:Infinity})",
            "({array:[undefined,function(){},Symbol('x'),NaN,,5],nested:{a:1}})",
            "({a:1,get b(){delete this.a;this.c=3;return 2;}})",
            "({shared:(()=>{var a={v:2};return [a,a];})()})",
            "({__proto__:null,['__proto__']:{polluted:true},['constructor']:'plain'})",
            "({array:new Proxy([1,,3],{})})",
            "({array:new Proxy([1,2,3],{get(t,k){return k==='length'?2.9:Reflect.get(t,k);}})})",
            "({typed:new Uint8Array([1,2,3]),map:new Map([['x',1]])})",
            "({get a(){return undefined;},get b(){return null;}})",
        ] {
            let expected = evaluate(scope, &format!("JSON.stringify({expression})"))?.to_rust_string_lossy(scope);
            let value = evaluate(scope, expression)?;
            let actual = intrinsics.from_js(scope, value)?.unwrap();
            assert_eq!(actual, serde_json::from_str::<JsonValue>(&expected).unwrap(), "{expression}");
        }
        Ok(())
    });
}

/// Rejects cyclic values, BigInts, throwing getters and excessive depth without poisoning V8.
#[test]
fn invalid_values_do_not_poison_the_context() {
    with_values(|scope, intrinsics| {
        for expression in [
            "(()=>{var v={};v.self=v;return v;})()",
            "({value:1n})",
            "({get value(){throw Error('getter failure');}})",
            "(()=>{var v={};for(var i=0;i<140;i++)v={child:v};return v;})()",
        ] {
            v8::tc_scope!(let scope, scope);
            let value = evaluate(scope, expression)?;
            assert!(intrinsics.from_js(scope, value).is_err());
            assert!(scope.has_caught());
        }
        let value = evaluate(scope, "({value:{data:4,toJSON(){throw Error('hook used');}}})")?;
        assert_eq!(
            intrinsics.from_js(scope, value)?,
            Some(serde_json::json!({"value":{"data":4}}))
        );
        Ok(())
    });
}

/// Constructs ordinary mutable JS objects and arrays with safe own data properties.
#[test]
fn returned_values_preserve_json_object_and_array_semantics() {
    with_values(|scope, intrinsics| {
        let value = serde_json::json!({"rows":[{"id":0,"name":"one"},{"id":1,"name":"two"}],"__proto__":{"safe":true},"value":null});
        let mapped = to_js(scope, &value)?;
        let key = text(scope, "mapped")?;
        scope
            .get_current_context()
            .global(scope)
            .create_data_property(scope, key.into(), mapped)
            .unwrap();
        evaluate(
            scope,
            r#"
            if(!Array.isArray(mapped.rows)||mapped.rows.length!==2)throw Error('Array identity');
            if(mapped.rows['01']!==undefined||'01'in mapped.rows)throw Error('Noncanonical index');
            if(mapped.rows[0]!==mapped.rows[0]||mapped.rows[0].name!=='one')throw Error('Identity');
            if(!Object.hasOwn(mapped,'__proto__')||mapped.safe!==undefined||!mapped.__proto__.safe)throw Error('Prototype safety');
            if(mapped.rows.map(value=>value.id).join(',')!=='0,1')throw Error('Iteration');
            mapped.rows[0].name='changed';delete mapped.rows[1].name;mapped.rows[1].extra=true;
            if(JSON.stringify(mapped.rows)!=='[{"id":0,"name":"changed"},{"id":1,"extra":true}]')throw Error('Mutation');
            mapped.rows.length=0;mapped.rows.length=2;
            if(0 in mapped.rows||mapped.rows[0]!==undefined)throw Error('Truncation');
            mapped.rows[0]={id:7};Object.seal(mapped);Object.freeze(mapped.rows);
            if(!Object.isSealed(mapped)||!Object.isFrozen(mapped.rows))throw Error('Freeze');
        "#,
        )?;
        assert!(intrinsics.from_js(scope, mapped)?.unwrap().is_object());
        Ok(())
    });
}

/// Direct allocation never invokes inherited setters and preserves an own __proto__ field.
#[test]
fn result_allocation_preserves_own_properties_without_invoking_setters() {
    with_values(|scope, _| {
        evaluate(scope, "globalThis.setterCalls=0;Object.defineProperty(Array.prototype,'0',{set(){setterCalls++;},configurable:true});")?;
        let value = serde_json::json!({"__proto__":{"polluted":true},"rows":[7,{"nested":[8]}],"text":"\"\n\u{2028}中"});
        let result = to_js(scope, &value)?;
        let key = text(scope, "result")?;
        scope
            .get_current_context()
            .global(scope)
            .create_data_property(scope, key.into(), result)
            .unwrap();
        let valid = evaluate(scope, "setterCalls===0&&Object.getPrototypeOf(result)===Object.prototype&&Object.hasOwn(result,'__proto__')&&result.polluted===undefined&&Object.hasOwn(result.rows,'0')&&Object.hasOwn(result.rows[1].nested,'0')")?;
        assert!(valid.is_true());
        let encoded = evaluate(scope, "JSON.stringify(result)")?.to_rust_string_lossy(scope);
        assert_eq!(serde_json::from_str::<JsonValue>(&encoded).unwrap(), value);
        Ok(())
    });
}

/// Measures pointer snapshot transfer and direct result construction with bounded handle scopes.
#[test]
#[ignore = "manual structured bridge benchmark"]
fn owned_value_codec_benchmark() {
    with_values(|scope, intrinsics| {
        let value = evaluate(scope, "({rows:Array.from({length:256},(_,id)=>({id,name:'role-'+id,enabled:true,settings:{prompt:'x'.repeat(64),tags:['a','b','c']}}))})")?;
        let data = intrinsics.from_js(scope, value)?.unwrap();
        for mode in ["input", "output", "serde-copy"] {
            let started = std::time::Instant::now();
            for _ in 0..100 {
                v8::scope!(let scope, scope);
                match mode {
                    "input" => {
                        let capsule = intrinsics.snapshot_owned(scope, value)?;
                        std::hint::black_box(intrinsics.from_js(scope, capsule)?);
                    }
                    "output" => {
                        std::hint::black_box(to_js(scope, &data)?);
                    }
                    "serde-copy" => {
                        std::hint::black_box(serde_json::to_value(&data).unwrap());
                    }
                    _ => unreachable!(),
                }
            }
            eprintln!(
                "BRIDGE_CODEC rows=256 mode={mode} mean_us_per_call={:.3}",
                started.elapsed().as_secs_f64() * 1e4
            );
        }
        Ok(())
    });
}


/// Transfers SDK data without consulting inherited or own serialization hooks or JSON globals.
#[test]
fn sdk_data_transfer_reads_fields_without_serialization_hooks() {
    with_values(|scope, intrinsics| {
        let value = evaluate(scope, r#"
            JSON.parse=JSON.stringify=function(){throw Error('JSON codec used');};
            Object.defineProperty(Object.prototype,'toJSON',{get(){throw Error('Inherited toJSON used');},configurable:true});
            var nested={title:'角色卡',enabled:true};
            Object.defineProperty(nested,'toJSON',{get(){throw Error('Own toJSON used');}});
            ({revision:3,instances:[{id:'row',values:[nested,42,null]}],missing:undefined,['__proto__']:{safe:true}});
        "#)?;
        let actual = intrinsics.arguments_from_js(scope, std::iter::once(value))?;
        assert_eq!(actual, vec![serde_json::json!({"revision":3,"instances":[{"id":"row","values":[{"title":"角色卡","enabled":true},42,null]}],"__proto__":{"safe":true}})]);
        Ok(())
    });
}

/// Rejects invalid direct data without consuming a partial request or poisoning the context.
#[test]
fn sdk_data_transfer_rejects_cycles_and_preserves_subsequent_requests() {
    with_values(|scope, intrinsics| {
        for expression in ["(()=>{let v={};v.self=v;return v;})()", "({value:1n})", "({get value(){throw Error('field failure');}})"] {
            v8::tc_scope!(let scope, scope);
            let value=evaluate(scope, expression)?;
            assert!(intrinsics.arguments_from_js(scope, std::iter::once(value)).is_err());
            assert!(scope.has_caught());
        }
        let value=evaluate(scope,"({rows:[{id:1},{id:2}],empty:[],array:[undefined,,null]})")?;
        assert_eq!(intrinsics.arguments_from_js(scope,std::iter::once(value))?,vec![serde_json::json!({"rows":[{"id":1},{"id":2}],"empty":[],"array":[null,null,null]})]);
        Ok(())
    });
}

/// Transfers a sliced allocation in constant work and revokes all JavaScript aliases before publication.
#[test]
fn byte_arguments_transfer_backing_storage_and_detach_aliases() {
    let bytes = std::cell::RefCell::new(None);
    with_values(|scope, intrinsics| {
        let value = evaluate(scope, "globalThis.storage=new Uint8Array([3,7,11,19]);globalThis.alias=storage.subarray(1,3);alias")?;
        let array = v8::Local::<v8::Uint8Array>::try_from(value).unwrap();
        let store = array.get_backing_store().unwrap();
        let pointer = store.data().unwrap().as_ptr().cast::<u8>();
        let arguments = intrinsics.arguments_from_js(scope, std::iter::once(value))?;
        let [HostJavaScriptValue::Buffer(owned)]: [_; 1] = arguments.try_into().unwrap() else { panic!("Expected binary ownership") };
        assert_eq!(owned.as_ptr(), unsafe { pointer.add(1) });
        assert_eq!(&owned[..], &[7, 11]);
        assert!(evaluate(scope, "storage.byteLength===0&&alias.byteLength===0")?.is_true());
        bytes.replace(Some(owned));
        Ok(())
    });
    assert_eq!(&bytes.into_inner().unwrap()[..], &[7, 11]);
}

/// Rejects shared mutable backing stores instead of publishing a concurrent writer.
#[test]
fn byte_arguments_reject_shared_storage() {
    with_values(|scope, intrinsics| {
        v8::tc_scope!(let scope, scope);
        let value = evaluate(scope, "new Uint8Array(new SharedArrayBuffer(8))")?;
        assert!(intrinsics.arguments_from_js(scope, std::iter::once(value)).is_err());
        Ok(())
    });
}
