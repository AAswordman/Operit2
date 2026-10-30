//! An owner defines a format step once. Both state and transaction effects use the step.
use super::{Data, Edit, Owner, Target, Transaction};
use crate::PreferencesDataStore::{
    emptyPreferences, stringPreferencesKey, Preferences, PreferencesDataStoreError as Error,
};
use std::{collections::BTreeMap, sync::Arc};

/// Full source context is available for record splits, codecs, references and transaction expansion.
/// Identity and business order belong to the immutable original and cannot be reallocated here.
#[derive(Clone)]
pub struct Context {
    pub source: Owner,
    pub destination: Owner,
    pub before: Data,
    pub after: Data,
    pub original: Option<Transaction>,
    pub transactions: Vec<Transaction>,
    pub history: Vec<Transaction>,
    pub references: BTreeMap<String, String>,
    pub routes: BTreeMap<String, String>,
}

impl Context {
    pub fn state(data: Data) -> Self {
        Self {
            source: data.owner.clone(),
            destination: data.owner.clone(),
            before: data.clone(),
            after: data,
            original: None,
            transactions: vec![],
            history: vec![],
            references: BTreeMap::new(),
            routes: BTreeMap::new(),
        }
    }

    /// Compatibility for codecs that keep their logical keys and modification scopes.
    /// Structural changes use explicit Rules on this same context.
    pub fn map_preferences(
        &mut self,
        version: u32,
        convert: &dyn Fn(u32, &mut Preferences) -> Result<(), Error>,
    ) -> Result<(), Error> {
        fn project(
            data: &mut Data,
            version: u32,
            convert: &dyn Fn(u32, &mut Preferences) -> Result<(), Error>,
        ) -> Result<(), Error> {
            let mut preferences = emptyPreferences();
            for (key, value) in &data.values {
                preferences.set(&stringPreferencesKey(key), value.clone());
            }
            convert(version, &mut preferences)?;
            let values: BTreeMap<_, _> = preferences
                .iterEntries()
                .map(|(key, value)| (key.to_owned(), value.clone()))
                .collect();
            if (data.revision.is_some() || !data.versions.is_empty())
                && data.values.keys().any(|key| !values.contains_key(key))
            {
                return Err(Error::Message(
                    "Structural migration requires an explicit scope rule".into(),
                ));
            }
            for encoded in data.versions.keys().chain(data.additions.keys()) {
                let (key, path): (String, Vec<super::Path>) = serde_json::from_str(encoded)?;
                if data.values.get(&key) != values.get(&key)
                    && (!path.is_empty() || data.additions.contains_key(encoded))
                {
                    return Err(Error::Message(
                        "Scoped or additive migration requires an explicit scope rule".into(),
                    ));
                }
            }
            data.values = values;
            Ok(())
        }
        if self.original.is_none() {
            project(&mut self.after, version, convert)?;
            self.before = self.after.clone();
            return Ok(());
        }
        let mut source = self.before.clone();
        project(&mut self.before, version, convert)?;
        project(&mut self.after, version, convert)?;
        // Convert each explicit effect against its own intermediate state. A later
        // edit of the same key must not replace an earlier edit's payload.
        for transaction in &mut self.transactions {
            for edit in &mut transaction.edits {
                let effect = Transaction {
                    id: transaction.id.clone(),
                    revision: transaction.revision.clone(),
                    target: transaction.target.clone(),
                    edits: vec![edit.clone()],
                };
                source.apply(&effect).map_err(Error::Message)?;
                let mut converted = source.clone();
                project(&mut converted, version, convert)?;
                match edit {
                    Edit::Set { key, path, value } => {
                        *value = super::nested_value(converted.values.get(key), path)
                            .cloned()
                            .ok_or_else(|| {
                                Error::Message("Migration changed an edit scope".into())
                            })?;
                    }
                    Edit::Patch { key, set, .. } => {
                        for (field, value) in set {
                            *value = converted
                                .values
                                .get(key)
                                .and_then(|v| v.get(field))
                                .cloned()
                                .ok_or_else(|| {
                                    Error::Message("Migration changed a patch scope".into())
                                })?;
                        }
                    }
                    Edit::ReplaceRecords { prefix, records } => {
                        for (id, value) in records {
                            *value = converted
                                .values
                                .get(&format!("{prefix}{id}"))
                                .cloned()
                                .ok_or_else(|| {
                                    Error::Message("Migration changed a record identity".into())
                                })?;
                        }
                    }
                    Edit::Delete { .. } | Edit::Increment { .. } => {}
                }
            }
        }
        Ok(())
    }
}

