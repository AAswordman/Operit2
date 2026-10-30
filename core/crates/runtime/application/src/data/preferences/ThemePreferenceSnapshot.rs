use std::collections::BTreeMap;

use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Preferences, PreferencesDataStoreError,
};
use serde_json::Value;

/// Defines the complete wire value and ordinary preference encoding for one theme field.
#[derive(Clone, Copy)]
pub(crate) struct SnapshotField {
    pub(crate) json_key: &'static str,
    pub(crate) preference_key: &'static str,
    pub(crate) kind: FieldKind,
}

/// Describes a strict JSON field type and its allowed values.
#[derive(Clone, Copy)]
pub(crate) enum FieldKind {
    Boolean,
    OptionalColor,
    OptionalString,
    OptionalPalette,
    Enum(&'static [&'static str]),
    OptionalEnum(&'static [&'static str]),
    Number { min: f64, max: f64 },
}

// Keep this table exactly aligned with Flutter ThemePreferenceSnapshot.toJson.
// Pixel padding and avatar radius have no finite UI maximum, but must be finite and nonnegative.
pub(crate) const SNAPSHOT_FIELDS: &[SnapshotField] = &[
    SnapshotField {
        json_key: "themeMode",
        preference_key: "theme_mode",
        kind: FieldKind::Enum(&["system", "light", "dark"]),
    },
    SnapshotField {
        json_key: "useCustomColors",
        preference_key: "use_custom_colors",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "customPrimaryColor",
        preference_key: "custom_primary_color",
        kind: FieldKind::OptionalColor,
    },
    SnapshotField {
        json_key: "customSecondaryColor",
        preference_key: "custom_secondary_color",
        kind: FieldKind::OptionalColor,
    },
    SnapshotField {
        json_key: "useBackgroundImage",
        preference_key: "use_background_image",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "backgroundImageUri",
        preference_key: "background_image_uri",
        kind: FieldKind::OptionalString,
    },
    SnapshotField {
        json_key: "backgroundMediaType",
        preference_key: "background_media_type",
        kind: FieldKind::Enum(&["image", "video"]),
    },
    SnapshotField {
        json_key: "backgroundImageOpacity",
        preference_key: "background_image_opacity",
        kind: FieldKind::Number { min: 0.0, max: 1.0 },
    },
    SnapshotField {
        json_key: "videoBackgroundMuted",
        preference_key: "video_background_muted",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "videoBackgroundLoop",
        preference_key: "video_background_loop",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "useBackgroundBlur",
        preference_key: "use_background_blur",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "backgroundBlurRadius",
        preference_key: "background_blur_radius",
        kind: FieldKind::Number {
            min: 0.0,
            max: 40.0,
        },
    },
    SnapshotField {
        json_key: "transparentSurfaceEnabled",
        preference_key: "transparent_surface_enabled",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "chatInputFloating",
        preference_key: "chat_input_floating",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "inputStyle",
        preference_key: "input_style",
        kind: FieldKind::Enum(&["classic", "agent"]),
    },
    SnapshotField {
        json_key: "bubbleShowAvatar",
        preference_key: "bubble_show_avatar",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "bubbleWideLayoutEnabled",
        preference_key: "bubble_wide_layout_enabled",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "cursorUserBubbleColor",
        preference_key: "cursor_user_bubble_color",
        kind: FieldKind::OptionalColor,
    },
    SnapshotField {
        json_key: "bubbleUserBubbleColor",
        preference_key: "bubble_user_bubble_color",
        kind: FieldKind::OptionalColor,
    },
    SnapshotField {
        json_key: "bubbleAiBubbleColor",
        preference_key: "bubble_ai_bubble_color",
        kind: FieldKind::OptionalColor,
    },
    SnapshotField {
        json_key: "bubbleUserTextColor",
        preference_key: "bubble_user_text_color",
        kind: FieldKind::OptionalColor,
    },
    SnapshotField {
        json_key: "bubbleAiTextColor",
        preference_key: "bubble_ai_text_color",
        kind: FieldKind::OptionalColor,
    },
    SnapshotField {
        json_key: "bubbleUserUseImage",
        preference_key: "bubble_user_use_image",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "bubbleAiUseImage",
        preference_key: "bubble_ai_use_image",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "bubbleUserImageUri",
        preference_key: "bubble_user_image_uri",
        kind: FieldKind::OptionalString,
    },
    SnapshotField {
        json_key: "bubbleAiImageUri",
        preference_key: "bubble_ai_image_uri",
        kind: FieldKind::OptionalString,
    },
    SnapshotField {
        json_key: "bubbleUserImageRenderMode",
        preference_key: "bubble_user_image_render_mode",
        kind: FieldKind::Enum(&["tiled_nine_slice", "nine_patch"]),
    },
    SnapshotField {
        json_key: "bubbleAiImageRenderMode",
        preference_key: "bubble_ai_image_render_mode",
        kind: FieldKind::Enum(&["tiled_nine_slice", "nine_patch"]),
    },
    SnapshotField {
        json_key: "bubbleUserImageCropLeft",
        preference_key: "bubble_user_image_crop_left",
        kind: FieldKind::Number {
            min: 0.0,
            max: 0.45,
        },
    },
    SnapshotField {
        json_key: "bubbleUserImageCropTop",
        preference_key: "bubble_user_image_crop_top",
        kind: FieldKind::Number {
            min: 0.0,
            max: 0.45,
        },
    },
    SnapshotField {
        json_key: "bubbleUserImageCropRight",
        preference_key: "bubble_user_image_crop_right",
        kind: FieldKind::Number {
            min: 0.0,
            max: 0.45,
        },
    },
    SnapshotField {
        json_key: "bubbleUserImageCropBottom",
        preference_key: "bubble_user_image_crop_bottom",
        kind: FieldKind::Number {
            min: 0.0,
            max: 0.45,
        },
    },
    SnapshotField {
        json_key: "bubbleUserImageRepeatStart",
        preference_key: "bubble_user_image_repeat_start",
        kind: FieldKind::Number {
            min: 0.05,
            max: 0.9,
        },
    },
    SnapshotField {
        json_key: "bubbleUserImageRepeatEnd",
        preference_key: "bubble_user_image_repeat_end",
        kind: FieldKind::Number {
            min: 0.06,
            max: 0.95,
        },
    },
    SnapshotField {
        json_key: "bubbleUserImageRepeatYStart",
        preference_key: "bubble_user_image_repeat_y_start",
        kind: FieldKind::Number {
            min: 0.05,
            max: 0.9,
        },
    },
    SnapshotField {
        json_key: "bubbleUserImageRepeatYEnd",
        preference_key: "bubble_user_image_repeat_y_end",
        kind: FieldKind::Number {
            min: 0.06,
            max: 0.95,
        },
    },
    SnapshotField {
        json_key: "bubbleUserImageScale",
        preference_key: "bubble_user_image_scale",
        kind: FieldKind::Number { min: 0.2, max: 3.0 },
    },
    SnapshotField {
        json_key: "bubbleAiImageCropLeft",
        preference_key: "bubble_ai_image_crop_left",
        kind: FieldKind::Number {
            min: 0.0,
            max: 0.45,
        },
    },
    SnapshotField {
        json_key: "bubbleAiImageCropTop",
        preference_key: "bubble_ai_image_crop_top",
        kind: FieldKind::Number {
            min: 0.0,
            max: 0.45,
        },
    },
    SnapshotField {
        json_key: "bubbleAiImageCropRight",
        preference_key: "bubble_ai_image_crop_right",
        kind: FieldKind::Number {
            min: 0.0,
            max: 0.45,
        },
    },
    SnapshotField {
        json_key: "bubbleAiImageCropBottom",
        preference_key: "bubble_ai_image_crop_bottom",
        kind: FieldKind::Number {
            min: 0.0,
            max: 0.45,
        },
    },
    SnapshotField {
        json_key: "bubbleAiImageRepeatStart",
        preference_key: "bubble_ai_image_repeat_start",
        kind: FieldKind::Number {
            min: 0.05,
            max: 0.9,
        },
    },
    SnapshotField {
        json_key: "bubbleAiImageRepeatEnd",
        preference_key: "bubble_ai_image_repeat_end",
        kind: FieldKind::Number {
            min: 0.06,
            max: 0.95,
        },
    },
    SnapshotField {
        json_key: "bubbleAiImageRepeatYStart",
        preference_key: "bubble_ai_image_repeat_y_start",
        kind: FieldKind::Number {
            min: 0.05,
            max: 0.9,
        },
    },
    SnapshotField {
        json_key: "bubbleAiImageRepeatYEnd",
        preference_key: "bubble_ai_image_repeat_y_end",
        kind: FieldKind::Number {
            min: 0.06,
            max: 0.95,
        },
    },
    SnapshotField {
        json_key: "bubbleAiImageScale",
        preference_key: "bubble_ai_image_scale",
        kind: FieldKind::Number { min: 0.2, max: 3.0 },
    },
    SnapshotField {
        json_key: "bubbleUserRoundedCornersEnabled",
        preference_key: "bubble_rounded_corners_enabled",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "bubbleAiRoundedCornersEnabled",
        preference_key: "bubble_ai_rounded_corners_enabled",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "bubbleUserContentPaddingLeft",
        preference_key: "bubble_content_padding_left",
        kind: FieldKind::Number {
            min: 0.0,
            max: f64::MAX,
        },
    },
    SnapshotField {
        json_key: "bubbleUserContentPaddingRight",
        preference_key: "bubble_content_padding_right",
        kind: FieldKind::Number {
            min: 0.0,
            max: f64::MAX,
        },
    },
    SnapshotField {
        json_key: "bubbleAiContentPaddingLeft",
        preference_key: "bubble_ai_content_padding_left",
        kind: FieldKind::Number {
            min: 0.0,
            max: f64::MAX,
        },
    },
    SnapshotField {
        json_key: "bubbleAiContentPaddingRight",
        preference_key: "bubble_ai_content_padding_right",
        kind: FieldKind::Number {
            min: 0.0,
            max: f64::MAX,
        },
    },
    SnapshotField {
        json_key: "customUserAvatarUri",
        preference_key: "custom_user_avatar_uri",
        kind: FieldKind::OptionalString,
    },
    SnapshotField {
        json_key: "avatarShape",
        preference_key: "avatar_shape",
        kind: FieldKind::Enum(&["circle", "square"]),
    },
    SnapshotField {
        json_key: "avatarCornerRadius",
        preference_key: "avatar_corner_radius",
        kind: FieldKind::Number {
            min: 0.0,
            max: f64::MAX,
        },
    },
    SnapshotField {
        json_key: "useCustomFont",
        preference_key: "use_custom_font",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "fontType",
        preference_key: "font_type",
        kind: FieldKind::Enum(&["system", "file"]),
    },
    SnapshotField {
        json_key: "systemFontName",
        preference_key: "system_font_name",
        kind: FieldKind::OptionalEnum(&["default", "serif", "sans-serif", "monospace", "cursive"]),
    },
    SnapshotField {
        json_key: "customFontPath",
        preference_key: "custom_font_path",
        kind: FieldKind::OptionalString,
    },
    SnapshotField {
        json_key: "fontScale",
        preference_key: "font_scale",
        kind: FieldKind::Number {
            min: 0.85,
            max: 1.3,
        },
    },
    SnapshotField {
        json_key: "bubbleUserUseCustomFont",
        preference_key: "bubble_user_use_custom_font",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "bubbleUserFontType",
        preference_key: "bubble_user_font_type",
        kind: FieldKind::Enum(&["system", "file"]),
    },
    SnapshotField {
        json_key: "bubbleUserSystemFontName",
        preference_key: "bubble_user_system_font_name",
        kind: FieldKind::OptionalEnum(&["default", "serif", "sans-serif", "monospace", "cursive"]),
    },
    SnapshotField {
        json_key: "bubbleUserCustomFontPath",
        preference_key: "bubble_user_custom_font_path",
        kind: FieldKind::OptionalString,
    },
    SnapshotField {
        json_key: "bubbleAiUseCustomFont",
        preference_key: "bubble_ai_use_custom_font",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "bubbleAiFontType",
        preference_key: "bubble_ai_font_type",
        kind: FieldKind::Enum(&["system", "file"]),
    },
    SnapshotField {
        json_key: "bubbleAiSystemFontName",
        preference_key: "bubble_ai_system_font_name",
        kind: FieldKind::OptionalEnum(&["default", "serif", "sans-serif", "monospace", "cursive"]),
    },
    SnapshotField {
        json_key: "bubbleAiCustomFontPath",
        preference_key: "bubble_ai_custom_font_path",
        kind: FieldKind::OptionalString,
    },
    SnapshotField {
        json_key: "showThinkingProcess",
        preference_key: "show_thinking_process",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "showModelProvider",
        preference_key: "show_model_provider",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "showModelName",
        preference_key: "show_model_name",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "showRoleName",
        preference_key: "show_role_name",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "showUserName",
        preference_key: "show_user_name",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "showMessageTokenStats",
        preference_key: "show_message_token_stats",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "showMessageTimingStats",
        preference_key: "show_message_timing_stats",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "showMessageTimestamp",
        preference_key: "show_message_timestamp",
        kind: FieldKind::Boolean,
    },
    SnapshotField {
        json_key: "showInputProcessingStatus",
        preference_key: "show_input_processing_status",
        kind: FieldKind::Boolean,
    },
];

