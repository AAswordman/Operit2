use std::collections::BTreeMap;

use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Preferences, PreferencesDataStoreError,
};
use serde_json::Value;

/// Defines the complete wire value and ordinary preference encoding for one theme field.
#[derive(Clone, Copy)]
struct SnapshotField {
    json_key: &'static str,
    preference_key: &'static str,
    kind: FieldKind,
}

/// Describes a strict JSON field type and its allowed values.
#[derive(Clone, Copy)]
enum FieldKind {
    Boolean,
    OptionalColor,
    OptionalString,
    Enum(&'static [&'static str]),
    OptionalEnum(&'static [&'static str]),
    Number { min: f64, max: f64 },
}

// Keep this table exactly aligned with Flutter ThemePreferenceSnapshot.toJson.
// Pixel padding and avatar radius have no finite UI maximum, but must be finite and nonnegative.
const SNAPSHOT_FIELDS: &[SnapshotField] = &[
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
        json_key: "chatStyle",
        preference_key: "chat_style",
        kind: FieldKind::Enum(&["cursor", "bubble"]),
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

/// Rejects missing, unknown, mistyped, out-of-range and inconsistent snapshot values.
pub fn validateThemePreferenceSnapshot(
    snapshot: &BTreeMap<String, Value>,
) -> Result<(), PreferencesDataStoreError> {
    for key in snapshot.keys() {
        if !SNAPSHOT_FIELDS.iter().any(|field| field.json_key == key) {
            return Err(invalidSnapshot(key, "unknown field"));
        }
    }
    for field in SNAPSHOT_FIELDS {
        let value = snapshot
            .get(field.json_key)
            .ok_or_else(|| invalidSnapshot(field.json_key, "missing field"))?;
        validateField(field, value)?;
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

/// Applies a validated complete snapshot using the existing ordinary appearance key names.
pub(crate) fn writeThemePreferenceSnapshot(
    preferences: &mut Preferences,
    snapshot: &BTreeMap<String, Value>,
) -> Result<(), PreferencesDataStoreError> {
    validateThemePreferenceSnapshot(snapshot)?;
    for field in SNAPSHOT_FIELDS {
        let value = snapshot
            .get(field.json_key)
            .ok_or_else(|| invalidSnapshot(field.json_key, "missing field"))?;
        let key = stringPreferencesKey(field.preference_key);
        match value {
            Value::Null => preferences.remove(&key),
            Value::String(value) => preferences.set(&key, value.clone()),
            Value::Bool(_) | Value::Number(_) => preferences.set(&key, value.to_string()),
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
    for field in SNAPSHOT_FIELDS {
        let expected = snapshot
            .get(field.json_key)
            .ok_or_else(|| invalidSnapshot(field.json_key, "missing field"))?;
        let key = stringPreferencesKey(field.preference_key);
        let actual = match (expected, preferences.get(&key)) {
            (Value::Null, None) => Value::Null,
            (Value::Null, Some(_)) => {
                return Err(invalidSnapshot(
                    field.json_key,
                    "active ordinary preference must be absent for a null snapshot value",
                ));
            }
            (Value::String(_), Some(text)) => Value::String(text.clone()),
            (_, Some(text)) => serde_json::from_str::<Value>(text)?,
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
