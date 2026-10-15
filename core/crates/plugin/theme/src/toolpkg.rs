//! Theme template contributions declared by the ordinary ToolPkg manifest.

use crate::{
    ThemeTemplateMetadata, ThemeValidationError, ValidatedThemeTemplate, MAX_MANIFEST_BYTES,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};

/// Declares one theme template using package-relative definition and preview paths.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeTemplateDeclaration {
    pub id: String,
    pub path: String,
    pub preview: String,
    pub files: Vec<crate::ManifestFile>,
}

/// Encodes a normal ToolPkg manifest containing one authoring template.
pub fn toolpkg_manifest_value(metadata: &ThemeTemplateMetadata) -> Value {
    let mut value = json!({
        "schema_version": 1, "api_version": "2.0.0",
        "toolpkg_id": metadata.package_id, "version": metadata.version,
        "display_name": metadata.display_name, "author": [metadata.author.name],
        "themes": [{"id": metadata.template_id, "path": metadata.path,
            "preview": metadata.preview, "files": metadata.files}],
    });
    if let Some(license) = &metadata.license {
        value["license"] = json!(license);
    }
    if let Some(description) = &metadata.description {
        value["description"] = json!(description);
    }
    value
}

/// Reads a referenced ZIP resource within its declared byte budget.
fn read_resource(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    path: &str,
    limit: usize,
) -> Result<Vec<u8>, ThemeValidationError> {
    let mut file = archive
        .by_name(path)
        .map_err(|error| ThemeValidationError::contract(path, error.to_string()))?;
    if file.is_dir() || file.is_symlink() || file.encrypted() || file.size() > limit as u64 {
        return Err(ThemeValidationError::contract(
            path,
            "referenced resource is not a bounded regular file",
        ));
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| ThemeValidationError::contract(path, error.to_string()))?;
    if bytes.len() > limit {
        return Err(ThemeValidationError::contract(
            path,
            "resource byte budget exceeded",
        ));
    }
    Ok(bytes)
}

/// Resolves a safe package-relative path without modifying its spelling.
fn package_path(base: &str, path: &str) -> Result<String, ThemeValidationError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains(['\\', ':'])
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(ThemeValidationError::contract(
            path,
            "resource must use a safe package-relative path",
        ));
    }
    Ok(format!("{base}{path}"))
}

/// Reads a package metadata string without imposing theme-specific package requirements.
fn metadata_text(value: Option<&Value>, path: &str) -> Result<String, ThemeValidationError> {
    match value {
        None => Ok(String::new()),
        Some(Value::String(text)) => Ok(text.clone()),
        Some(Value::Object(texts)) => {
            let mut values = Vec::new();
            let entries = match texts.get("values") {
                Some(Value::Object(entries)) => entries,
                Some(_) => {
                    return Err(ThemeValidationError::contract(
                        path,
                        "localized values must be an object",
                    ))
                }
                None => texts,
            };
            for text in entries.values() {
                values.push(text.as_str().ok_or_else(|| {
                    ThemeValidationError::contract(path, "localized metadata must contain text")
                })?);
            }
            Ok(values.join(" / "))
        }
        Some(_) => Err(ThemeValidationError::contract(
            path,
            "metadata must contain text",
        )),
    }
}