/// Input appearance fields added after the released saved-theme snapshot contract.
/// Older saved configurations do not own fields they never declared.
pub(crate) const EXTENDED_SNAPSHOT_FIELDS: &[SnapshotField] = &[
    SnapshotField { json_key: "themePaletteJson", preference_key: "theme_palette_json", kind: FieldKind::OptionalPalette },
    SnapshotField { json_key: "inputBackgroundColor", preference_key: "input_background_color", kind: FieldKind::OptionalColor },
    SnapshotField { json_key: "inputRoundedCornersEnabled", preference_key: "input_rounded_corners_enabled", kind: FieldKind::Boolean },
    SnapshotField { json_key: "inputUseImage", preference_key: "input_use_image", kind: FieldKind::Boolean },
    SnapshotField { json_key: "inputImageUri", preference_key: "input_image_uri", kind: FieldKind::OptionalString },
    SnapshotField { json_key: "inputImageRenderMode", preference_key: "input_image_render_mode", kind: FieldKind::Enum(&["tiled_nine_slice", "nine_patch"]) },
    SnapshotField { json_key: "inputImageCropLeft", preference_key: "input_image_crop_left", kind: FieldKind::Number { min: 0.0, max: f64::MAX } },
    SnapshotField { json_key: "inputImageCropTop", preference_key: "input_image_crop_top", kind: FieldKind::Number { min: 0.0, max: f64::MAX } },
    SnapshotField { json_key: "inputImageCropRight", preference_key: "input_image_crop_right", kind: FieldKind::Number { min: 0.0, max: f64::MAX } },
    SnapshotField { json_key: "inputImageCropBottom", preference_key: "input_image_crop_bottom", kind: FieldKind::Number { min: 0.0, max: f64::MAX } },
    SnapshotField { json_key: "inputImageRepeatStart", preference_key: "input_image_repeat_start", kind: FieldKind::Number { min: 0.0, max: 1.0 } },
    SnapshotField { json_key: "inputImageRepeatEnd", preference_key: "input_image_repeat_end", kind: FieldKind::Number { min: 0.0, max: 1.0 } },
    SnapshotField { json_key: "inputImageRepeatYStart", preference_key: "input_image_repeat_y_start", kind: FieldKind::Number { min: 0.0, max: 1.0 } },
    SnapshotField { json_key: "inputImageRepeatYEnd", preference_key: "input_image_repeat_y_end", kind: FieldKind::Number { min: 0.0, max: 1.0 } },
    SnapshotField { json_key: "inputImageScale", preference_key: "input_image_scale", kind: FieldKind::Number { min: 0.01, max: f64::MAX } },
    SnapshotField { json_key: "inputSettingsAsset", preference_key: "input_settings_asset", kind: FieldKind::OptionalString },
    SnapshotField { json_key: "inputSettingsAssetSize", preference_key: "input_settings_asset_size", kind: FieldKind::Number { min: 0.0, max: f64::MAX } },
    SnapshotField { json_key: "inputAttachmentAsset", preference_key: "input_attachment_asset", kind: FieldKind::OptionalString },
    SnapshotField { json_key: "inputAttachmentAssetSize", preference_key: "input_attachment_asset_size", kind: FieldKind::Number { min: 0.0, max: f64::MAX } },
    SnapshotField { json_key: "inputMicAsset", preference_key: "input_mic_asset", kind: FieldKind::OptionalString },
    SnapshotField { json_key: "inputMicAssetSize", preference_key: "input_mic_asset_size", kind: FieldKind::Number { min: 0.0, max: f64::MAX } },
    SnapshotField { json_key: "inputSendAsset", preference_key: "input_send_asset", kind: FieldKind::OptionalString },
    SnapshotField { json_key: "inputSendAssetSize", preference_key: "input_send_asset_size", kind: FieldKind::Number { min: 0.0, max: f64::MAX } },
    SnapshotField { json_key: "inputStopAsset", preference_key: "input_stop_asset", kind: FieldKind::OptionalString },
    SnapshotField { json_key: "inputStopAssetSize", preference_key: "input_stop_asset_size", kind: FieldKind::Number { min: 0.0, max: f64::MAX } },
    // Keep the released field valid; user schema 5 copies its value into the
    // per-role fields for persisted preferences and saved configurations.
    SnapshotField { json_key: "chatStyle", preference_key: "chat_style", kind: FieldKind::Enum(&["cursor", "bubble"]) },
    SnapshotField { json_key: "userChatStyle", preference_key: "user_chat_style", kind: FieldKind::Enum(&["cursor", "bubble"]) },
    SnapshotField { json_key: "aiChatStyle", preference_key: "ai_chat_style", kind: FieldKind::Enum(&["cursor", "bubble"]) },
];

