use std::collections::{BTreeMap, BTreeSet};

use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;
use serde::{Deserialize, Serialize};

/// Identifies one enabled package that actually registered the requested public API.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(non_snake_case)]
pub struct ToolPkgPublicApiOwner {
    pub containerPackageName: String,
    pub apiName: String,
}

/// Reads an exact published method without executing its handler or inventing a UI contribution.
fn registered_owner(runtime: &ToolPkgContainerRuntime, api_name: &str) -> Result<bool, String> {
    let declarations = runtime
        .publicApis
        .iter()
        .filter(
            /* Selects only the exact published method name, never a substring or domain prefix. */
            |declaration| declaration.id == api_name,
        )
        .collect::<Vec<_>>();
    let declaration = match declarations.as_slice() {
        [] => return Ok(false),
        [declaration] => declaration,
        _ => {
            return Err(format!(
                "Duplicate public API: {}/{api_name}",
                runtime.packageName
            ))
        }
    };
    if declaration.function.trim().is_empty() {
        return Err(format!(
            "Public API handler is empty: {}/{api_name}",
            runtime.packageName
        ));
    }
    if runtime.publicApi.is_none() {
        return Err(format!(
            "{} does not publish public_api",
            runtime.packageName
        ));
    }
    if runtime.packageName.trim().is_empty() || runtime.packageName.trim() != runtime.packageName {
        return Err("Public API owner package name is invalid".to_string());
    }
    Ok(true)
}

