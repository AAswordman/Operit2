use std::collections::{BTreeMap, BTreeSet};
use serde_json::{Map, Value, json};
use super::ToolPkgComposeDslProtocol::*;

/// Retains shared programs and source parameters for finite host consumers without expanded node storage.
#[derive(Clone, Default)]
pub struct ToolPkgComposeDslProgramStore {
    programs: BTreeMap<String, ToolPkgComposeDslProgram>,
    instances: BTreeMap<String, ToolPkgComposeDslInstance>,
    root: Option<ToolPkgComposeDslReference>,
    revision: u64,
    parameters: BTreeMap<String, usize>,
    references: BTreeMap<String, BTreeSet<String>>,
}

/// Provides a transient instruction view to a host renderer without retaining a second tree.
pub struct ToolPkgComposeDslView {
    pub node_type: String,
    pub props: BTreeMap<String, Value>,
    pub children: Vec<ToolPkgComposeDslReference>,
    pub slots: BTreeMap<String, Vec<ToolPkgComposeDslReference>>,
}

impl ToolPkgComposeDslProgramStore {
    /// Checks immutable operands once and indexes parameter and local-instruction requirements.
    fn index_operand(code: &Value, count: usize, required: &mut usize, edges: &mut BTreeSet<usize>) -> Result<(), String> {
        let code = code.as_array().ok_or("Compose operand must be an array")?;
        let tag = code.first().and_then(Value::as_u64).ok_or("Invalid Compose operand tag")?;
        match tag {
            0 => { if code.len() != 2 { return Err("Invalid Compose constant".into()); } },
            1 | 4 => {
                if code.len() != 2 { return Err("Invalid Compose operand address".into()); }
                let index: usize = code[1].as_u64().ok_or("Invalid Compose operand index")?.try_into().map_err(|_| "Compose address exceeds usize")?;
                if tag == 1 { *required = (*required).max(index.checked_add(1).ok_or("Compose parameter overflow")?); }
                else { if index >= count { return Err("Unknown Compose local instruction".into()); } edges.insert(index); }
            },
            2 | 3 => {
                let expected = if tag == 2 { 3 } else { 2 };
                if code.len() != expected { return Err("Invalid Compose container operand".into()); }
                let members = code[expected - 1].as_array().ok_or("Invalid Compose operand members")?;
                if tag == 2 {
                    let names = code[1].as_array().ok_or("Invalid Compose operand fields")?;
                    if names.len() != members.len() || names.iter().any(|name| !name.is_string()) || names.iter().map(|name| name.as_str().unwrap()).collect::<BTreeSet<_>>().len() != names.len() { return Err("Invalid Compose operand fields".into()); }
                }
                for member in members { Self::index_operand(member, count, required, edges)?; }
            },
            5 => {
                if code.len() != 3 { return Err("Invalid Compose concatenation".into()); }
                Self::index_operand(&code[1], count, required, edges)?;
                Self::index_operand(&code[2], count, required, edges)?;
            },
            _ => return Err("Unknown Compose operand tag".into()),
        }
        Ok(())
    }

    /// Rejects cycles in code once, independently of the number of repeated instances.
    fn validate_program(program: &ToolPkgComposeDslProgram) -> Result<usize, String> {
        if program.root as usize >= program.nodes.len() { return Err("Compose program root is out of range".into()); }
        let mut required = 0;
        let mut edges = vec![BTreeSet::new(); program.nodes.len()];
        for (node, instruction) in program.nodes.iter().enumerate() {
            Self::index_operand(&instruction.props, program.nodes.len(), &mut required, &mut edges[node])?;
            Self::index_operand(&instruction.children, program.nodes.len(), &mut required, &mut edges[node])?;
        }
        /// Walks immutable local edges using active and completed instruction sets.
        fn visit(node: usize, edges: &[BTreeSet<usize>], active: &mut BTreeSet<usize>, visited: &mut BTreeSet<usize>) -> Result<(), String> {
            if active.contains(&node) { return Err("Compose program instructions form a cycle".into()); }
            if !visited.insert(node) { return Ok(()); }
            active.insert(node);
            for child in &edges[node] { visit(*child, edges, active, visited)?; }
            active.remove(&node);
            Ok(())
        }
        let (mut active, mut visited) = (BTreeSet::new(), BTreeSet::new());
        for node in 0..program.nodes.len() { visit(node, &edges, &mut active, &mut visited)?; }
        Ok(required)
    }