/// Enumerates the declared ordinary appearance fields without inferring key names.
pub(crate) fn snapshotFields() -> impl Iterator<Item = &'static SnapshotField> {
    SNAPSHOT_FIELDS.iter().chain(EXTENDED_SNAPSHOT_FIELDS.iter())
}

/// Rejects missing, unknown, mistyped, out-of-range and inconsistent snapshot values.
pub fn validateThemePreferenceSnapshot(
    snapshot: &BTreeMap<String, Value>,
) -> Result<(), PreferencesDataStoreError> {
    for key in snapshot.keys() {
        if !snapshotFields().any(|field| field.json_key == key) {
            return Err(invalidSnapshot(key, "unknown field"));
        }
    }
    for field in SNAPSHOT_FIELDS {
        let value = snapshot
            .get(field.json_key)
            .ok_or_else(|| invalidSnapshot(field.json_key, "missing field"))?;
        validateField(field, value)?;
    }
    for field in EXTENDED_SNAPSHOT_FIELDS {
        if let Some(value) = snapshot.get(field.json_key) {
            validateField(field, value)?;
        }
    }
    for (start, end) in [
        ("bubbleUserImageRepeatStart", "bubbleUserImageRepeatEnd"),
        ("bubbleUserImageRepeatYStart", "bubbleUserImageRepeatYEnd"),
        ("bubbleAiImageRepeatStart", "bubbleAiImageRepeatEnd"),
        ("bubbleAiImageRepeatYStart", "bubbleAiImageRepeatYEnd"),
    ] {
        let start_value = requiredNumber(snapshot, start)?;
        let end_value = requiredNumber(snapshot, end)?;
        if end_value < start_value + 0.01 {
            return Err(invalidSnapshot(
                end,
                "repeat end must be at least 0.01 after its start",
            ));
        }
    }
    Ok(())
}

