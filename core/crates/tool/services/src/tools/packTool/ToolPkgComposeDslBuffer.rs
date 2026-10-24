//! Checked views of the single Compose arena format; native consumers decode only what they use.
use super::*;
use serde_json::{json, Map};

pub struct ComposeBuffer<'a> { bytes: &'a [u8] }

impl<'a> ComposeBuffer<'a> {
    /// Checks fixed response metadata without traversing the update payload.
    pub fn new(bytes: &'a [u8]) -> Result<Self, String> {
        let arena = Self { bytes };
        if bytes.len() < 32 || arena.u32(0)? != 0x31425043 || arena.u32(4)? != 1 || arena.u32(8)? as usize != bytes.len() || arena.u32(12)? > 1 {
            return Err("Invalid Compose buffer header".into());
        }
        Ok(arena)
    }
    /// Reads one bounded little-endian metadata word.
    fn u32(&self, offset: usize) -> Result<u32, String> {
        let data = self.bytes.get(offset..offset.checked_add(4).ok_or("Compose address overflow")?).ok_or("Compose address exceeds buffer")?;
        Ok(u32::from_le_bytes(data.try_into().unwrap()))
    }
    /// Resolves the typed descriptor and validates its complete range.
    fn descriptor(&self, offset: u32) -> Result<(u32, usize, usize), String> {
        let offset = offset as usize;
        if offset < 32 || offset % 8 != 0 || offset.checked_add(16).is_none_or(|end| end > self.bytes.len()) { return Err("Invalid Compose value address".into()); }
        Ok((self.u32(offset)?, self.u32(offset + 4)? as usize, self.u32(offset + 8)? as usize))
    }
    /// Reads one scalar or container while enforcing depth, size and address bounds.
    pub fn value(&self, offset: u32) -> Result<Value, String> { self.read(offset, 0, &mut 0) }
    /// Visits only values explicitly requested by a finite Rust consumer.
    fn read(&self, offset: u32, depth: usize, nodes: &mut usize) -> Result<Value, String> {
        *nodes += 1;
        if depth > 128 || *nodes > 1_000_000 { return Err("Compose value exceeds depth/node limit".into()); }
        let (tag, length, data) = self.descriptor(offset)?;
        let child = |address, nodes: &mut usize| self.read(address, depth + 1, nodes);
        match tag {
            0 => Ok(Value::Null), 1 => Ok(Value::Bool(false)), 2 => Ok(Value::Bool(true)),
            3 => {
                let start = offset as usize + 8;
                let number = f64::from_le_bytes(self.bytes[start..start + 8].try_into().unwrap());
                if !number.is_finite() { return Err("Nonfinite Compose number".into()); }
                if number >= 0.0 && number < u64::MAX as f64 && number.fract() == 0.0 { Ok(Value::from(number as u64)) }
                else if number >= i64::MIN as f64 && number < 0.0 && number.fract() == 0.0 { Ok(Value::from(number as i64)) }
                else { Ok(Value::from(number)) }
            }
            4 => {
                let end = data.checked_add(length.checked_mul(2).ok_or("Compose string overflow")?).ok_or("Compose string overflow")?;
                let bytes = self.bytes.get(data..end).ok_or("Compose string exceeds buffer")?;
                Ok(Value::String(String::from_utf16_lossy(&bytes.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect::<Vec<_>>())))
            }
            5 | 6 => {
                if length > 1_000_000 { return Err("Compose container exceeds node limit".into()); }
                let stride = if tag == 5 { 4 } else { 8 };
                let end = data.checked_add(length.checked_mul(stride).ok_or("Compose container overflow")?).ok_or("Compose container overflow")?;
                if end > self.bytes.len() { return Err("Compose container exceeds buffer".into()); }
                if tag == 5 { (0..length).map(|index| child(self.u32(data + index * 4)?, nodes)).collect::<Result<Vec<_>, _>>().map(Value::Array) }
                else {
                    let mut result = Map::new();
                    for index in 0..length {
                        let key = child(self.u32(data + index * 8)?, nodes)?.as_str().ok_or("Compose field name is not a string")?.to_owned();
                        if result.insert(key, child(self.u32(data + index * 8 + 4)?, nodes)?).is_some() { return Err("Duplicate Compose field name".into()); }
                    }
                    Ok(Value::Object(result))
                }
            }
            7 => Ok(json!({"__composeUndefined":true})),
            8 => Ok(json!({"__actionId":child(data as u32, nodes)?.as_str().ok_or("Invalid Compose event address")?})),
            9 => Ok(json!({"__composeRef":[child(data as u32, nodes)?.as_str().ok_or("Invalid Compose instance address")?,length]})),
            _ => Err("Unknown Compose value tag".into()),
        }
    }
    /// Returns whether the response completes its command without reading its update.
    pub fn is_final(&self) -> Result<bool, String> { Ok(self.u32(12)? == 1) }
    /// Tests update presence without materializing any program or parameter.
    pub fn has_update(&self) -> Result<bool, String> { Ok(self.u32(16)? != 0) }
    /// Reads an application result independently of the indexed layout payload.
    pub fn action_result(&self) -> Result<Option<Value>, String> { let offset = self.u32(20)?; if offset == 0 { Ok(None) } else { self.value(offset).map(Some) } }
    /// Reads the small navigation envelope while keeping the layout arena untouched.
    pub fn navigation(&self) -> Result<Vec<Value>, String> { let value = self.value(self.u32(24)?)?; value.as_array().cloned().ok_or("Compose navigation must be an array".into()) }
    /// Materializes a finite native renderer's view, outside the response delivery path.
    pub fn transaction(&self) -> Result<ToolPkgComposeDslDecodedTransaction, String> {
        let value = self.value(self.u32(16)?)?;
        let values = value.as_array().filter(|values| values.len() == 7).ok_or("Invalid Compose transaction record")?;
        let programs = serde_json::from_value(values[4].clone()).map_err(|error| error.to_string())?;
        let instances = values[5].as_array().ok_or("Invalid Compose instance table")?.iter().map(|value| {
            let fields = value.as_array().filter(|values| values.len() == 4).ok_or("Invalid Compose instance record")?;
            let patches = fields[3].as_array().ok_or("Invalid Compose patch table")?.iter().map(|patch| {
                let fields = patch.as_array().filter(|fields| fields.len() == 3).ok_or("Invalid Compose patch record")?;
                Ok(ToolPkgComposeDslPatch {node: fields[0].as_u64().and_then(|node| node.try_into().ok()).ok_or("Invalid Compose patch node")?,field:fields[1].as_str().ok_or("Invalid Compose patch field")?.into(),value:fields[2].clone()})
            }).collect::<Result<Vec<_>, String>>()?;
            Ok(ToolPkgComposeDslInstance { id:fields[0].as_str().ok_or("Invalid Compose instance identity")?.into(), programId:fields[1].as_str().ok_or("Invalid Compose program identity")?.into(), values:fields[2].as_array().ok_or("Invalid Compose parameters")?.clone(), patches })
        }).collect::<Result<Vec<_>, String>>()?;
        Ok(ToolPkgComposeDslDecodedTransaction { reset:values[0].as_bool().ok_or("Invalid Compose reset flag")?, revision:values[1].as_u64().ok_or("Invalid Compose revision")?, root:ToolPkgComposeDslReference {instanceId:values[2].as_str().ok_or("Invalid Compose root")?.into(),node:values[3].as_u64().and_then(|node|node.try_into().ok()).ok_or("Invalid Compose root node")?}, programs, instances, removed:serde_json::from_value(values[6].clone()).map_err(|error|error.to_string())? })
    }
}