    /// Visits dynamic addresses without materializing layout instructions or retaining a second tree.
    fn inspect_references(value: &Value, output: &mut BTreeSet<ToolPkgComposeDslReference>) -> Result<(), String> {
        if let Some(values) = value.as_array() { for value in values { Self::inspect_references(value, output)?; } }
        else if let Some(values) = value.as_object() {
            if let Some(reference) = values.get("__composeRef") {
                let reference = reference.as_array().ok_or("Invalid Compose reference")?;
                if reference.len() != 2 { return Err("Invalid Compose reference length".into()); }
                output.insert(ToolPkgComposeDslReference { instanceId: reference[0].as_str().ok_or("Invalid Compose instance address")?.into(), node: reference[1].as_u64().ok_or("Invalid Compose local address")?.try_into().map_err(|_| "Compose local address exceeds u32")? });
            } else { for value in values.values() { Self::inspect_references(value, output)?; } }
        }
        Ok(())
    }

    /// Validates indexed program and instance changes completely before publishing an atomic revision.
    pub fn apply(&mut self, update: ToolPkgComposeDslTransaction) -> Result<(), String> {
        self.apply_decoded(update.decode()?)
    }

    /// Applies an explicitly decoded snapshot for finite native renderers.
    pub fn apply_decoded(&mut self, update: ToolPkgComposeDslDecodedTransaction) -> Result<(), String> {
        if !update.reset && update.revision != self.revision + 1 { return Err("Compose transaction revision is out of order".into()); }
        let definitions: BTreeMap<_, _> = update.programs.iter().map(|program| (program.id.as_str(), program)).collect();
        let sources: BTreeMap<_, _> = update.instances.iter().map(|source| (source.id.as_str(), source)).collect();
        let removed: BTreeSet<_> = update.removed.iter().map(String::as_str).collect();
        if definitions.len() != update.programs.len() || sources.len() != update.instances.len() || removed.len() != update.removed.len() { return Err("Duplicate Compose transaction address".into()); }
        let program = |id: &str| definitions.get(id).copied().or_else(|| (!update.reset).then(|| self.programs.get(id)).flatten());
        let instance = |id: &str| sources.get(id).copied().or_else(|| (!update.reset && !removed.contains(id)).then(|| self.instances.get(id)).flatten());
        let mut requirements = BTreeMap::new();
        for definition in &update.programs {
            if !update.reset && self.programs.contains_key(&definition.id) { return Err("Compose program was installed twice".into()); }
            requirements.insert(definition.id.clone(), Self::validate_program(definition)?);
        }
        let mut references = if update.reset { BTreeMap::new() } else { self.references.clone() };
        for source in &update.instances {
            let definition = program(&source.programId).ok_or("Unknown Compose instance program")?;
            let required = requirements.get(&source.programId).or_else(|| self.parameters.get(&source.programId)).ok_or("Missing Compose program index")?;
            if source.values.len() < *required { return Err("Missing Compose instance parameters".into()); }
            if !update.reset && self.instances.get(&source.id).is_some_and(|previous| previous.programId != source.programId) { return Err("Compose source program changed without a new identity".into()); }
            let mut addresses = BTreeSet::new();
            for value in &source.values { Self::inspect_references(value, &mut addresses)?; }
            let mut fields = BTreeSet::new();
            for patch in &source.patches {
                if !fields.insert((patch.node, &patch.field)) || patch.node as usize >= definition.nodes.len() || !matches!(patch.field.as_str(), "type" | "props" | "children" | "slots") { return Err("Invalid Compose public-node patch".into()); }
                if (patch.field == "type" && !patch.value.is_string()) || (matches!(patch.field.as_str(), "props" | "slots") && !patch.value.is_object()) { return Err("Invalid Compose patch value".into()); }
                Self::inspect_references(&patch.value, &mut addresses)?;
            }
            let mut outgoing = BTreeSet::new();
            for address in addresses {
                let target = instance(&address.instanceId).ok_or("Unknown Compose dynamic instance")?;
                if address.node as usize >= program(&target.programId).ok_or("Unknown Compose target program")?.nodes.len() { return Err("Unknown Compose dynamic instruction".into()); }
                if address.instanceId != source.id { outgoing.insert(address.instanceId); }
            }
            references.insert(source.id.clone(), outgoing);
        }
        let root = instance(&update.root.instanceId).ok_or("Unknown Compose root instance")?;
        if update.root.node as usize >= program(&root.programId).ok_or("Unknown Compose root program")?.nodes.len() { return Err("Unknown Compose root instruction".into()); }
        for id in &removed {
            if update.reset || !self.instances.contains_key(*id) || sources.contains_key(*id) { return Err("Invalid Compose source removal".into()); }
            references.remove(*id);
        }
        /// Checks source membership without traversing any source's immutable layout instructions.
        fn visit(id: &str, edges: &BTreeMap<String, BTreeSet<String>>, active: &mut BTreeSet<String>, visited: &mut BTreeSet<String>) -> Result<(), String> {
            if active.contains(id) { return Err("Compose instances form a cycle".into()); }
            if !visited.insert(id.into()) { return Ok(()); }
            active.insert(id.into());
            for target in edges.get(id).ok_or("Compose source retains a removed instance")? { visit(target, edges, active, visited)?; }
            active.remove(id);
            Ok(())
        }
        let (mut active, mut visited) = (BTreeSet::new(), BTreeSet::new());
        for id in references.keys() { visit(id, &references, &mut active, &mut visited)?; }
        if update.reset { self.programs.clear(); self.instances.clear(); self.parameters.clear(); }
        self.parameters.extend(requirements);
        self.references = references;
        for definition in update.programs { self.programs.insert(definition.id.clone(), definition); }
        for source in update.instances { self.instances.insert(source.id.clone(), source); }
        for id in update.removed { self.instances.remove(&id); }
        self.root = Some(update.root);
        self.revision = update.revision;
        Ok(())
    }