/// Reads ordinary fields using the declared initial values only for unset preference keys.
pub(crate) fn readThemePreferenceSnapshot(
    preferences: &Preferences,
    initialSnapshot: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, Value>, PreferencesDataStoreError> {
    validateThemePreferenceSnapshot(initialSnapshot)?;
    let mut snapshot = initialSnapshot.clone();
    for field in snapshotFields() {
        if let Some(text) = preferences.get(&stringPreferencesKey(field.preference_key)) {
            let value = text.clone();
            validateField(field, &value)?;
            snapshot.insert(field.json_key.to_string(), value);
        }
    }
    if let Some(text) = preferences.get(&stringPreferencesKey("use_system_theme")) {
        let enabled = serde_json::from_value::<bool>(text.clone())?;
        if enabled {
            snapshot.insert("themeMode".to_string(), Value::String("system".to_string()));
        }
    }
    validateThemePreferenceSnapshot(&snapshot)?;
    Ok(snapshot)
}

/// Converts released string scalars and signed ARGB integers through the current field schema.
pub(crate) fn decodeLegacyThemePreference(
    field: &SnapshotField,
    value: Value,
) -> Result<Value, PreferencesDataStoreError> {
    let mut decoded = match (field.kind, value) {
        (
            FieldKind::Boolean | FieldKind::OptionalColor | FieldKind::Number { .. },
            Value::String(encoded),
        ) => serde_json::from_str::<Value>(&encoded)?,
        (_, value) => value,
    };
    if matches!(field.kind, FieldKind::OptionalColor) && !decoded.is_null() {
        let number = decoded.as_i64().ok_or_else(|| {
            invalidSnapshot(
                field.json_key,
                &format!("invalid legacy ARGB value: {decoded}"),
            )
        })?;
        if number < i32::MIN as i64 || number > u32::MAX as i64 {
            return Err(invalidSnapshot(
                field.json_key,
                &format!("legacy ARGB value outside 32-bit range: {decoded}"),
            ));
        }
        decoded = Value::from(number as u32);
    }
    validateField(field, &decoded)?;
    Ok(decoded)
}

/// Converts ordinary appearance preferences before publishing the new user schema version.
pub(crate) fn migrateLegacyThemePreferences(
    preferences: &mut Preferences,
) -> Result<(), PreferencesDataStoreError> {
    for field in snapshotFields() {
        let key = stringPreferencesKey(field.preference_key);
        if let Some(value) = preferences.get(&key).cloned() {
            preferences.set(&key, decodeLegacyThemePreference(field, value)?);
        }
    }
    Ok(())
}

/// Converts historical snapshots at the migration boundary and removes fields the current schema no longer owns.
pub(crate) fn migrateLegacyThemeSnapshot(
    snapshot: &mut BTreeMap<String, Value>,
) -> Result<(), PreferencesDataStoreError> {
    snapshot.retain(|key, _| snapshotFields().any(|field| field.json_key == key));
    for field in snapshotFields() {
        if let Some(value) = snapshot.get_mut(field.json_key) {
            *value = decodeLegacyThemePreference(field, value.clone())?;
        }
    }
    validateThemePreferenceSnapshot(snapshot)
}

/// Collects exact cleanup inputs and converts only legacy settings supported by the current theme schema.
pub(crate) fn readLegacyCharacterThemeSnapshot(
    preferences: &Preferences,
    prefix: &str,
) -> Result<(BTreeMap<String, Value>, Vec<(String, Value)>), PreferencesDataStoreError> {
    let mut scoped = operit_store::PreferencesDataStore::emptyPreferences();
    let mut source_entries = Vec::new();
    for (key, value) in preferences.iterEntries() {
        if let Some(field) = key.strip_prefix(prefix) {
            source_entries.push((key.to_string(), value.clone()));
            // The released resolver reads the user avatar globally and does not apply scoped AI avatars.
            if matches!(field, "custom_user_avatar_uri" | "custom_ai_avatar_uri") {
                continue;
            }
            let descriptor = match field {
                "use_system_theme" => SnapshotField {
                    json_key: "useSystemTheme",
                    preference_key: "use_system_theme",
                    kind: FieldKind::Boolean,
                },
                _ => {
                    let Some(descriptor) = snapshotFields().find(|entry| entry.preference_key == field) else {
                        continue;
                    };
                    *descriptor
                }
            };
            let decoded = decodeLegacyThemePreference(&descriptor, value.clone())?;
            scoped.set(&stringPreferencesKey(field), decoded);
        }
    }
    if let Some(avatar) = preferences.get(&stringPreferencesKey("custom_user_avatar_uri")) {
        scoped.set(
            &stringPreferencesKey("custom_user_avatar_uri"),
            avatar.clone(),
        );
    }
    // These are the released resolver's declared initial fields, independent of current global appearance.
    let initial: BTreeMap<String, Value> = serde_json::from_str(
        r#"{
  "themeMode": "system",
  "useCustomColors": false,
  "customPrimaryColor": null,
  "customSecondaryColor": null,
  "useBackgroundImage": false,
  "backgroundImageUri": null,
  "backgroundMediaType": "image",
  "backgroundImageOpacity": 0.3,
  "videoBackgroundMuted": true,
  "videoBackgroundLoop": true,
  "useBackgroundBlur": false,
  "backgroundBlurRadius": 12,
  "transparentSurfaceEnabled": false,
  "chatInputFloating": false,
  "inputStyle": "agent",
  "chatStyle": "cursor",
  "bubbleShowAvatar": true,
  "bubbleWideLayoutEnabled": false,
  "cursorUserBubbleColor": null,
  "bubbleUserBubbleColor": null,
  "bubbleAiBubbleColor": null,
  "bubbleUserTextColor": null,
  "bubbleAiTextColor": null,
  "bubbleUserUseImage": false,
  "bubbleAiUseImage": false,
  "bubbleUserImageUri": null,
  "bubbleAiImageUri": null,
  "bubbleUserImageRenderMode": "tiled_nine_slice",
  "bubbleAiImageRenderMode": "tiled_nine_slice",
  "bubbleUserImageCropLeft": 0,
  "bubbleUserImageCropTop": 0,
  "bubbleUserImageCropRight": 0,
  "bubbleUserImageCropBottom": 0,
  "bubbleUserImageRepeatStart": 0.35,
  "bubbleUserImageRepeatEnd": 0.65,
  "bubbleUserImageRepeatYStart": 0.35,
  "bubbleUserImageRepeatYEnd": 0.65,
  "bubbleUserImageScale": 1,
  "bubbleAiImageCropLeft": 0,
  "bubbleAiImageCropTop": 0,
  "bubbleAiImageCropRight": 0,
  "bubbleAiImageCropBottom": 0,
  "bubbleAiImageRepeatStart": 0.35,
  "bubbleAiImageRepeatEnd": 0.65,
  "bubbleAiImageRepeatYStart": 0.35,
  "bubbleAiImageRepeatYEnd": 0.65,
  "bubbleAiImageScale": 1,
  "bubbleUserRoundedCornersEnabled": true,
  "bubbleAiRoundedCornersEnabled": true,
  "bubbleUserContentPaddingLeft": 12,
  "bubbleUserContentPaddingRight": 12,
  "bubbleAiContentPaddingLeft": 12,
  "bubbleAiContentPaddingRight": 12,
  "customUserAvatarUri": null,
  "avatarShape": "circle",
  "avatarCornerRadius": 8,
  "useCustomFont": false,
  "fontType": "system",
  "systemFontName": null,
  "customFontPath": null,
  "fontScale": 1,
  "bubbleUserUseCustomFont": false,
  "bubbleUserFontType": "system",
  "bubbleUserSystemFontName": "default",
  "bubbleUserCustomFontPath": null,
  "bubbleAiUseCustomFont": false,
  "bubbleAiFontType": "system",
  "bubbleAiSystemFontName": "default",
  "bubbleAiCustomFontPath": null,
  "showThinkingProcess": true,
  "showModelProvider": false,
  "showModelName": false,
  "showRoleName": true,
  "showUserName": true,
  "showMessageTokenStats": false,
  "showMessageTimingStats": false,
  "showMessageTimestamp": false,
  "showInputProcessingStatus": true
}"#,
    )?;
    Ok((
        readThemePreferenceSnapshot(&scoped, &initial)?,
        source_entries,
    ))
}