/// Enumerates enabled publications; an empty match set is valid, while malformed catalogs remain errors.
pub(super) fn discover_public_api_owners(
    api_name: &str,
    registered: &[ToolPkgContainerRuntime],
    enabled: &[ToolPkgContainerRuntime],
) -> Result<Vec<ToolPkgPublicApiOwner>, String> {
    if api_name.trim().is_empty() {
        return Err("Public API method name is required".to_string());
    }
    if api_name.trim() != api_name {
        return Err("Public API method name must match its exact registered name".to_string());
    }
    let mut declared = BTreeMap::new();
    for runtime in registered {
        if !registered_owner(runtime, api_name)? {
            continue;
        }
        if declared
            .insert(runtime.packageName.clone(), runtime)
            .is_some()
        {
            return Err(format!(
                "Duplicate public API owner: {}/{api_name}",
                runtime.packageName
            ));
        }
    }
    let mut owners = Vec::new();
    let mut selected = BTreeSet::new();
    for runtime in enabled {
        if !registered_owner(runtime, api_name)? {
            continue;
        }
        if !declared.contains_key(&runtime.packageName) {
            return Err(format!(
                "Enabled public API owner is absent from the registered catalog: {}/{api_name}",
                runtime.packageName
            ));
        }
        if !selected.insert(runtime.packageName.clone()) {
            return Err(format!(
                "Duplicate enabled public API owner: {}/{api_name}",
                runtime.packageName
            ));
        }
        owners.push(ToolPkgPublicApiOwner {
            containerPackageName: runtime.packageName.clone(),
            apiName: api_name.to_string(),
        });
    }
    Ok(owners)
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgRegisteredFunctionHook;

    /// Creates only pure catalog metadata using the SDK's actual registration record type.
    fn runtime(package: &str, api: &str) -> ToolPkgContainerRuntime {
        ToolPkgContainerRuntime {
            packageName: package.to_string(),
            publicApi: Some("src/api.ts".to_string()),
            publicApis: vec![ToolPkgRegisteredFunctionHook {
                id: api.to_string(),
                function: "publishedCallback".to_string(),
                functionSource: None,
            }],
            ..Default::default()
        }
    }

    /// Preserves the enabled catalog order while allowing independent packages to publish the same generic method.
    #[test]
    fn returns_only_enabled_owners_in_catalog_order() {
        let first = runtime("first", "chat.context.actions");
        let second = runtime("second", "chat.context.actions");
        let disabled = runtime("disabled", "chat.context.actions");
        let owners = discover_public_api_owners(
            "chat.context.actions",
            &[first.clone(), second.clone(), disabled],
            &[second, first],
        )
        .unwrap();
        assert_eq!(
            owners,
            vec![
                ToolPkgPublicApiOwner {
                    containerPackageName: "second".to_string(),
                    apiName: "chat.context.actions".to_string()
                },
                ToolPkgPublicApiOwner {
                    containerPackageName: "first".to_string(),
                    apiName: "chat.context.actions".to_string()
                }
            ]
        );
    }

    /// Supports arbitrary exact registerApi names rather than a character, memory, or UI API whitelist.
    #[test]
    fn uses_exact_generic_registration_names() {
        let owner = runtime("sample", "arbitrary.extension.method");
        assert_eq!(
            discover_public_api_owners(
                "arbitrary.extension.method",
                &[owner.clone()],
                &[owner.clone()]
            )
            .unwrap()
            .len(),
            1
        );
        for name in ["Arbitrary.extension.method", "missing"] {
            assert!(discover_public_api_owners(name, &[owner.clone()], &[owner.clone()]).unwrap().is_empty());
        }
        for name in ["", " arbitrary.extension.method"] {
            assert!(discover_public_api_owners(name, &[owner.clone()], &[owner.clone()]).is_err());
        }
    }

    /// Allows the chat shell to discover zero optional contributions without inventing an owner.
    #[test]
    fn enumerates_empty_uninstalled_and_disabled_contributions() {
        let owner = runtime("disabled", "chat.context.actions");
        assert!(discover_public_api_owners("chat.context.actions", &[owner], &[]).unwrap().is_empty());
        assert!(discover_public_api_owners("chat.context.actions", &[], &[]).unwrap().is_empty());
        let legacy = ToolPkgContainerRuntime { packageName: "legacy".into(), ..Default::default() };
        assert!(discover_public_api_owners("chat.context.actions", &[legacy.clone()], &[legacy]).unwrap().is_empty());
    }

    /// Rejects two methods with the same exact name before either handler can be selected.
    #[test]
    fn rejects_duplicate_exports() {
        let mut owner = runtime("sample", "chat.list.sections");
        owner.publicApis.push(owner.publicApis[0].clone());
        assert_eq!(
            discover_public_api_owners("chat.list.sections", &[owner.clone()], &[owner])
                .unwrap_err(),
            "Duplicate public API: sample/chat.list.sections"
        );
    }

    /// Rejects repeated package owners even when the underlying catalog has malformed duplicate rows.
    #[test]
    fn rejects_duplicate_owner_rows() {
        let owner = runtime("sample", "chat.list.sections");
        assert!(discover_public_api_owners(
            "chat.list.sections",
            &[owner.clone(), owner.clone()],
            &[owner.clone()]
        )
        .unwrap_err()
        .starts_with("Duplicate public API owner:"));
        assert!(discover_public_api_owners(
            "chat.list.sections",
            &[owner.clone()],
            &[owner.clone(), owner]
        )
        .unwrap_err()
        .starts_with("Duplicate enabled public API owner:"));
    }

    /// Propagates invalid registration metadata instead of listing an owner whose executor will reject it.
    #[test]
    fn rejects_unpublished_and_empty_handlers() {
        let mut owner = runtime("sample", "chat.context.actions");
        owner.publicApi = None;
        assert_eq!(
            discover_public_api_owners("chat.context.actions", &[owner.clone()], &[owner.clone()])
                .unwrap_err(),
            "sample does not publish public_api"
        );
        owner.publicApi = Some("src/api.ts".to_string());
        owner.publicApis[0].function = " ".to_string();
        assert_eq!(
            discover_public_api_owners("chat.context.actions", &[owner.clone()], &[owner])
                .unwrap_err(),
            "Public API handler is empty: sample/chat.context.actions"
        );
    }

    /// Rejects an enabled snapshot that claims an owner absent from the authoritative registered catalog.
    #[test]
    fn rejects_inconsistent_catalog_owner() {
        let registered = runtime("registered", "chat.context.actions");
        let enabled = runtime("unregistered", "chat.context.actions");
        assert_eq!(discover_public_api_owners("chat.context.actions", &[registered], &[enabled]).unwrap_err(), "Enabled public API owner is absent from the registered catalog: unregistered/chat.context.actions");
    }
}