    /// Resolves a required source instance.
    fn instance(&self, id: &str) -> Result<&ToolPkgComposeDslInstance, String> { self.instances.get(id).ok_or_else(|| format!("Unknown Compose instance: {id}")) }

    /// Evaluates a program operand for an explicit host read.
    fn operand(&self, instance: &ToolPkgComposeDslInstance, code: &Value) -> Result<Value, String> {
        let code = code.as_array().ok_or("Compose operand must be an array")?;
        let value = |index: usize| code.get(index).ok_or("Truncated Compose operand");
        match value(0)?.as_u64().ok_or("Invalid Compose operand tag")? {
            0 => Ok(value(1)?.clone()),
            1 => Ok(instance.values.get(value(1)?.as_u64().ok_or("Invalid parameter index")? as usize).ok_or("Compose parameter index is out of range")?.clone()),
            2 => {
                let names = value(1)?.as_array().ok_or("Invalid Compose field names")?;
                let values = value(2)?.as_array().ok_or("Invalid Compose field operands")?;
                if names.len() != values.len() { return Err("Compose field count differs".into()); }
                let mut result = Map::new();
                for (name, code) in names.iter().zip(values) { result.insert(name.as_str().ok_or("Invalid Compose field name")?.to_string(), self.operand(instance, code)?); }
                Ok(Value::Object(result))
            },
            3 => Ok(Value::Array(value(1)?.as_array().ok_or("Invalid Compose member operands")?.iter().map(|code| self.operand(instance, code)).collect::<Result<_, _>>()?)),
            4 => Ok(json!({"__composeRef": [instance.id, value(1)?]})),
            5 => {
                let left = self.operand(instance, value(1)?)?;
                let right = self.operand(instance, value(2)?)?;
                let mut result = left.as_array().ok_or("Compose concat requires arrays")?.clone();
                result.extend_from_slice(right.as_array().ok_or("Compose concat requires arrays")?);
                Ok(Value::Array(result))
            },
            tag => Err(format!("Unknown Compose operand: {tag}")),
        }
    }