/// Validates exact ordinary appearance keys before staging a partial configuration edit.
pub(crate) fn validateThemePreferenceKeys<'a>(
    keys: impl Iterator<Item = &'a str>,
) -> Result<(), PreferencesDataStoreError> {
    for key in keys {
        if !snapshotFields().any(|field| field.preference_key == key)
        {
            return Err(invalidSnapshot(key, "unknown appearance preference"));
        }
    }
    Ok(())
}

/// Applies a validated complete snapshot using the existing ordinary appearance key names.
pub(crate) fn writeThemePreferenceSnapshot(
    preferences: &mut Preferences,
    snapshot: &BTreeMap<String, Value>,
) -> Result<(), PreferencesDataStoreError> {
    validateThemePreferenceSnapshot(snapshot)?;
    for field in snapshotFields() {
        let key = stringPreferencesKey(field.preference_key);
        let Some(value) = snapshot.get(field.json_key) else {
            // An older configuration declares none of the extended appearance values.
            preferences.remove(&key);
            continue;
        };
        match value {
            Value::Null => preferences.remove(&key),
            Value::String(value) => preferences.set(&key, value.clone()),
            Value::Bool(_) | Value::Number(_) => preferences.set(&key, value.clone()),
            _ => {
                return Err(invalidSnapshot(
                    field.json_key,
                    "unsupported preference value",
                ))
            }
        }
    }
    // The obsolete flag must not override the newly committed explicit theme_mode.
    preferences.remove(&stringPreferencesKey("use_system_theme"));
    Ok(())
}

