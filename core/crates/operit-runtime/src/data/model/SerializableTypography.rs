use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SerializableTypography {
    pub displayLarge: SerializableTextStyle,
    pub displayMedium: SerializableTextStyle,
    pub displaySmall: SerializableTextStyle,
    pub headlineLarge: SerializableTextStyle,
    pub headlineMedium: SerializableTextStyle,
    pub headlineSmall: SerializableTextStyle,
    pub titleLarge: SerializableTextStyle,
    pub titleMedium: SerializableTextStyle,
    pub titleSmall: SerializableTextStyle,
    pub bodyLarge: SerializableTextStyle,
    pub bodyMedium: SerializableTextStyle,
    pub bodySmall: SerializableTextStyle,
    pub labelLarge: SerializableTextStyle,
    pub labelMedium: SerializableTextStyle,
    pub labelSmall: SerializableTextStyle,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SerializableTextStyle {
    pub fontSize: f32,
    pub lineHeight: f32,
    pub letterSpacing: f32,
    pub fontWeight: i32,
}