    /// Collects only logical references from a UI child value.
    fn children(value: &Value, result: &mut Vec<ToolPkgComposeDslReference>) -> Result<(), String> {
        if let Some(array) = value.as_array() { for value in array { Self::children(value, result)?; } }
        else if let Some(reference) = value.get("__composeRef") {
            let reference = reference.as_array().ok_or("Invalid Compose reference")?;
            if reference.len() != 2 { return Err("Invalid Compose reference length".into()); }
            result.push(ToolPkgComposeDslReference { instanceId: reference[0].as_str().ok_or("Invalid Compose instance address")?.into(), node: reference[1].as_u64().ok_or("Invalid Compose local address")?.try_into().map_err(|_| "Compose local address exceeds u32")? });
        }
        Ok(())
    }

    /// Exposes one instruction directly to a renderer without constructing a retained node graph.
    pub fn view(&self, reference: &ToolPkgComposeDslReference) -> Result<ToolPkgComposeDslView, String> {
        let instance = self.instance(&reference.instanceId)?;
        let program = self.programs.get(&instance.programId).ok_or("Unknown Compose program")?;
        let instruction = program.nodes.get(reference.node as usize).ok_or("Unknown Compose instruction")?;
        let raw = self.operand(instance, &instruction.props)?;
        let mut content = self.operand(instance, &instruction.children)?;
        let mut result = ToolPkgComposeDslView { node_type: instruction.nodeType.clone(), props: BTreeMap::new(), children: Vec::new(), slots: BTreeMap::new() };
        if let Some(props) = raw.as_object() {
            for (key, value) in props {
                if key == "content" && (!instruction.explicitChildren || content.get("__composeUndefined") == Some(&Value::Bool(true))) { content = value.clone(); continue; }
                let mut children = Vec::new();
                Self::children(value, &mut children)?;
                if children.is_empty() { result.props.insert(key.clone(), value.clone()); }
                else { result.slots.insert(key.clone(), children); }
            }
        }
        Self::children(&content, &mut result.children)?;
        for patch in instance.patches.iter().filter(|patch| patch.node == reference.node) {
            match patch.field.as_str() {
                "type" => result.node_type = patch.value.as_str().ok_or("Compose type must be a string")?.into(),
                "props" => result.props = patch.value.as_object().ok_or("Compose props must be an object")?.clone().into_iter().collect(),
                "children" => { result.children.clear(); Self::children(&patch.value, &mut result.children)?; },
                "slots" => { result.slots.clear(); for (key, value) in patch.value.as_object().ok_or("Compose slots must be an object")? { let mut refs = Vec::new(); Self::children(value, &mut refs)?; result.slots.insert(key.clone(), refs); } },
                _ => return Err("Invalid Compose patch field".into()),
            }
        }
        Ok(result)
    }

    /// Resolves the current logical root after the first committed transaction.
    pub fn root(&self) -> Result<&ToolPkgComposeDslReference, String> { self.root.as_ref().ok_or_else(|| "Compose session has no root".into()) }

    /// Exports immutable programs and current parameters for a finite desktop widget host.
    pub fn snapshot(&self) -> Result<ToolPkgComposeDslDecodedTransaction, String> { Ok(ToolPkgComposeDslDecodedTransaction { reset: true, revision: self.revision, root: self.root()?.clone(), programs: self.programs.values().cloned().collect(), instances: self.instances.values().cloned().collect(), removed: Vec::new() }) }
}