/// Verifies that an active named configuration matches the committed ordinary appearance.
pub(crate) fn assertThemePreferenceSnapshot(
    preferences: &Preferences,
    snapshot: &BTreeMap<String, Value>,
) -> Result<(), PreferencesDataStoreError> {
    validateThemePreferenceSnapshot(snapshot)?;
    for field in snapshotFields() {
        let Some(expected) = snapshot.get(field.json_key) else {
            continue;
        };
        let key = stringPreferencesKey(field.preference_key);
        let actual = match (expected, preferences.get(&key)) {
            (Value::Null, None) => Value::Null,
            (Value::Null, Some(_)) => {
                return Err(invalidSnapshot(
                    field.json_key,
                    "active ordinary preference must be absent for a null snapshot value",
                ));
            }
            (_, Some(value)) => value.clone(),
            (_, None) => {
                return Err(invalidSnapshot(
                    field.json_key,
                    "active ordinary preference is missing",
                ))
            }
        };
        validateField(field, &actual)?;
        let equal = match field.kind {
            FieldKind::Number { .. } => expected.as_f64() == actual.as_f64(),
            _ => expected == &actual,
        };
        if !equal {
            return Err(invalidSnapshot(
                field.json_key,
                "active ordinary preference differs from its saved configuration",
            ));
        }
    }
    if preferences
        .get(&stringPreferencesKey("use_system_theme"))
        .is_some()
    {
        return Err(PreferencesDataStoreError::Message(
            "An active named theme must not retain use_system_theme".to_string(),
        ));
    }
    Ok(())
}