impl ToolPkgComposeDslTransaction {
    /// Reads a native consumer's logical view from the same immutable wire representation.
    pub fn decode(&self) -> Result<ToolPkgComposeDslDecodedTransaction, String> { ComposeBuffer::new(&self.buffer)?.transaction() }
}

/// Writes explicit finite-renderer snapshots using the same indexed arena ABI.
struct SnapshotWriter { bytes: Vec<u8> }
impl SnapshotWriter {
    /// Reserves aligned storage while constructing an explicit snapshot.
    fn allocate(&mut self, length: usize) -> Result<usize, String> {
        let offset = self.bytes.len();
        let end = offset.checked_add(length).and_then(|end| end.checked_add(7)).map(|end| end & !7).filter(|end| *end <= 0x3fffffff).ok_or("Compose snapshot exceeds address range")?;
        self.bytes.resize(end, 0); Ok(offset)
    }
    /// Writes one fixed metadata word.
    fn word(&mut self, offset: usize, value: u32) { self.bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes()); }
    /// Encodes finite host-owned state without introducing another wire representation.
    fn value(&mut self, value: &Value) -> Result<u32, String> {
        let offset = self.allocate(16)?;
        match value {
            Value::Null => {},
            Value::Bool(value) => self.word(offset, if *value { 2 } else { 1 }),
            Value::Number(value) => { self.word(offset, 3); self.bytes[offset + 8..offset + 16].copy_from_slice(&value.as_f64().ok_or("Invalid snapshot number")?.to_le_bytes()); },
            Value::String(value) => {
                let chars: Vec<_> = value.encode_utf16().collect(); let data = self.allocate(chars.len() * 2)?;
                self.word(offset, 4); self.word(offset + 4, chars.len() as u32); self.word(offset + 8, data as u32);
                for (index, value) in chars.iter().enumerate() { self.bytes[data + index * 2..data + index * 2 + 2].copy_from_slice(&value.to_le_bytes()); }
            },
            Value::Array(values) => {
                let addresses = values.iter().map(|value| self.value(value)).collect::<Result<Vec<_>, _>>()?;
                let data = self.allocate(addresses.len() * 4)?;
                self.word(offset, 5); self.word(offset + 4, values.len() as u32); self.word(offset + 8, data as u32);
                for (index, value) in addresses.iter().enumerate() { self.word(data + index * 4, *value); }
            },
            Value::Object(values) => {
                let addresses = values.iter().map(|(key, value)| Ok((self.value(&Value::String(key.clone()))?, self.value(value)?))).collect::<Result<Vec<_>, String>>()?;
                let data = self.allocate(addresses.len() * 8)?;
                self.word(offset, 6); self.word(offset + 4, values.len() as u32); self.word(offset + 8, data as u32);
                for (index, (key, value)) in addresses.iter().enumerate() { self.word(data + index * 8, *key); self.word(data + index * 8 + 4, *value); }
            },
        }
        Ok(offset as u32)
    }
}
impl ToolPkgComposeDslDecodedTransaction {
    /// Creates a binary publication for a finite native renderer's explicit snapshot.
    pub fn encode(&self) -> Result<ToolPkgComposeDslTransaction, String> {
        let sources: Vec<_> = self.instances.iter().map(|source| {
            let patches: Vec<_> = source.patches.iter().map(|patch| json!([patch.node, patch.field, patch.value])).collect();
            json!([source.id, source.programId, source.values, patches])
        }).collect();
        let value = json!([self.reset,self.revision,self.root.instanceId,self.root.node,self.programs,sources,self.removed]);
        let mut writer = SnapshotWriter {bytes:vec![0;32]};
        let update = writer.value(&value)?; let navigation = writer.value(&json!([]))?;
        writer.word(0, 0x31425043); writer.word(4, 1); writer.word(8, writer.bytes.len() as u32); writer.word(12, 1); writer.word(16, update); writer.word(24, navigation);
        Ok(ToolPkgComposeDslTransaction {buffer: operit_link::CoreBuffer(writer.bytes.into())})
    }
}
