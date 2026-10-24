//! Strict, immutable request snapshots with one-shot Rust ownership transfer.
use super::*;

/// Retains an owned request until its private capsule is consumed or collected.
pub(super) struct OwnedJson {
    value: Option<JsonValue>,
    nodes: usize,
    depth: usize,
    capsule: Option<v8::Weak<v8::Object>>,
}

impl JsonIntrinsics {
    /// Copies only plain data descriptors, without invoking accessors or toJSON.
    pub(crate) fn snapshot_owned<'s>(
        &self,
        scope: &mut v8::PinScope<'s, '_>,
        value: v8::Local<'s, v8::Value>,
    ) -> HostResult<v8::Local<'s, v8::Value>> {
        if !value.is_object() || self.array(scope, value)? {
            return Err(type_error(scope, "Tool parameters must be a JSON object"));
        }
        let mut nodes = 0;
        let mut depth = 0;
        let value = self.read_plain(scope, value, &mut Vec::new(), &mut nodes, 0, &mut depth)?;
        if !value.is_object() {
            return Err(type_error(scope, "Tool parameters must be a JSON object"));
        }
        let mut allocation = Box::new(OwnedJson {
            value: Some(value),
            nodes,
            depth,
            capsule: None,
        });
        let pointer = std::ptr::from_mut(allocation.as_mut());
        let address = pointer as usize;
        let object = v8::Object::new(scope);
        let key = v8::Local::new(scope, &self.capsule_key);
        let external = v8::External::new(scope, pointer.cast());
        if object.set_private(scope, key, external.into()) != Some(true) {
            return Err(engine_error());
        }
        let snapshots = Rc::downgrade(&self.snapshots);
        allocation.capsule = Some(v8::Weak::with_finalizer(
            scope,
            object,
            Box::new(move |_| {
                if let Some(snapshots) = snapshots.upgrade() {
                    snapshots.borrow_mut().remove(&address);
                }
            }),
        ));
        self.snapshots.borrow_mut().insert(address, allocation);
        Ok(object.into())
    }

    /// Moves a private capsule's request once and accounts for its complete subtree.
    pub(super) fn take_owned<'s>(
        &self,
        scope: &mut v8::PinScope<'s, '_>,
        value: v8::Local<'s, v8::Value>,
        nodes: &mut usize,
        depth: usize,
    ) -> HostResult<Option<(JsonValue, usize)>> {
        let Ok(object) = v8::Local::<v8::Object>::try_from(value) else {
            return Ok(None);
        };
        let key = v8::Local::new(scope, &self.capsule_key);
        let external = object.get_private(scope, key).ok_or_else(engine_error)?;
        if external.is_undefined() {
            return Ok(None);
        }
        let external = v8::Local::<v8::External>::try_from(external)
            .expect("Host-owned private snapshot pointer");
        // SAFETY: only this Host can set capsule_key. Its weakly retained Box remains
        // stable until this rooted capsule is collected or the owning isolate drops.
        // No engine call or user code runs while the allocation is borrowed.
        let snapshot = unsafe { &mut *external.value().cast::<OwnedJson>() };
        if nodes.saturating_add(snapshot.nodes.saturating_sub(1)) > MAX_NODES
            || depth.saturating_add(snapshot.depth) > MAX_DEPTH
        {
            return Err(range_error(
                scope,
                "Tool request exceeds structured depth/node limit",
            ));
        }
        *nodes += snapshot.nodes.saturating_sub(1);
        match snapshot.value.take() {
            Some(value) => Ok(Some((value, depth + snapshot.depth))),
            None => Err(type_error(
                scope,
                "Owned tool parameters have already been consumed",
            )),
        }
    }

    /// Validates every own descriptor and snapshots its data under a shared budget.
    fn read_plain<'s>(
        &self,
        scope: &mut v8::PinScope<'s, '_>,
        value: v8::Local<'s, v8::Value>,
        ancestors: &mut Vec<v8::Local<'s, v8::Value>>,
        nodes: &mut usize,
        depth: usize,
        max_depth: &mut usize,
    ) -> HostResult<JsonValue> {
        check_budget(scope, nodes, depth)?;
        *max_depth = (*max_depth).max(depth);
        if value.is_null() {
            return Ok(JsonValue::Null);
        }
        if value.is_boolean() {
            return Ok(JsonValue::Bool(value.boolean_value(scope)));
        }
        if value.is_string() {
            return Ok(JsonValue::String(value.to_rust_string_lossy(scope)));
        }
        if value.is_number() {
            return number(scope, value, true);
        }
        if !value.is_object() {
            return Err(type_error(scope, "Tool request contains a non-JSON value"));
        }
        if let Some((owned, owned_depth)) = self.take_owned(scope, value, nodes, depth)? {
            *max_depth = (*max_depth).max(owned_depth);
            return Ok(owned);
        }
        let object = v8::Local::<v8::Object>::try_from(value).map_err(|_| engine_error())?;
        let array = self.array(scope, value)?;
        if !array {
            let prototype = object.get_prototype(scope).ok_or_else(engine_error)?;
            let plain = v8::Local::new(scope, &self.plain_prototype);
            if !prototype.is_null() && !prototype.strict_equals(plain) {
                return Err(type_error(scope, "Tool request contains a non-JSON value"));
            }
        }
        if ancestors
            .iter()
            .any(|ancestor| ancestor.strict_equals(value))
        {
            return Err(type_error(scope, "Tool request contains a cyclic value"));
        }
        let keys = object
            .get_own_property_names(
                scope,
                v8::GetPropertyNamesArgs {
                    mode: v8::KeyCollectionMode::OwnOnly,
                    property_filter: v8::PropertyFilter::ALL_PROPERTIES,
                    key_conversion: v8::KeyConversionMode::ConvertToString,
                    index_filter: v8::IndexFilter::IncludeIndices,
                },
            )
            .ok_or_else(engine_error)?;
        let length = if array {
            array_length(scope, object)?
        } else {
            0
        };
        if array && keys.length() as usize != length + 1 {
            return Err(type_error(
                scope,
                "Tool request contains non-JSON array properties",
            ));
        }
        if keys.length() as usize
            > MAX_NODES
                .saturating_sub(*nodes)
                .saturating_add(usize::from(array))
        {
            return Err(range_error(
                scope,
                "Tool request exceeds structured node limit",
            ));
        }
        ancestors.push(value);
        let mut fields = Map::new();
        let mut elements = if array {
            vec![JsonValue::Null; length]
        } else {
            Vec::new()
        };
        for index in 0..keys.length() {
            let key = keys.get_index(scope, index).ok_or_else(engine_error)?;
            if !key.is_string() {
                return Err(type_error(scope, "Tool request contains a non-JSON key"));
            }
            let name = key.to_rust_string_lossy(scope);
            if array && name == "length" {
                continue;
            }
            let key = v8::Local::<v8::Name>::try_from(key).map_err(|_| engine_error())?;
            let descriptor = object
                .get_own_property_descriptor(scope, key)
                .ok_or_else(engine_error)?;
            let descriptor = v8::Local::<v8::Object>::try_from(descriptor)
                .map_err(|_| type_error(scope, "Tool request contains a non-JSON property"))?;
            let value_key = text(scope, "value")?;
            let enumerable_key = text(scope, "enumerable")?;
            if descriptor.has_own_property(scope, value_key.into()) != Some(true)
                || !descriptor
                    .get(scope, enumerable_key.into())
                    .ok_or_else(engine_error)?
                    .is_true()
            {
                return Err(type_error(
                    scope,
                    "Tool request contains a non-JSON property",
                ));
            }
            let value = descriptor
                .get(scope, value_key.into())
                .ok_or_else(engine_error)?;
            let value = self.read_plain(scope, value, ancestors, nodes, depth + 1, max_depth)?;
            if array {
                let index = name
                    .parse::<usize>()
                    .ok()
                    .filter(|index| *index < length && index.to_string() == name)
                    .ok_or_else(|| {
                        type_error(
                            scope,
                            "Tool request contains a non-JSON array hole or property",
                        )
                    })?;
                elements[index] = value;
            } else {
                fields.insert(name, value);
            }
        }
        ancestors.pop();
        Ok(if array {
            JsonValue::Array(elements)
        } else {
            JsonValue::Object(fields)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies address handoff moves the original nested allocations instead of copying data.
    #[test]
    fn address_handoff_preserves_nested_string_and_array_allocations() {
        crate::javascript_values::tests::with_values(|scope, intrinsics| {
            let value = evaluate(scope, "({text:'owned request data',rows:[1,2,3]})")?;
            let capsule = intrinsics.snapshot_owned(scope, value)?;
            let (text_pointer, array_pointer) = {
                let snapshots = intrinsics.snapshots.borrow();
                let snapshot = snapshots.values().next().expect("allocated request");
                let request = snapshot.value.as_ref().expect("unconsumed request");
                (
                    request["text"].as_str().unwrap().as_ptr(),
                    request["rows"].as_array().unwrap().as_ptr(),
                )
            };
            let request = intrinsics.from_js(scope, capsule)?.unwrap();
            assert_eq!(request["text"].as_str().unwrap().as_ptr(), text_pointer);
            assert_eq!(request["rows"].as_array().unwrap().as_ptr(), array_pointer);
            Ok(())
        });
    }
}
