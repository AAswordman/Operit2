use std::fmt;

/// Identifies the first ToolPkg API contract supported by Operit2.
pub const TOOLPKG_API_VERSION_2_0_0: &str = "2.0.0";

/// Identifies the ToolPkg API contract exposed by this SDK.
pub const CURRENT_TOOLPKG_API_VERSION: &str = TOOLPKG_API_VERSION_2_0_0;

/// Represents one parsed ToolPkg API semantic version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToolPkgApiVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ToolPkgApiVersion {
    /// Parses a major.minor.patch ToolPkg API version string.
    pub fn parse(value: &str) -> Result<Self, String> {
        let normalized = value.trim();
        let parts = normalized.split('.').collect::<Vec<_>>();
        if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
            return Err(format!(
                "ToolPkg API version must use major.minor.patch format: '{value}'"
            ));
        }
        let major = parseVersionPart(parts[0], value)?;
        let minor = parseVersionPart(parts[1], value)?;
        let patch = parseVersionPart(parts[2], value)?;
        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for ToolPkgApiVersion {
    /// Formats the ToolPkg API version as major.minor.patch.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Parses one decimal version component.
#[allow(non_snake_case)]
fn parseVersionPart(part: &str, source: &str) -> Result<u32, String> {
    if !part.chars().all(|character| character.is_ascii_digit()) {
        return Err(format!(
            "ToolPkg API version must use major.minor.patch format: '{source}'"
        ));
    }
    part.parse::<u32>()
        .map_err(|_| format!("ToolPkg API version contains an out-of-range component: '{source}'"))
}

/// Returns the ToolPkg API version text used for manifest defaults.
#[allow(non_snake_case)]
pub fn currentToolPkgApiVersionText() -> String {
    CURRENT_TOOLPKG_API_VERSION.to_string()
}

/// Lists the ToolPkg API versions accepted by this runtime.
#[allow(non_snake_case)]
pub fn supportedToolPkgApiVersions() -> Vec<ToolPkgApiVersion> {
    vec![ToolPkgApiVersion::parse(TOOLPKG_API_VERSION_2_0_0)
        .expect("ToolPkg API 2.0.0 constant must parse")]
}

/// Parses and validates a declared ToolPkg API version.
#[allow(non_snake_case)]
pub fn requireSupportedToolPkgApiVersion(value: &str) -> Result<ToolPkgApiVersion, String> {
    let declared = ToolPkgApiVersion::parse(value)?;
    if supportedToolPkgApiVersions().contains(&declared) {
        return Ok(declared);
    }
    Err(format!(
        "ToolPkg API version '{}' is not supported. Supported ToolPkg API versions: {}.",
        declared,
        supportedToolPkgApiVersions()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies ToolPkg API version parsing uses exact semantic components.
    #[test]
    fn parses_toolpkg_api_version() {
        assert_eq!(
            ToolPkgApiVersion::parse("2.0.0").unwrap(),
            ToolPkgApiVersion {
                major: 2,
                minor: 0,
                patch: 0,
            }
        );
    }

    /// Verifies the supported ToolPkg API set starts at the Operit2 contract.
    #[test]
    fn supports_operit2_toolpkg_api_version() {
        assert_eq!(
            requireSupportedToolPkgApiVersion("2.0.0")
                .unwrap()
                .to_string(),
            "2.0.0"
        );
        assert!(requireSupportedToolPkgApiVersion("1.0.1").is_err());
    }
}