/// Validates one field against its exact JSON type and value range.
fn validateField(field: &SnapshotField, value: &Value) -> Result<(), PreferencesDataStoreError> {
    let valid = match field.kind {
        FieldKind::Boolean => value.is_boolean(),
        FieldKind::OptionalColor => {
            value.is_null() || value.as_u64().is_some_and(|color| color <= u32::MAX as u64)
        }
        FieldKind::OptionalString => value.is_null() || value.is_string(),
        FieldKind::OptionalPalette => value.is_null() || value.as_str().is_some_and(|text| {
            serde_json::from_str::<Value>(text).is_ok_and(|palette|
                operit_plugin_theme::validate_palette(&palette).is_ok())
        }),
        FieldKind::Enum(allowed) => value
            .as_str()
            .is_some_and(|text| allowed.iter().any(|candidate| *candidate == text)),
        FieldKind::OptionalEnum(allowed) => {
            value.is_null()
                || value
                    .as_str()
                    .is_some_and(|text| allowed.iter().any(|candidate| *candidate == text))
        }
        FieldKind::Number { min, max } => value
            .as_f64()
            .is_some_and(|number| number.is_finite() && number >= min && number <= max),
    };
    if !valid {
        return Err(invalidSnapshot(
            field.json_key,
            "invalid type, enum value or numeric range",
        ));
    }
    Ok(())
}

/// Reads a numeric field after schema validation without substituting a value.
fn requiredNumber(
    snapshot: &BTreeMap<String, Value>,
    field: &str,
) -> Result<f64, PreferencesDataStoreError> {
    snapshot
        .get(field)
        .and_then(Value::as_f64)
        .ok_or_else(|| invalidSnapshot(field, "expected number"))
}

/// Creates a precise validation error for the offending snapshot field.
fn invalidSnapshot(field: &str, reason: &str) -> PreferencesDataStoreError {
    PreferencesDataStoreError::Message(format!("Invalid theme snapshot field {field}: {reason}"))
}