type Step = dyn Fn(u32, &mut Context) -> Result<(), Error> + Send + Sync;
#[derive(Clone)]
pub struct Definition(Arc<Step>);
impl Definition {
    pub fn new(
        step: impl Fn(u32, &mut Context) -> Result<(), Error> + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(step))
    }
    pub fn from_preferences(
        step: impl Fn(u32, &mut Preferences) -> Result<(), Error> + Send + Sync + 'static,
    ) -> Self {
        Self::new(move |version, context| context.map_preferences(version, &step))
    }
    pub fn apply(&self, version: u32, context: &mut Context) -> Result<(), Error> {
        let mut staged = context.clone();
        (self.0)(version, &mut staged)?;
        if staged.before.revision != context.before.revision
            || staged.after.revision != context.after.revision
        {
            return Err(Error::Message(
                "Migration changed a business revision".into(),
            ));
        }
        if let Some(original) = &context.original {
            if staged.original != context.original
                || staged.transactions.len() != 1
                || staged.transactions[0].id != original.id
                || staged.transactions[0].revision != original.revision
            {
                return Err(Error::Message(
                    "Migration changed transaction identity, order or atomic boundary".into(),
                ));
            }
            staged.before.owner = staged.destination.clone();
            staged.after.owner = staged.destination.clone();
            staged.transactions[0].target = Target::Data(staged.destination.clone());
            // The two sides must use the same state function m, even when the
            // transaction converter branches on its original or history.
            for (source, converted) in [
                (&context.before, &staged.before),
                (&context.after, &staged.after),
            ] {
                let mut state = context.clone();
                state.before = source.clone();
                state.after = source.clone();
                state.original = None;
                state.transactions.clear();
                (self.0)(version, &mut state)?;
                state.after.owner = staged.destination.clone();
                if state.after != *converted {
                    return Err(Error::Message(
                        "Migration violates the shared state function m".into(),
                    ));
                }
            }
            let mut applied = staged.before.clone();
            applied
                .apply(&staged.transactions[0])
                .map_err(Error::Message)?;
            if applied != staged.after {
                return Err(Error::Message(
                    "Migration violates m(Apply(A,T)) = Apply(m(A),m_A(T))".into(),
                ));
            }
        }
        *context = staged;
        Ok(())
    }
    pub fn state(&self, version: u32, preferences: &mut Preferences) -> Result<(), Error> {
        let owner = Owner {
            backend: super::Backend::DataStore,
            path: String::new(),
            format: version,
            layout: 1,
        };
        let mut data = preferences
            .view
            .clone()
            .unwrap_or_else(|| Data::empty(owner));
        data.values = preferences
            .iterEntries()
            .map(|(key, value)| (key.to_owned(), value.clone()))
            .collect();
        let mut context = Context::state(data);
        self.apply(version, &mut context)?;
        let mut migrated = emptyPreferences();
        for (key, value) in &context.after.values {
            migrated.set(&stringPreferencesKey(key), value.clone());
        }
        preferences.values = migrated.values;
        preferences.view = Some(context.after);
        Ok(())
    }
}

impl super::Form for Context {
    fn migrate(&mut self, migrations: &super::Migrations) -> super::Result<()> {
        migrations.migrate_data(&mut self.before)?;
        migrations.migrate_data(&mut self.after)?;
        for transaction in &mut self.transactions {
            migrations.migrate_transaction(transaction)?;
        }
        self.destination = self.after.owner.clone();
        Ok(())
    }
    fn apply_rule(&mut self, rule: &super::Rule) -> super::Result<()> {
        rule.apply(&mut self.before)?;
        rule.apply(&mut self.after)?;
        for transaction in &mut self.transactions {
            rule.apply(transaction)?;
        }
        Ok(())
    }
}
