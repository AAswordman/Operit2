//! Material 3 and Compose Foundation node properties exposed by the plugin UI DSL.
use super::compose_dsl::*;
use super::{JsDate, JsFuture};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
/// Completion returned by the row click handler, either immediately or asynchronously.
pub enum ComposeGeneratedRowPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Content accepted by a text field label: either plain text or composed child nodes.
pub enum ComposeGeneratedTextFieldPropsLabel {
    Variant1(String),
    Variant2(ComposeChildren),
}
/// Content accepted by a text field placeholder: either plain text or composed child nodes.
pub enum ComposeGeneratedTextFieldPropsPlaceholder {
    Variant1(String),
    Variant2(ComposeChildren),
}
/// Completion returned by the button click handler, either immediately or asynchronously.
pub enum ComposeGeneratedButtonPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the icon button click handler, either immediately or asynchronously.
pub enum ComposeGeneratedIconButtonPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the surface click handler, either immediately or asynchronously.
pub enum ComposeGeneratedSurfacePropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the assist chip click handler, either immediately or asynchronously.
pub enum ComposeGeneratedAssistChipPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the dropdown menu dismissal request handler, either immediately or asynchronously.
pub enum ComposeGeneratedDropdownMenuPropsOnDismissRequestOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Popup-window behavior controlling dropdown focus, dismissal, clipping, and platform sizing.
pub struct ComposeGeneratedDropdownMenuPropsProperties {
    /// Allows the popup window to receive focus and keyboard input.
    pub focusable: Option<bool>,
    /// Dismisses the popup when the platform back action is invoked.
    pub dismissOnBackPress: Option<bool>,
    /// Dismisses the popup when a pointer press occurs outside its bounds.
    pub dismissOnClickOutside: Option<bool>,
    /// Constrains the popup window to the visible screen bounds when true.
    pub clippingEnabled: Option<bool>,
    /// Uses the platform's default popup width constraints when true.
    pub usePlatformDefaultWidth: Option<bool>,
}
/// Completion returned by the elevated assist chip click handler, either immediately or asynchronously.
pub enum ComposeGeneratedElevatedAssistChipPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the elevated button click handler, either immediately or asynchronously.
pub enum ComposeGeneratedElevatedButtonPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the elevated filter chip click handler, either immediately or asynchronously.
pub enum ComposeGeneratedElevatedFilterChipPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the elevated suggestion chip click handler, either immediately or asynchronously.
pub enum ComposeGeneratedElevatedSuggestionChipPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the extended floating action button click handler, either immediately or asynchronously.
pub enum ComposeGeneratedExtendedFloatingActionButtonPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the filled icon button click handler, either immediately or asynchronously.
pub enum ComposeGeneratedFilledIconButtonPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the filled tonal button click handler, either immediately or asynchronously.
pub enum ComposeGeneratedFilledTonalButtonPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the filled tonal icon button click handler, either immediately or asynchronously.
pub enum ComposeGeneratedFilledTonalIconButtonPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the filter chip click handler, either immediately or asynchronously.
pub enum ComposeGeneratedFilterChipPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the floating action button click handler, either immediately or asynchronously.
pub enum ComposeGeneratedFloatingActionButtonPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the input chip click handler, either immediately or asynchronously.
pub enum ComposeGeneratedInputChipPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the large floating action button click handler, either immediately or asynchronously.
pub enum ComposeGeneratedLargeFloatingActionButtonPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the leading icon tab click handler, either immediately or asynchronously.
pub enum ComposeGeneratedLeadingIconTabPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the navigation drawer item click handler, either immediately or asynchronously.
pub enum ComposeGeneratedNavigationDrawerItemPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the navigation rail item click handler, either immediately or asynchronously.
pub enum ComposeGeneratedNavigationRailItemPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the outlined button click handler, either immediately or asynchronously.
pub enum ComposeGeneratedOutlinedButtonPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the outlined icon button click handler, either immediately or asynchronously.
pub enum ComposeGeneratedOutlinedIconButtonPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the pull to refresh box refresh handler, either immediately or asynchronously.
pub enum ComposeGeneratedPullToRefreshBoxPropsOnRefreshOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the radio button click handler, either immediately or asynchronously.
pub enum ComposeGeneratedRadioButtonPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the short navigation bar item click handler, either immediately or asynchronously.
pub enum ComposeGeneratedShortNavigationBarItemPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the small floating action button click handler, either immediately or asynchronously.
pub enum ComposeGeneratedSmallFloatingActionButtonPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the suggestion chip click handler, either immediately or asynchronously.
pub enum ComposeGeneratedSuggestionChipPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the tab click handler, either immediately or asynchronously.
pub enum ComposeGeneratedTabPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the text button click handler, either immediately or asynchronously.
pub enum ComposeGeneratedTextButtonPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the time picker dialog dismissal request handler, either immediately or asynchronously.
pub enum ComposeGeneratedTimePickerDialogPropsOnDismissRequestOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Completion returned by the wide navigation rail item click handler, either immediately or asynchronously.
pub enum ComposeGeneratedWideNavigationRailItemPropsOnClickOutput {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Properties configuring a vertical column layout with child alignment and arrangement.
pub struct ComposeGeneratedColumnProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Horizontal alignment applied to child nodes.
    pub horizontalAlignment: Option<ComposeAlignment>,
    /// Vertical spacing and placement strategy for child nodes.
    pub verticalArrangement: Option<ComposeArrangement>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a horizontal layout that arranges child nodes and can handle row clicks.
pub struct ComposeGeneratedRowProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Horizontal spacing and placement strategy for child nodes.
    pub horizontal_arrangement: Option<ComposeArrangement>,
    /// Called when the user activates the component.
    pub on_click: Option<
        Arc<dyn Fn() -> ComposeGeneratedRowPropsOnClickOutput + Send + Sync>,
    >,
    /// Vertical alignment applied to child nodes.
    pub vertical_alignment: Option<ComposeAlignment>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a stacking layout that aligns child nodes within the same bounds.
pub struct ComposeGeneratedBoxProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Alignment used to position child content within the available bounds.
    pub contentAlignment: Option<ComposeAlignment>,
    /// Passes the parent's minimum constraints through to child content when true.
    pub propagateMinConstraints: Option<bool>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring an empty layout node used to reserve space between neighboring content.
pub struct ComposeGeneratedSpacerProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a vertically scrolling lazy list that composes visible child content.
pub struct ComposeGeneratedLazyColumnProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Automatically scrolls the list to its final item as content is appended.
    pub autoScrollToEnd: Option<bool>,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Horizontal alignment applied to child nodes.
    pub horizontalAlignment: Option<ComposeAlignment>,
    /// Reverses item order and the list's scrolling direction when true.
    pub reverseLayout: Option<bool>,
    /// Space inserted between neighboring list items.
    pub spacing: Option<f64>,
    /// Vertical spacing and placement strategy for child nodes.
    pub verticalArrangement: Option<ComposeArrangement>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a horizontally scrolling lazy list that composes visible child content.
pub struct ComposeGeneratedLazyRowProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Horizontal spacing and placement strategy for child nodes.
    pub horizontalArrangement: Option<ComposeArrangement>,
    /// Reverses item order and the list's scrolling direction when true.
    pub reverseLayout: Option<bool>,
    /// Vertical alignment applied to child nodes.
    pub verticalAlignment: Option<ComposeAlignment>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a Material text node with typography, wrapping, overflow, and color controls.
pub struct ComposeGeneratedTextProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Primary color used to render the component.
    pub color: Option<ComposeColor>,
    /// Font family used to render the text.
    pub fontFamily: Option<String>,
    /// Text size used for the rendered content.
    pub fontSize: Option<f64>,
    /// Weight applied to the rendered text.
    pub fontWeight: Option<String>,
    /// Maximum number of text lines that may be displayed.
    pub maxLines: Option<f64>,
    /// Behavior used when text exceeds its available layout space.
    pub overflow: Option<ComposeTextOverflow>,
    /// Allows text to wrap at soft line-breaking opportunities.
    pub softWrap: Option<bool>,
    /// Typography or component style applied to the rendered content.
    pub style: Option<ComposeTextStyle>,
    /// Text content displayed by the component.
    pub text: String,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring an editable Material text input with labels, adornments, validation state, and value-change handling.
pub struct ComposeGeneratedTextFieldProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Marks the input as invalid and enables its error presentation.
    pub is_error: Option<bool>,
    /// Masks the entered value as password text when true.
    pub is_password: Option<bool>,
    /// Label content identifying the control or destination.
    pub label: Option<ComposeGeneratedTextFieldPropsLabel>,
    /// Icon content rendered before the component's label or value.
    pub leading_icon: Option<ComposeChildren>,
    /// Maximum number of text lines that may be displayed.
    pub max_lines: Option<f64>,
    /// Minimum number of text lines reserved by the input.
    pub min_lines: Option<f64>,
    /// Called with the newly entered text whenever the input value changes.
    pub on_value_change: Arc<dyn Fn(String) -> () + Send + Sync>,
    /// Hint content shown while the text field value is empty.
    pub placeholder: Option<ComposeGeneratedTextFieldPropsPlaceholder>,
    /// Content rendered immediately before the editable text.
    pub prefix: Option<ComposeChildren>,
    /// Prevents editing while preserving focus and text selection behavior.
    pub read_only: Option<bool>,
    /// Constrains the text field to a single horizontal line.
    pub single_line: Option<bool>,
    /// Typography or component style applied to the rendered content.
    pub style: Option<ComposeTextFieldStyle>,
    /// Content rendered immediately after the editable text.
    pub suffix: Option<ComposeChildren>,
    /// Supporting or validation content rendered below the text field.
    pub supporting_text: Option<ComposeChildren>,
    /// Icon content rendered after the component's label or value.
    pub trailing_icon: Option<ComposeChildren>,
    /// Controlled text currently displayed by the input.
    pub value: String,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a binary Material switch with controlled checked state and track and thumb styling.
pub struct ComposeGeneratedSwitchProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controlled toggle state indicating whether the control is checked.
    pub checked: bool,
    /// Color of the switch thumb while checked.
    pub checked_thumb_color: Option<ComposeColor>,
    /// Color of the switch track while checked.
    pub checked_track_color: Option<ComposeColor>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Called with the requested checked state after a toggle interaction.
    pub on_checked_change: Arc<dyn Fn(bool) -> () + Send + Sync>,
    /// Content rendered inside the switch thumb.
    pub thumb_content: Option<ComposeChildren>,
    /// Color of the switch thumb while unchecked.
    pub unchecked_thumb_color: Option<ComposeColor>,
    /// Color of the switch track while unchecked.
    pub unchecked_track_color: Option<ComposeColor>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a binary Material checkbox with controlled checked state.
pub struct ComposeGeneratedCheckboxProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controlled toggle state indicating whether the control is checked.
    pub checked: bool,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Called with the requested checked state after a toggle interaction.
    pub on_checked_change: Arc<dyn Fn(bool) -> () + Send + Sync>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a filled Material action button with configurable content, colors, padding, and click handling.
pub struct ComposeGeneratedButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Background color of the component's container.
    pub container_color: Option<ComposeColor>,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub content_color: Option<ComposeColor>,
    /// Padding between the container boundary and its child content.
    pub content_padding: Option<ComposePadding>,
    /// Background color used when the component is disabled.
    pub disabled_container_color: Option<ComposeColor>,
    /// Foreground color supplied to content while the component is disabled.
    pub disabled_content_color: Option<ComposeColor>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedButtonPropsOnClickOutput + Send + Sync,
    >,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Text content displayed by the component.
    pub text: Option<String>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a compact Material action button intended for an icon or custom content.
pub struct ComposeGeneratedIconButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Icon resource or composed icon content displayed by the component.
    pub icon: Option<String>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedIconButtonPropsOnClickOutput + Send + Sync,
    >,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a Material card surface that groups related content with shape, border, color, and elevation.
pub struct ComposeGeneratedCardProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Border drawn around the component's container.
    pub border: Option<ComposeBorder>,
    /// Background color of the component's container.
    pub containerColor: Option<ComposeColor>,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub contentColor: Option<ComposeColor>,
    /// Shadow elevation that visually raises the component above its surroundings.
    pub elevation: Option<f64>,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a theme scope that applies Material styling to its child content.
pub struct ComposeGeneratedMaterialThemeProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a Material surface that provides color, shape, elevation, opacity, and optional click handling.
pub struct ComposeGeneratedSurfaceProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Opacity applied to the rendered node, from transparent to fully opaque.
    pub alpha: Option<f64>,
    /// Primary color used to render the component.
    pub color: Option<ComposeColor>,
    /// Background color of the component's container.
    pub container_color: Option<ComposeColor>,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub content_color: Option<ComposeColor>,
    /// Called when the user activates the component.
    pub on_click: Option<
        Arc<dyn Fn() -> ComposeGeneratedSurfacePropsOnClickOutput + Send + Sync>,
    >,
    /// Physical shadow elevation cast by the surface.
    pub shadow_elevation: Option<f64>,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Tonal elevation used to adjust the surface color against its background.
    pub tonal_elevation: Option<f64>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a Material icon node with accessibility description, size, and tint.
pub struct ComposeGeneratedIconProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Accessible description announced for non-text visual content.
    pub contentDescription: Option<String>,
    /// Named image or icon resource used as the visual source.
    pub name: Option<String>,
    /// Rendered width and height of the icon.
    pub size: Option<f64>,
    /// Color tint applied to the icon.
    pub tint: Option<ComposeColor>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a horizontal Material progress indicator for determinate or indeterminate work.
pub struct ComposeGeneratedLinearProgressIndicatorProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Primary color used to render the component.
    pub color: Option<ComposeColor>,
    /// Determinate completion fraction displayed by the progress indicator.
    pub progress: Option<f64>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a circular Material progress indicator with configurable color and stroke width.
pub struct ComposeGeneratedCircularProgressIndicatorProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Primary color used to render the component.
    pub color: Option<ComposeColor>,
    /// Width of the circular progress indicator's painted stroke.
    pub strokeWidth: Option<f64>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring the layout host where queued snackbar messages are displayed.
pub struct ComposeGeneratedSnackbarHostProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a compact assist action chip with label, optional icons, enabled state, and click handling.
pub struct ComposeGeneratedAssistChipProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Label content identifying the control or destination.
    pub label: ComposeChildren,
    /// Icon content rendered before the component's label or value.
    pub leading_icon: Option<ComposeChildren>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedAssistChipPropsOnClickOutput + Send + Sync,
    >,
    /// Icon content rendered after the component's label or value.
    pub trailing_icon: Option<ComposeChildren>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a small status or count indicator rendered over associated content.
pub struct ComposeGeneratedBadgeProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub contentColor: Option<ComposeColor>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a container that positions a badge relative to its primary content.
pub struct ComposeGeneratedBadgedBoxProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Badge content positioned over or beside the associated destination content.
    pub badge: ComposeChildren,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring the sheet surface and content used inside a dismissible navigation drawer.
pub struct ComposeGeneratedDismissibleDrawerSheetProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Tonal elevation used to distinguish the drawer sheet from its surroundings.
    pub drawerTonalElevation: Option<f64>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a navigation drawer that can be opened, closed, and dismissed with gestures.
pub struct ComposeGeneratedDismissibleNavigationDrawerProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Navigation content rendered inside the drawer or rail panel.
    pub drawerContent: ComposeChildren,
    /// Allows pointer gestures to open and close the navigation container.
    pub gesturesEnabled: Option<bool>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a horizontal separator between adjacent regions of content.
pub struct ComposeGeneratedDividerProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Primary color used to render the component.
    pub color: Option<ComposeColor>,
    /// Thickness of the divider line.
    pub thickness: Option<f64>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring an anchored popup menu with explicit visibility, positioning, dismissal, and window behavior.
pub struct ComposeGeneratedDropdownMenuProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Controls whether the popup menu is currently visible.
    pub expanded: bool,
    /// Position offset applied to the popup relative to its anchor.
    pub offset: Option<f64>,
    /// Called when user interaction requests that the popup or dialog close.
    pub on_dismiss_request: Arc<
        dyn Fn() -> ComposeGeneratedDropdownMenuPropsOnDismissRequestOutput + Send + Sync,
    >,
    /// Popup-window focus, dismissal, clipping, and sizing behavior.
    pub properties: Option<ComposeGeneratedDropdownMenuPropsProperties>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring an elevated assist action chip with label, optional icons, enabled state, and click handling.
pub struct ComposeGeneratedElevatedAssistChipProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Label content identifying the control or destination.
    pub label: ComposeChildren,
    /// Icon content rendered before the component's label or value.
    pub leading_icon: Option<ComposeChildren>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedElevatedAssistChipPropsOnClickOutput + Send + Sync,
    >,
    /// Icon content rendered after the component's label or value.
    pub trailing_icon: Option<ComposeChildren>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring an elevated Material action button with configurable content, colors, padding, shape, and click handling.
pub struct ComposeGeneratedElevatedButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Background color of the component's container.
    pub container_color: Option<ComposeColor>,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub content_color: Option<ComposeColor>,
    /// Padding between the container boundary and its child content.
    pub content_padding: Option<ComposePadding>,
    /// Background color used when the component is disabled.
    pub disabled_container_color: Option<ComposeColor>,
    /// Foreground color supplied to content while the component is disabled.
    pub disabled_content_color: Option<ComposeColor>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedElevatedButtonPropsOnClickOutput + Send + Sync,
    >,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring an elevated Material card that groups related content above its surrounding surface.
pub struct ComposeGeneratedElevatedCardProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Border drawn around the component's container.
    pub border: Option<ComposeBorder>,
    /// Background color of the component's container.
    pub containerColor: Option<ComposeColor>,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub contentColor: Option<ComposeColor>,
    /// Shadow elevation that visually raises the component above its surroundings.
    pub elevation: Option<f64>,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring an elevated selectable filter chip with controlled selection, icons, and click handling.
pub struct ComposeGeneratedElevatedFilterChipProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Label content identifying the control or destination.
    pub label: ComposeChildren,
    /// Icon content rendered before the component's label or value.
    pub leading_icon: Option<ComposeChildren>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedElevatedFilterChipPropsOnClickOutput + Send + Sync,
    >,
    /// Controlled state indicating whether this option or destination is selected.
    pub selected: bool,
    /// Icon content rendered after the component's label or value.
    pub trailing_icon: Option<ComposeChildren>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring an elevated suggestion chip that presents a recommended action.
pub struct ComposeGeneratedElevatedSuggestionChipProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Icon resource or composed icon content displayed by the component.
    pub icon: Option<ComposeChildren>,
    /// Label content identifying the control or destination.
    pub label: ComposeChildren,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn(
        ) -> ComposeGeneratedElevatedSuggestionChipPropsOnClickOutput + Send + Sync,
    >,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring an extended floating action button for a prominent screen-level action.
pub struct ComposeGeneratedExtendedFloatingActionButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub content_color: Option<ComposeColor>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn(
        ) -> ComposeGeneratedExtendedFloatingActionButtonPropsOnClickOutput + Send + Sync,
    >,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a filled compact Material action button intended for an icon or custom content.
pub struct ComposeGeneratedFilledIconButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Icon resource or composed icon content displayed by the component.
    pub icon: Option<String>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedFilledIconButtonPropsOnClickOutput + Send + Sync,
    >,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a filled icon button with controlled checked state.
pub struct ComposeGeneratedFilledIconToggleButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controlled toggle state indicating whether the control is checked.
    pub checked: bool,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Called with the requested checked state after a toggle interaction.
    pub on_checked_change: Arc<dyn Fn(bool) -> () + Send + Sync>,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a tonal filled Material action button with configurable content, colors, padding, and click handling.
pub struct ComposeGeneratedFilledTonalButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Background color of the component's container.
    pub container_color: Option<ComposeColor>,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub content_color: Option<ComposeColor>,
    /// Padding between the container boundary and its child content.
    pub content_padding: Option<ComposePadding>,
    /// Background color used when the component is disabled.
    pub disabled_container_color: Option<ComposeColor>,
    /// Foreground color supplied to content while the component is disabled.
    pub disabled_content_color: Option<ComposeColor>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedFilledTonalButtonPropsOnClickOutput + Send + Sync,
    >,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a tonal filled compact Material action button intended for an icon or custom content.
pub struct ComposeGeneratedFilledTonalIconButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Icon resource or composed icon content displayed by the component.
    pub icon: Option<String>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedFilledTonalIconButtonPropsOnClickOutput + Send + Sync,
    >,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a tonal filled icon button with controlled checked state.
pub struct ComposeGeneratedFilledTonalIconToggleButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controlled toggle state indicating whether the control is checked.
    pub checked: bool,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Called with the requested checked state after a toggle interaction.
    pub on_checked_change: Arc<dyn Fn(bool) -> () + Send + Sync>,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a selectable filter chip with controlled selection, optional icons, and click handling.
pub struct ComposeGeneratedFilterChipProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Label content identifying the control or destination.
    pub label: ComposeChildren,
    /// Icon content rendered before the component's label or value.
    pub leading_icon: Option<ComposeChildren>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedFilterChipPropsOnClickOutput + Send + Sync,
    >,
    /// Controlled state indicating whether this option or destination is selected.
    pub selected: bool,
    /// Icon content rendered after the component's label or value.
    pub trailing_icon: Option<ComposeChildren>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a floating action button for a prominent screen-level action.
pub struct ComposeGeneratedFloatingActionButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub content_color: Option<ComposeColor>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedFloatingActionButtonPropsOnClickOutput + Send + Sync,
    >,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a horizontal separator with configurable thickness and color.
pub struct ComposeGeneratedHorizontalDividerProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Primary color used to render the component.
    pub color: Option<ComposeColor>,
    /// Thickness of the divider line.
    pub thickness: Option<f64>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring an icon button with controlled checked state.
pub struct ComposeGeneratedIconToggleButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controlled toggle state indicating whether the control is checked.
    pub checked: bool,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Icon resource or composed icon content displayed by the component.
    pub icon: Option<String>,
    /// Called with the requested checked state after a toggle interaction.
    pub on_checked_change: Arc<dyn Fn(bool) -> () + Send + Sync>,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring an input chip representing user-supplied information, with avatar, label, icons, and selection state.
pub struct ComposeGeneratedInputChipProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Avatar content shown at the leading edge of the input chip.
    pub avatar: Option<ComposeChildren>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Label content identifying the control or destination.
    pub label: ComposeChildren,
    /// Icon content rendered before the component's label or value.
    pub leading_icon: Option<ComposeChildren>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedInputChipPropsOnClickOutput + Send + Sync,
    >,
    /// Controlled state indicating whether this option or destination is selected.
    pub selected: bool,
    /// Icon content rendered after the component's label or value.
    pub trailing_icon: Option<ComposeChildren>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a large floating action button for a prominent screen-level action.
pub struct ComposeGeneratedLargeFloatingActionButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub content_color: Option<ComposeColor>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn(
        ) -> ComposeGeneratedLargeFloatingActionButtonPropsOnClickOutput + Send + Sync,
    >,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a selectable tab that displays a leading icon beside its label.
pub struct ComposeGeneratedLeadingIconTabProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Icon resource or composed icon content displayed by the component.
    pub icon: ComposeChildren,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedLeadingIconTabPropsOnClickOutput + Send + Sync,
    >,
    /// Controlled state indicating whether this option or destination is selected.
    pub selected: bool,
    /// Text content displayed by the component.
    pub text: ComposeChildren,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a Material list row with headline, supporting, overline, leading, and trailing content slots.
pub struct ComposeGeneratedListItemProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Primary headline content of the list item.
    pub headlineContent: ComposeChildren,
    /// Content rendered before the list item's headline.
    pub leadingContent: Option<ComposeChildren>,
    /// Optional overline content rendered above the list item's headline.
    pub overlineContent: Option<ComposeChildren>,
    /// Physical shadow elevation cast by the surface.
    pub shadowElevation: Option<f64>,
    /// Secondary supporting content of the list item.
    pub supportingContent: Option<ComposeChildren>,
    /// Tonal elevation used to adjust the surface color against its background.
    pub tonalElevation: Option<f64>,
    /// Content rendered after the list item's headline and supporting text.
    pub trailingContent: Option<ComposeChildren>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring the sheet surface and content used inside a modal navigation drawer.
pub struct ComposeGeneratedModalDrawerSheetProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Tonal elevation used to distinguish the drawer sheet from its surroundings.
    pub drawerTonalElevation: Option<f64>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a modal navigation drawer with gesture handling and separate drawer and body content.
pub struct ComposeGeneratedModalNavigationDrawerProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Navigation content rendered inside the drawer or rail panel.
    pub drawerContent: ComposeChildren,
    /// Allows pointer gestures to open and close the navigation container.
    pub gesturesEnabled: Option<bool>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a modal wide navigation rail with collapsible header and navigation content.
pub struct ComposeGeneratedModalWideNavigationRailProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Top padding applied to the header while the wide rail is expanded.
    pub expandedHeaderTopPadding: Option<f64>,
    /// Optional header content rendered before the main navigation destinations.
    pub header: Option<ComposeChildren>,
    /// Hides the configured content when the wide navigation rail collapses.
    pub hideOnCollapse: Option<bool>,
    /// Vertical spacing and placement strategy for child nodes.
    pub verticalArrangement: Option<ComposeArrangement>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a bottom navigation bar that lays out destination content.
pub struct ComposeGeneratedNavigationBarProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub contentColor: Option<ComposeColor>,
    /// Tonal elevation used to adjust the surface color against its background.
    pub tonalElevation: Option<f64>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a selectable destination row within a navigation drawer.
pub struct ComposeGeneratedNavigationDrawerItemProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Badge content positioned over or beside the associated destination content.
    pub badge: Option<ComposeChildren>,
    /// Icon resource or composed icon content displayed by the component.
    pub icon: Option<ComposeChildren>,
    /// Label content identifying the control or destination.
    pub label: ComposeChildren,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedNavigationDrawerItemPropsOnClickOutput + Send + Sync,
    >,
    /// Controlled state indicating whether this option or destination is selected.
    pub selected: bool,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a vertical navigation rail with an optional header and destination content.
pub struct ComposeGeneratedNavigationRailProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub contentColor: Option<ComposeColor>,
    /// Optional header content rendered before the main navigation destinations.
    pub header: Option<ComposeChildren>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a selectable destination item within a navigation rail.
pub struct ComposeGeneratedNavigationRailItemProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Keeps the destination label visible even when the item is not selected.
    pub always_show_label: Option<bool>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Icon resource or composed icon content displayed by the component.
    pub icon: ComposeChildren,
    /// Label content identifying the control or destination.
    pub label: Option<ComposeChildren>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedNavigationRailItemPropsOnClickOutput + Send + Sync,
    >,
    /// Controlled state indicating whether this option or destination is selected.
    pub selected: bool,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring an outlined Material action button with configurable content, colors, padding, shape, and click handling.
pub struct ComposeGeneratedOutlinedButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Background color of the component's container.
    pub container_color: Option<ComposeColor>,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub content_color: Option<ComposeColor>,
    /// Padding between the container boundary and its child content.
    pub content_padding: Option<ComposePadding>,
    /// Background color used when the component is disabled.
    pub disabled_container_color: Option<ComposeColor>,
    /// Foreground color supplied to content while the component is disabled.
    pub disabled_content_color: Option<ComposeColor>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedOutlinedButtonPropsOnClickOutput + Send + Sync,
    >,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring an outlined Material card that groups related content within a bordered surface.
pub struct ComposeGeneratedOutlinedCardProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Border drawn around the component's container.
    pub border: Option<ComposeBorder>,
    /// Background color of the component's container.
    pub containerColor: Option<ComposeColor>,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub contentColor: Option<ComposeColor>,
    /// Shadow elevation that visually raises the component above its surroundings.
    pub elevation: Option<f64>,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring an outlined compact Material action button intended for an icon or custom content.
pub struct ComposeGeneratedOutlinedIconButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Icon resource or composed icon content displayed by the component.
    pub icon: Option<String>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedOutlinedIconButtonPropsOnClickOutput + Send + Sync,
    >,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring an outlined icon button with controlled checked state.
pub struct ComposeGeneratedOutlinedIconToggleButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controlled toggle state indicating whether the control is checked.
    pub checked: bool,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Called with the requested checked state after a toggle interaction.
    pub on_checked_change: Arc<dyn Fn(bool) -> () + Send + Sync>,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring the always-visible sheet surface and content of a permanent navigation drawer.
pub struct ComposeGeneratedPermanentDrawerSheetProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Tonal elevation used to distinguish the drawer sheet from its surroundings.
    pub drawerTonalElevation: Option<f64>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring an always-visible navigation drawer with separate drawer and body content.
pub struct ComposeGeneratedPermanentNavigationDrawerProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Navigation content rendered inside the drawer or rail panel.
    pub drawerContent: ComposeChildren,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a primary tab strip that scrolls when its tabs exceed the available width.
pub struct ComposeGeneratedPrimaryScrollableTabRowProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Default foreground color supplied to child content.
    pub contentColor: Option<ComposeColor>,
    /// Divider content separating the tab strip from adjacent content.
    pub divider: Option<ComposeChildren>,
    /// Horizontal padding before the first and after the last scrollable tab.
    pub edgePadding: Option<f64>,
    /// Custom indicator content showing selection or refresh state.
    pub indicator: Option<ComposeChildren>,
    /// Zero-based index of the tab whose indicator is active.
    pub selectedTabIndex: f64,
    /// Tab nodes rendered by the tab row.
    pub tabs: ComposeChildren,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a primary fixed-width tab strip with selection indicator and divider slots.
pub struct ComposeGeneratedPrimaryTabRowProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Default foreground color supplied to child content.
    pub contentColor: Option<ComposeColor>,
    /// Divider content separating the tab strip from adjacent content.
    pub divider: Option<ComposeChildren>,
    /// Custom indicator content showing selection or refresh state.
    pub indicator: Option<ComposeChildren>,
    /// Zero-based index of the tab whose indicator is active.
    pub selectedTabIndex: f64,
    /// Tab nodes rendered by the tab row.
    pub tabs: ComposeChildren,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a typography scope that supplies a text style to descendant text nodes.
pub struct ComposeGeneratedProvideTextStyleProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Typography or component style applied to the rendered content.
    pub style: Option<ComposeTextStyle>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a pull-to-refresh container with controlled refresh state, indicator, and refresh callback.
pub struct ComposeGeneratedPullToRefreshBoxProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Alignment used to position child content within the available bounds.
    pub content_alignment: Option<ComposeAlignment>,
    /// Custom indicator content showing selection or refresh state.
    pub indicator: Option<ComposeChildren>,
    /// Controlled state indicating that refresh work is currently active.
    pub is_refreshing: bool,
    /// Called when the pull gesture requests a content refresh.
    pub on_refresh: Arc<
        dyn Fn() -> ComposeGeneratedPullToRefreshBoxPropsOnRefreshOutput + Send + Sync,
    >,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a selectable Material radio control for choosing one option from a group.
pub struct ComposeGeneratedRadioButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedRadioButtonPropsOnClickOutput + Send + Sync,
    >,
    /// Controlled state indicating whether this option or destination is selected.
    pub selected: bool,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a Material screen layout coordinating top bar, bottom bar, body, snackbar host, and floating action button.
pub struct ComposeGeneratedScaffoldProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Content rendered in the scaffold's bottom bar slot.
    pub bottomBar: Option<ComposeChildren>,
    /// Background color of the component's container.
    pub containerColor: Option<ComposeColor>,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub contentColor: Option<ComposeColor>,
    /// Content rendered in the scaffold's floating action button slot.
    pub floatingActionButton: Option<ComposeChildren>,
    /// Host content responsible for displaying scaffold snackbar messages.
    pub snackbarHost: Option<ComposeChildren>,
    /// Content rendered in the scaffold's top app bar slot.
    pub topBar: Option<ComposeChildren>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a secondary tab strip that scrolls when its tabs exceed the available width.
pub struct ComposeGeneratedSecondaryScrollableTabRowProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Default foreground color supplied to child content.
    pub contentColor: Option<ComposeColor>,
    /// Divider content separating the tab strip from adjacent content.
    pub divider: Option<ComposeChildren>,
    /// Horizontal padding before the first and after the last scrollable tab.
    pub edgePadding: Option<f64>,
    /// Custom indicator content showing selection or refresh state.
    pub indicator: Option<ComposeChildren>,
    /// Zero-based index of the tab whose indicator is active.
    pub selectedTabIndex: f64,
    /// Tab nodes rendered by the tab row.
    pub tabs: ComposeChildren,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a secondary fixed-width tab strip with selection indicator and divider slots.
pub struct ComposeGeneratedSecondaryTabRowProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Default foreground color supplied to child content.
    pub contentColor: Option<ComposeColor>,
    /// Divider content separating the tab strip from adjacent content.
    pub divider: Option<ComposeChildren>,
    /// Custom indicator content showing selection or refresh state.
    pub indicator: Option<ComposeChildren>,
    /// Zero-based index of the tab whose indicator is active.
    pub selectedTabIndex: f64,
    /// Tab nodes rendered by the tab row.
    pub tabs: ComposeChildren,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a compact bottom navigation bar that lays out destination content.
pub struct ComposeGeneratedShortNavigationBarProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub contentColor: Option<ComposeColor>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a selectable destination item within a compact navigation bar.
pub struct ComposeGeneratedShortNavigationBarItemProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Icon resource or composed icon content displayed by the component.
    pub icon: ComposeChildren,
    /// Label content identifying the control or destination.
    pub label: ComposeChildren,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn(
        ) -> ComposeGeneratedShortNavigationBarItemPropsOnClickOutput + Send + Sync,
    >,
    /// Controlled state indicating whether this option or destination is selected.
    pub selected: bool,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a small floating action button for a prominent screen-level action.
pub struct ComposeGeneratedSmallFloatingActionButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub content_color: Option<ComposeColor>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn(
        ) -> ComposeGeneratedSmallFloatingActionButtonPropsOnClickOutput + Send + Sync,
    >,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a transient message surface with optional action and dismissal content.
pub struct ComposeGeneratedSnackbarProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Action content displayed alongside the snackbar message.
    pub action: Option<ComposeChildren>,
    /// Places the snackbar action on a separate line when true.
    pub actionOnNewLine: Option<bool>,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub contentColor: Option<ComposeColor>,
    /// Optional snackbar control that dismisses the current message.
    pub dismissAction: Option<ComposeChildren>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a suggestion chip that presents a recommended action.
pub struct ComposeGeneratedSuggestionChipProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Icon resource or composed icon content displayed by the component.
    pub icon: Option<ComposeChildren>,
    /// Label content identifying the control or destination.
    pub label: ComposeChildren,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedSuggestionChipPropsOnClickOutput + Send + Sync,
    >,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a selectable tab with custom content and controlled selection state.
pub struct ComposeGeneratedTabProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Called when the user activates the component.
    pub on_click: Arc<dyn Fn() -> ComposeGeneratedTabPropsOnClickOutput + Send + Sync>,
    /// Controlled state indicating whether this option or destination is selected.
    pub selected: bool,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a low-emphasis text action button with configurable content, colors, padding, and click handling.
pub struct ComposeGeneratedTextButtonProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Background color of the component's container.
    pub container_color: Option<ComposeColor>,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Default foreground color supplied to child content.
    pub content_color: Option<ComposeColor>,
    /// Padding between the container boundary and its child content.
    pub content_padding: Option<ComposePadding>,
    /// Background color used when the component is disabled.
    pub disabled_container_color: Option<ComposeColor>,
    /// Foreground color supplied to content while the component is disabled.
    pub disabled_content_color: Option<ComposeColor>,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn() -> ComposeGeneratedTextButtonPropsOnClickOutput + Send + Sync,
    >,
    /// Shape used for the component's container outline.
    pub shape: Option<ComposeShape>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a Material time-picker dialog with title, controls, confirmation, dismissal, and mode switching slots.
pub struct ComposeGeneratedTimePickerDialogProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Confirmation action displayed by the time-picker dialog.
    pub confirm_button: ComposeChildren,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Optional dismissal action displayed by the time-picker dialog.
    pub dismiss_button: Option<ComposeChildren>,
    /// Optional control for switching the time picker's input mode.
    pub mode_toggle_button: Option<ComposeChildren>,
    /// Called when user interaction requests that the popup or dialog close.
    pub on_dismiss_request: Arc<
        dyn Fn(
        ) -> ComposeGeneratedTimePickerDialogPropsOnDismissRequestOutput + Send + Sync,
    >,
    /// Title content displayed at the top of the time-picker dialog.
    pub title: ComposeChildren,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a vertical separator with configurable thickness and color.
pub struct ComposeGeneratedVerticalDividerProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Primary color used to render the component.
    pub color: Option<ComposeColor>,
    /// Thickness of the divider line.
    pub thickness: Option<f64>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a visual handle indicating that a surface can be dragged vertically.
pub struct ComposeGeneratedVerticalDragHandleProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring an expanded vertical navigation rail with header and destination content.
pub struct ComposeGeneratedWideNavigationRailProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Optional header content rendered before the main navigation destinations.
    pub header: Option<ComposeChildren>,
    /// Vertical spacing and placement strategy for child nodes.
    pub verticalArrangement: Option<ComposeArrangement>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a selectable destination item that adapts to an expanded or collapsed wide navigation rail.
pub struct ComposeGeneratedWideNavigationRailItemProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controls whether the component accepts user interaction.
    pub enabled: Option<bool>,
    /// Icon resource or composed icon content displayed by the component.
    pub icon: ComposeChildren,
    /// Label content identifying the control or destination.
    pub label: ComposeChildren,
    /// Called when the user activates the component.
    pub on_click: Arc<
        dyn Fn(
        ) -> ComposeGeneratedWideNavigationRailItemPropsOnClickOutput + Send + Sync,
    >,
    /// Indicates whether the containing wide navigation rail is expanded.
    pub rail_expanded: bool,
    /// Controlled state indicating whether this option or destination is selected.
    pub selected: bool,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub z_index: Option<f64>,
}
/// Properties configuring a stacking layout that exposes its available constraints while aligning child content.
pub struct ComposeGeneratedBoxWithConstraintsProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Alignment used to position child content within the available bounds.
    pub contentAlignment: Option<ComposeAlignment>,
    /// Passes the parent's minimum constraints through to child content when true.
    pub propagateMinConstraints: Option<bool>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a lightweight text node with typography, wrapping, and overflow controls.
pub struct ComposeGeneratedBasicTextProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Font family used to render the text.
    pub fontFamily: Option<String>,
    /// Text size used for the rendered content.
    pub fontSize: Option<f64>,
    /// Maximum number of text lines that may be displayed.
    pub maxLines: Option<f64>,
    /// Behavior used when text exceeds its available layout space.
    pub overflow: Option<ComposeTextOverflow>,
    /// Allows text to wrap at soft line-breaking opportunities.
    pub softWrap: Option<bool>,
    /// Typography or component style applied to the rendered content.
    pub style: Option<ComposeTextStyle>,
    /// Text content displayed by the component.
    pub text: String,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a scope that prevents text selection within its child content.
pub struct ComposeGeneratedDisableSelectionProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring an image node supporting local, URI, URL, resource, and icon sources with scaling and accessibility controls.
pub struct ComposeGeneratedImageProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Opacity applied to the rendered node, from transparent to fully opaque.
    pub alpha: Option<f64>,
    /// Alignment used to position child content within the available bounds.
    pub contentAlignment: Option<ComposeAlignment>,
    /// Accessible description announced for non-text visual content.
    pub contentDescription: Option<String>,
    /// Scaling strategy used to fit the image into its layout bounds.
    pub contentScale: Option<ComposeContentScale>,
    /// File URI identifying the image source.
    pub fileUri: Option<String>,
    /// Icon resource or composed icon content displayed by the component.
    pub icon: Option<String>,
    /// Named image or icon resource used as the visual source.
    pub name: Option<String>,
    /// Local filesystem path identifying the image source.
    pub path: Option<String>,
    /// Source identifier used to load the image.
    pub src: Option<String>,
    /// URI identifying the image source.
    pub uri: Option<String>,
    /// Network URL identifying the image source.
    pub url: Option<String>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a scope that enables text selection across its child content.
pub struct ComposeGeneratedSelectionContainerProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the component.
    pub content: Option<ComposeChildren>,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
}
/// Properties configuring a drawing surface that executes an ordered list of canvas commands.
pub struct ComposeGeneratedCanvasProps {
    /// Common layout, sizing, semantics, and modifier properties applied to this node.
    pub base_compose_common_props: ComposeCommonProps,
    /// Drawing order relative to sibling nodes; larger values render above smaller values.
    pub zIndex: Option<f64>,
    /// Ordered drawing commands executed by the canvas.
    pub commands: Option<Vec<ComposeCanvasCommand>>,
}
/// Factories available for constructing every supported Material 3 and Foundation UI node.
pub struct ComposeMaterial3GeneratedUiFactoryRegistry {
    /// Factory used to construct Column nodes from their typed properties.
    pub Column: ComposeNodeFactory<ComposeGeneratedColumnProps>,
    /// Factory used to construct Row nodes from their typed properties.
    pub Row: ComposeNodeFactory<ComposeGeneratedRowProps>,
    /// Factory used to construct Box nodes from their typed properties.
    pub Box: ComposeNodeFactory<ComposeGeneratedBoxProps>,
    /// Factory used to construct Spacer nodes from their typed properties.
    pub Spacer: ComposeNodeFactory<ComposeGeneratedSpacerProps>,
    /// Factory used to construct Lazy Column nodes from their typed properties.
    pub LazyColumn: ComposeNodeFactory<ComposeGeneratedLazyColumnProps>,
    /// Factory used to construct Lazy Row nodes from their typed properties.
    pub LazyRow: ComposeNodeFactory<ComposeGeneratedLazyRowProps>,
    /// Factory used to construct Text nodes from their typed properties.
    pub Text: ComposeNodeFactory<ComposeGeneratedTextProps>,
    /// Factory used to construct Text Field nodes from their typed properties.
    pub TextField: ComposeNodeFactory<ComposeGeneratedTextFieldProps>,
    /// Factory used to construct Switch nodes from their typed properties.
    pub Switch: ComposeNodeFactory<ComposeGeneratedSwitchProps>,
    /// Factory used to construct Checkbox nodes from their typed properties.
    pub Checkbox: ComposeNodeFactory<ComposeGeneratedCheckboxProps>,
    /// Factory used to construct Button nodes from their typed properties.
    pub Button: ComposeNodeFactory<ComposeGeneratedButtonProps>,
    /// Factory used to construct Icon Button nodes from their typed properties.
    pub IconButton: ComposeNodeFactory<ComposeGeneratedIconButtonProps>,
    /// Factory used to construct Card nodes from their typed properties.
    pub Card: ComposeNodeFactory<ComposeGeneratedCardProps>,
    /// Factory used to construct Material Theme nodes from their typed properties.
    pub MaterialTheme: ComposeNodeFactory<ComposeGeneratedMaterialThemeProps>,
    /// Factory used to construct Surface nodes from their typed properties.
    pub Surface: ComposeNodeFactory<ComposeGeneratedSurfaceProps>,
    /// Factory used to construct Icon nodes from their typed properties.
    pub Icon: ComposeNodeFactory<ComposeGeneratedIconProps>,
    /// Factory used to construct Linear Progress Indicator nodes from their typed properties.
    pub LinearProgressIndicator: ComposeNodeFactory<
        ComposeGeneratedLinearProgressIndicatorProps,
    >,
    /// Factory used to construct Circular Progress Indicator nodes from their typed properties.
    pub CircularProgressIndicator: ComposeNodeFactory<
        ComposeGeneratedCircularProgressIndicatorProps,
    >,
    /// Factory used to construct Snackbar Host nodes from their typed properties.
    pub SnackbarHost: ComposeNodeFactory<ComposeGeneratedSnackbarHostProps>,
    /// Factory used to construct Assist Chip nodes from their typed properties.
    pub AssistChip: ComposeNodeFactory<ComposeGeneratedAssistChipProps>,
    /// Factory used to construct Badge nodes from their typed properties.
    pub Badge: ComposeNodeFactory<ComposeGeneratedBadgeProps>,
    /// Factory used to construct Badged Box nodes from their typed properties.
    pub BadgedBox: ComposeNodeFactory<ComposeGeneratedBadgedBoxProps>,
    /// Factory used to construct Dismissible Drawer Sheet nodes from their typed properties.
    pub DismissibleDrawerSheet: ComposeNodeFactory<
        ComposeGeneratedDismissibleDrawerSheetProps,
    >,
    /// Factory used to construct Dismissible Navigation Drawer nodes from their typed properties.
    pub DismissibleNavigationDrawer: ComposeNodeFactory<
        ComposeGeneratedDismissibleNavigationDrawerProps,
    >,
    /// Factory used to construct Divider nodes from their typed properties.
    pub Divider: ComposeNodeFactory<ComposeGeneratedDividerProps>,
    /// Factory used to construct Dropdown Menu nodes from their typed properties.
    pub DropdownMenu: ComposeNodeFactory<ComposeGeneratedDropdownMenuProps>,
    /// Factory used to construct Elevated Assist Chip nodes from their typed properties.
    pub ElevatedAssistChip: ComposeNodeFactory<ComposeGeneratedElevatedAssistChipProps>,
    /// Factory used to construct Elevated Button nodes from their typed properties.
    pub ElevatedButton: ComposeNodeFactory<ComposeGeneratedElevatedButtonProps>,
    /// Factory used to construct Elevated Card nodes from their typed properties.
    pub ElevatedCard: ComposeNodeFactory<ComposeGeneratedElevatedCardProps>,
    /// Factory used to construct Elevated Filter Chip nodes from their typed properties.
    pub ElevatedFilterChip: ComposeNodeFactory<ComposeGeneratedElevatedFilterChipProps>,
    /// Factory used to construct Elevated Suggestion Chip nodes from their typed properties.
    pub ElevatedSuggestionChip: ComposeNodeFactory<
        ComposeGeneratedElevatedSuggestionChipProps,
    >,
    /// Factory used to construct Extended Floating Action Button nodes from their typed properties.
    pub ExtendedFloatingActionButton: ComposeNodeFactory<
        ComposeGeneratedExtendedFloatingActionButtonProps,
    >,
    /// Factory used to construct Filled Icon Button nodes from their typed properties.
    pub FilledIconButton: ComposeNodeFactory<ComposeGeneratedFilledIconButtonProps>,
    /// Factory used to construct Filled Icon Toggle Button nodes from their typed properties.
    pub FilledIconToggleButton: ComposeNodeFactory<
        ComposeGeneratedFilledIconToggleButtonProps,
    >,
    /// Factory used to construct Filled Tonal Button nodes from their typed properties.
    pub FilledTonalButton: ComposeNodeFactory<ComposeGeneratedFilledTonalButtonProps>,
    /// Factory used to construct Filled Tonal Icon Button nodes from their typed properties.
    pub FilledTonalIconButton: ComposeNodeFactory<
        ComposeGeneratedFilledTonalIconButtonProps,
    >,
    /// Factory used to construct Filled Tonal Icon Toggle Button nodes from their typed properties.
    pub FilledTonalIconToggleButton: ComposeNodeFactory<
        ComposeGeneratedFilledTonalIconToggleButtonProps,
    >,
    /// Factory used to construct Filter Chip nodes from their typed properties.
    pub FilterChip: ComposeNodeFactory<ComposeGeneratedFilterChipProps>,
    /// Factory used to construct Floating Action Button nodes from their typed properties.
    pub FloatingActionButton: ComposeNodeFactory<
        ComposeGeneratedFloatingActionButtonProps,
    >,
    /// Factory used to construct Horizontal Divider nodes from their typed properties.
    pub HorizontalDivider: ComposeNodeFactory<ComposeGeneratedHorizontalDividerProps>,
    /// Factory used to construct Icon Toggle Button nodes from their typed properties.
    pub IconToggleButton: ComposeNodeFactory<ComposeGeneratedIconToggleButtonProps>,
    /// Factory used to construct Input Chip nodes from their typed properties.
    pub InputChip: ComposeNodeFactory<ComposeGeneratedInputChipProps>,
    /// Factory used to construct Large Floating Action Button nodes from their typed properties.
    pub LargeFloatingActionButton: ComposeNodeFactory<
        ComposeGeneratedLargeFloatingActionButtonProps,
    >,
    /// Factory used to construct Leading Icon Tab nodes from their typed properties.
    pub LeadingIconTab: ComposeNodeFactory<ComposeGeneratedLeadingIconTabProps>,
    /// Factory used to construct List Item nodes from their typed properties.
    pub ListItem: ComposeNodeFactory<ComposeGeneratedListItemProps>,
    /// Factory used to construct Modal Drawer Sheet nodes from their typed properties.
    pub ModalDrawerSheet: ComposeNodeFactory<ComposeGeneratedModalDrawerSheetProps>,
    /// Factory used to construct Modal Navigation Drawer nodes from their typed properties.
    pub ModalNavigationDrawer: ComposeNodeFactory<
        ComposeGeneratedModalNavigationDrawerProps,
    >,
    /// Factory used to construct Modal Wide Navigation Rail nodes from their typed properties.
    pub ModalWideNavigationRail: ComposeNodeFactory<
        ComposeGeneratedModalWideNavigationRailProps,
    >,
    /// Factory used to construct Navigation Bar nodes from their typed properties.
    pub NavigationBar: ComposeNodeFactory<ComposeGeneratedNavigationBarProps>,
    /// Factory used to construct Navigation Drawer Item nodes from their typed properties.
    pub NavigationDrawerItem: ComposeNodeFactory<
        ComposeGeneratedNavigationDrawerItemProps,
    >,
    /// Factory used to construct Navigation Rail nodes from their typed properties.
    pub NavigationRail: ComposeNodeFactory<ComposeGeneratedNavigationRailProps>,
    /// Factory used to construct Navigation Rail Item nodes from their typed properties.
    pub NavigationRailItem: ComposeNodeFactory<ComposeGeneratedNavigationRailItemProps>,
    /// Factory used to construct Outlined Button nodes from their typed properties.
    pub OutlinedButton: ComposeNodeFactory<ComposeGeneratedOutlinedButtonProps>,
    /// Factory used to construct Outlined Card nodes from their typed properties.
    pub OutlinedCard: ComposeNodeFactory<ComposeGeneratedOutlinedCardProps>,
    /// Factory used to construct Outlined Icon Button nodes from their typed properties.
    pub OutlinedIconButton: ComposeNodeFactory<ComposeGeneratedOutlinedIconButtonProps>,
    /// Factory used to construct Outlined Icon Toggle Button nodes from their typed properties.
    pub OutlinedIconToggleButton: ComposeNodeFactory<
        ComposeGeneratedOutlinedIconToggleButtonProps,
    >,
    /// Factory used to construct Permanent Drawer Sheet nodes from their typed properties.
    pub PermanentDrawerSheet: ComposeNodeFactory<
        ComposeGeneratedPermanentDrawerSheetProps,
    >,
    /// Factory used to construct Permanent Navigation Drawer nodes from their typed properties.
    pub PermanentNavigationDrawer: ComposeNodeFactory<
        ComposeGeneratedPermanentNavigationDrawerProps,
    >,
    /// Factory used to construct Primary Scrollable Tab Row nodes from their typed properties.
    pub PrimaryScrollableTabRow: ComposeNodeFactory<
        ComposeGeneratedPrimaryScrollableTabRowProps,
    >,
    /// Factory used to construct Primary Tab Row nodes from their typed properties.
    pub PrimaryTabRow: ComposeNodeFactory<ComposeGeneratedPrimaryTabRowProps>,
    /// Factory used to construct Provide Text Style nodes from their typed properties.
    pub ProvideTextStyle: ComposeNodeFactory<ComposeGeneratedProvideTextStyleProps>,
    /// Factory used to construct Pull To Refresh Box nodes from their typed properties.
    pub PullToRefreshBox: ComposeNodeFactory<ComposeGeneratedPullToRefreshBoxProps>,
    /// Factory used to construct Radio Button nodes from their typed properties.
    pub RadioButton: ComposeNodeFactory<ComposeGeneratedRadioButtonProps>,
    /// Factory used to construct Scaffold nodes from their typed properties.
    pub Scaffold: ComposeNodeFactory<ComposeGeneratedScaffoldProps>,
    /// Factory used to construct Secondary Scrollable Tab Row nodes from their typed properties.
    pub SecondaryScrollableTabRow: ComposeNodeFactory<
        ComposeGeneratedSecondaryScrollableTabRowProps,
    >,
    /// Factory used to construct Secondary Tab Row nodes from their typed properties.
    pub SecondaryTabRow: ComposeNodeFactory<ComposeGeneratedSecondaryTabRowProps>,
    /// Factory used to construct Short Navigation Bar nodes from their typed properties.
    pub ShortNavigationBar: ComposeNodeFactory<ComposeGeneratedShortNavigationBarProps>,
    /// Factory used to construct Short Navigation Bar Item nodes from their typed properties.
    pub ShortNavigationBarItem: ComposeNodeFactory<
        ComposeGeneratedShortNavigationBarItemProps,
    >,
    /// Factory used to construct Small Floating Action Button nodes from their typed properties.
    pub SmallFloatingActionButton: ComposeNodeFactory<
        ComposeGeneratedSmallFloatingActionButtonProps,
    >,
    /// Factory used to construct Snackbar nodes from their typed properties.
    pub Snackbar: ComposeNodeFactory<ComposeGeneratedSnackbarProps>,
    /// Factory used to construct Suggestion Chip nodes from their typed properties.
    pub SuggestionChip: ComposeNodeFactory<ComposeGeneratedSuggestionChipProps>,
    /// Factory used to construct Tab nodes from their typed properties.
    pub Tab: ComposeNodeFactory<ComposeGeneratedTabProps>,
    /// Factory used to construct Text Button nodes from their typed properties.
    pub TextButton: ComposeNodeFactory<ComposeGeneratedTextButtonProps>,
    /// Factory used to construct Time Picker Dialog nodes from their typed properties.
    pub TimePickerDialog: ComposeNodeFactory<ComposeGeneratedTimePickerDialogProps>,
    /// Factory used to construct Vertical Divider nodes from their typed properties.
    pub VerticalDivider: ComposeNodeFactory<ComposeGeneratedVerticalDividerProps>,
    /// Factory used to construct Vertical Drag Handle nodes from their typed properties.
    pub VerticalDragHandle: ComposeNodeFactory<ComposeGeneratedVerticalDragHandleProps>,
    /// Factory used to construct Wide Navigation Rail nodes from their typed properties.
    pub WideNavigationRail: ComposeNodeFactory<ComposeGeneratedWideNavigationRailProps>,
    /// Factory used to construct Wide Navigation Rail Item nodes from their typed properties.
    pub WideNavigationRailItem: ComposeNodeFactory<
        ComposeGeneratedWideNavigationRailItemProps,
    >,
    /// Factory used to construct Box With Constraints nodes from their typed properties.
    pub BoxWithConstraints: ComposeNodeFactory<ComposeGeneratedBoxWithConstraintsProps>,
    /// Factory used to construct Basic Text nodes from their typed properties.
    pub BasicText: ComposeNodeFactory<ComposeGeneratedBasicTextProps>,
    /// Factory used to construct Disable Selection nodes from their typed properties.
    pub DisableSelection: ComposeNodeFactory<ComposeGeneratedDisableSelectionProps>,
    /// Factory used to construct Image nodes from their typed properties.
    pub Image: ComposeNodeFactory<ComposeGeneratedImageProps>,
    /// Factory used to construct Selection Container nodes from their typed properties.
    pub SelectionContainer: ComposeNodeFactory<ComposeGeneratedSelectionContainerProps>,
    /// Factory used to construct Canvas nodes from their typed properties.
    pub Canvas: ComposeNodeFactory<ComposeGeneratedCanvasProps>,
}