/// Checks declared template resources after the shared ToolPkg parser has resolved its manifest.
pub fn validate_declared_theme_templates(
    bytes: &[u8],
    manifest_path: &str,
    manifest_bytes: &[u8],
    value: &Value,
) -> Result<Vec<ValidatedThemeTemplate>, ThemeValidationError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| ThemeValidationError::contract(manifest_path, error.to_string()))?;
    let declarations = match value.get("themes") {
        None => return Ok(Vec::new()),
        Some(themes) => {
            let definitions = themes.as_array().ok_or_else(|| {
                ThemeValidationError::contract("manifest/themes", "templates must be an array")
            })?;
            let schema: Value = serde_json::from_str(crate::THEME_TEMPLATE_SCHEMA)
                .expect("trusted template schema");
            for definition in definitions {
                crate::schema::validate(&schema, &schema, definition, "manifest/themes", true)?;
            }
            serde_json::from_value::<Vec<ThemeTemplateDeclaration>>(themes.clone()).map_err(
                |error| ThemeValidationError::contract("manifest/themes", error.to_string()),
            )?
        }
    };
    let package_id = value["toolpkg_id"].as_str().ok_or_else(|| {
        ThemeValidationError::contract("manifest/toolpkg_id", "package id must be text")
    })?;
    let base = match manifest_path.rsplit_once('/') {
        Some((base, _)) => format!("{base}/"),
        None => String::new(),
    };
    let mut ids = BTreeSet::new();
    let mut templates = Vec::new();
    for declaration in declarations {
        if declaration.id.is_empty()
            || declaration.id.contains(':')
            || !ids.insert(declaration.id.clone())
        {
            return Err(ThemeValidationError::contract(
                "manifest/themes/id",
                "template ids must be nonempty, unique and contain no colon",
            ));
        }
        let theme_path = package_path(&base, &declaration.path)?;
        let preview_path = package_path(&base, &declaration.preview)?;
        let resource_base = match theme_path.rsplit_once('/') {
            Some((base, _)) => format!("{base}/"),
            None => String::new(),
        };
        let mut files = BTreeMap::new();
        for resource in &declaration.files {
            crate::validate_theme_resource_path(&resource.path)?;
            let (_, limit) = crate::metadata_for_path(&resource.path)?;
            let path = match resource.path.as_str() {
                "theme.json" => theme_path.clone(),
                "preview.png" => preview_path.clone(),
                _ => package_path(&resource_base, &resource.path)?,
            };
            files.insert(
                resource.path.clone(),
                read_resource(&mut archive, &path, limit)?,
            );
        }
        let authors = match value.get("author") {
            None => String::new(),
            Some(Value::String(author)) => author.clone(),
            Some(Value::Array(authors)) => authors
                .iter()
                .map(|author| {
                    author.as_str().ok_or_else(|| {
                        ThemeValidationError::contract(
                            "manifest/author",
                            "author must contain text",
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
                .join(", "),
            Some(_) => {
                return Err(ThemeValidationError::contract(
                    "manifest/author",
                    "author must contain text",
                ))
            }
        };
        let license = match value.get("license") {
            None => None,
            Some(Value::String(path)) => Some(path.clone()),
            Some(_) => {
                return Err(ThemeValidationError::contract(
                    "manifest/license",
                    "agreement reference must be text",
                ))
            }
        };
        let metadata = ThemeTemplateMetadata {
            theme_id: format!("{package_id}:{}", declaration.id),
            package_id: package_id.to_owned(),
            template_id: declaration.id,
            path: declaration.path,
            preview: declaration.preview,
            version: metadata_text(value.get("version"), "manifest/version")?,
            display_name: metadata_text(value.get("display_name"), "manifest/display_name")?,
            author: crate::Author { name: authors },
            license,
            files: declaration.files,
            description: match value.get("description") {
                None => None,
                Some(text) => Some(metadata_text(Some(text), "manifest/description")?),
            },
        };
        templates.push(crate::validate_template(
            bytes,
            metadata,
            manifest_bytes,
            files,
        )?);
    }
    Ok(templates)
}

/// Checks Studio's explicit root-manifest ToolPkg output before returning authored bytes.
pub fn validate_toolpkg_theme_templates(
    bytes: &[u8],
) -> Result<Vec<ValidatedThemeTemplate>, ThemeValidationError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| ThemeValidationError::contract("manifest.json", error.to_string()))?;
    let manifest_bytes = read_resource(&mut archive, "manifest.json", MAX_MANIFEST_BYTES)?;
    let value = crate::json::parse(&manifest_bytes, MAX_MANIFEST_BYTES, "manifest.json")?;
    validate_declared_theme_templates(bytes, "manifest.json", &manifest_bytes, &value)
}

/// Resolves an explicitly selected template by its package-qualified identity.
pub fn validate_toolpkg_theme_template(
    bytes: &[u8],
    theme_id: &str,
) -> Result<ValidatedThemeTemplate, ThemeValidationError> {
    validate_toolpkg_theme_templates(bytes)?
        .into_iter()
        .find(|template| template.metadata().theme_id == theme_id)
        .ok_or_else(|| {
            ThemeValidationError::contract(
                "manifest/themes",
                "selected theme template is not declared",
            )
        })
}

/// Validates every template in Studio's final archive using its explicit rewritten manifest path.
pub fn validate_authored_toolpkg_theme_templates(
    bytes: &[u8], manifest_path: &str,
) -> Result<Vec<ValidatedThemeTemplate>, ThemeValidationError> {
    if bytes.len() > crate::MAX_ARCHIVE_BYTES {
        return Err(ThemeValidationError::contract(manifest_path, "archive byte budget exceeded"));
    }
    package_path("", manifest_path)?;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| ThemeValidationError::contract(manifest_path, error.to_string()))?;
    let manifest_bytes = read_resource(&mut archive, manifest_path, MAX_MANIFEST_BYTES)?;
    let value = crate::json::parse(&manifest_bytes, MAX_MANIFEST_BYTES, manifest_path)?;
    validate_declared_theme_templates(bytes, manifest_path, &manifest_bytes, &value)
}
