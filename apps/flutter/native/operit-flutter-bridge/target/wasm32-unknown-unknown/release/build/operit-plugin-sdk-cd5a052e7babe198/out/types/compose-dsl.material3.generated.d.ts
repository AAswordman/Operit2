// Generated from operit-plugin-sdk Rust declarations.

import type { ComposeAlignment, ComposeArrangement, ComposeBorder, ComposeCanvasCommand, ComposeChildren, ComposeColor, ComposeCommonProps, ComposeContentScale, ComposeNodeFactory, ComposePadding, ComposeShape, ComposeTextFieldStyle, ComposeTextOverflow, ComposeTextStyle } from "./compose-dsl";

/**
 * Completion returned by the row click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedRowPropsOnClickOutput = void | Promise<void>;

/**
 * Content accepted by a text field label: either plain text or composed child nodes.
 */
export type ComposeGeneratedTextFieldPropsLabel = string | ComposeChildren;

/**
 * Content accepted by a text field placeholder: either plain text or composed child nodes.
 */
export type ComposeGeneratedTextFieldPropsPlaceholder = string | ComposeChildren;

/**
 * Completion returned by the button click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedButtonPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the icon button click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedIconButtonPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the surface click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedSurfacePropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the assist chip click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedAssistChipPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the dropdown menu dismissal request handler, either immediately or asynchronously.
 */
export type ComposeGeneratedDropdownMenuPropsOnDismissRequestOutput = void | Promise<void>;

/**
 * Popup-window behavior controlling dropdown focus, dismissal, clipping, and platform sizing.
 */
export interface ComposeGeneratedDropdownMenuPropsProperties {
  /**
   * Allows the popup window to receive focus and keyboard input.
   */
  focusable?: boolean;
  /**
   * Dismisses the popup when the platform back action is invoked.
   */
  dismissOnBackPress?: boolean;
  /**
   * Dismisses the popup when a pointer press occurs outside its bounds.
   */
  dismissOnClickOutside?: boolean;
  /**
   * Constrains the popup window to the visible screen bounds when true.
   */
  clippingEnabled?: boolean;
  /**
   * Uses the platform's default popup width constraints when true.
   */
  usePlatformDefaultWidth?: boolean;
}

/**
 * Completion returned by the elevated assist chip click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedElevatedAssistChipPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the elevated button click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedElevatedButtonPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the elevated filter chip click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedElevatedFilterChipPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the elevated suggestion chip click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedElevatedSuggestionChipPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the extended floating action button click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedExtendedFloatingActionButtonPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the filled icon button click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedFilledIconButtonPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the filled tonal button click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedFilledTonalButtonPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the filled tonal icon button click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedFilledTonalIconButtonPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the filter chip click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedFilterChipPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the floating action button click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedFloatingActionButtonPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the input chip click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedInputChipPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the large floating action button click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedLargeFloatingActionButtonPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the leading icon tab click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedLeadingIconTabPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the navigation drawer item click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedNavigationDrawerItemPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the navigation rail item click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedNavigationRailItemPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the outlined button click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedOutlinedButtonPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the outlined icon button click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedOutlinedIconButtonPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the pull to refresh box refresh handler, either immediately or asynchronously.
 */
export type ComposeGeneratedPullToRefreshBoxPropsOnRefreshOutput = void | Promise<void>;

/**
 * Completion returned by the radio button click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedRadioButtonPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the short navigation bar item click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedShortNavigationBarItemPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the small floating action button click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedSmallFloatingActionButtonPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the suggestion chip click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedSuggestionChipPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the tab click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedTabPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the text button click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedTextButtonPropsOnClickOutput = void | Promise<void>;

/**
 * Completion returned by the time picker dialog dismissal request handler, either immediately or asynchronously.
 */
export type ComposeGeneratedTimePickerDialogPropsOnDismissRequestOutput = void | Promise<void>;

/**
 * Completion returned by the wide navigation rail item click handler, either immediately or asynchronously.
 */
export type ComposeGeneratedWideNavigationRailItemPropsOnClickOutput = void | Promise<void>;

/**
 * Properties configuring a vertical column layout with child alignment and arrangement.
 */
export interface ComposeGeneratedColumnProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Horizontal alignment applied to child nodes.
   */
  horizontalAlignment?: ComposeAlignment;
  /**
   * Vertical spacing and placement strategy for child nodes.
   */
  verticalArrangement?: ComposeArrangement;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a horizontal layout that arranges child nodes and can handle row clicks.
 */
export interface ComposeGeneratedRowProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Horizontal spacing and placement strategy for child nodes.
   */
  horizontalArrangement?: ComposeArrangement;
  /**
   * Called when the user activates the component.
   */
  onClick?: () => ComposeGeneratedRowPropsOnClickOutput;
  /**
   * Vertical alignment applied to child nodes.
   */
  verticalAlignment?: ComposeAlignment;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a stacking layout that aligns child nodes within the same bounds.
 */
export interface ComposeGeneratedBoxProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Alignment used to position child content within the available bounds.
   */
  contentAlignment?: ComposeAlignment;
  /**
   * Passes the parent's minimum constraints through to child content when true.
   */
  propagateMinConstraints?: boolean;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an empty layout node used to reserve space between neighboring content.
 */
export interface ComposeGeneratedSpacerProps extends ComposeCommonProps {
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a vertically scrolling lazy list that composes visible child content.
 */
export interface ComposeGeneratedLazyColumnProps extends ComposeCommonProps {
  /**
   * Automatically scrolls the list to its final item as content is appended.
   */
  autoScrollToEnd?: boolean;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Horizontal alignment applied to child nodes.
   */
  horizontalAlignment?: ComposeAlignment;
  /**
   * Reverses item order and the list's scrolling direction when true.
   */
  reverseLayout?: boolean;
  /**
   * Space inserted between neighboring list items.
   */
  spacing?: number;
  /**
   * Vertical spacing and placement strategy for child nodes.
   */
  verticalArrangement?: ComposeArrangement;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a horizontally scrolling lazy list that composes visible child content.
 */
export interface ComposeGeneratedLazyRowProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Horizontal spacing and placement strategy for child nodes.
   */
  horizontalArrangement?: ComposeArrangement;
  /**
   * Reverses item order and the list's scrolling direction when true.
   */
  reverseLayout?: boolean;
  /**
   * Vertical alignment applied to child nodes.
   */
  verticalAlignment?: ComposeAlignment;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a Material text node with typography, wrapping, overflow, and color controls.
 */
export interface ComposeGeneratedTextProps extends ComposeCommonProps {
  /**
   * Primary color used to render the component.
   */
  color?: ComposeColor;
  /**
   * Font family used to render the text.
   */
  fontFamily?: string;
  /**
   * Text size used for the rendered content.
   */
  fontSize?: number;
  /**
   * Weight applied to the rendered text.
   */
  fontWeight?: string;
  /**
   * Maximum number of text lines that may be displayed.
   */
  maxLines?: number;
  /**
   * Behavior used when text exceeds its available layout space.
   */
  overflow?: ComposeTextOverflow;
  /**
   * Allows text to wrap at soft line-breaking opportunities.
   */
  softWrap?: boolean;
  /**
   * Typography or component style applied to the rendered content.
   */
  style?: ComposeTextStyle;
  /**
   * Text content displayed by the component.
   */
  text: string;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an editable Material text input with labels, adornments, validation state, and value-change handling.
 */
export interface ComposeGeneratedTextFieldProps extends ComposeCommonProps {
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Marks the input as invalid and enables its error presentation.
   */
  isError?: boolean;
  /**
   * Masks the entered value as password text when true.
   */
  isPassword?: boolean;
  /**
   * Label content identifying the control or destination.
   */
  label?: ComposeGeneratedTextFieldPropsLabel;
  /**
   * Icon content rendered before the component's label or value.
   */
  leadingIcon?: ComposeChildren;
  /**
   * Maximum number of text lines that may be displayed.
   */
  maxLines?: number;
  /**
   * Minimum number of text lines reserved by the input.
   */
  minLines?: number;
  /**
   * Called with the newly entered text whenever the input value changes.
   */
  onValueChange: (arg0: string) => void;
  /**
   * Hint content shown while the text field value is empty.
   */
  placeholder?: ComposeGeneratedTextFieldPropsPlaceholder;
  /**
   * Content rendered immediately before the editable text.
   */
  prefix?: ComposeChildren;
  /**
   * Prevents editing while preserving focus and text selection behavior.
   */
  readOnly?: boolean;
  /**
   * Constrains the text field to a single horizontal line.
   */
  singleLine?: boolean;
  /**
   * Typography or component style applied to the rendered content.
   */
  style?: ComposeTextFieldStyle;
  /**
   * Content rendered immediately after the editable text.
   */
  suffix?: ComposeChildren;
  /**
   * Supporting or validation content rendered below the text field.
   */
  supportingText?: ComposeChildren;
  /**
   * Icon content rendered after the component's label or value.
   */
  trailingIcon?: ComposeChildren;
  /**
   * Controlled text currently displayed by the input.
   */
  value: string;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a binary Material switch with controlled checked state and track and thumb styling.
 */
export interface ComposeGeneratedSwitchProps extends ComposeCommonProps {
  /**
   * Controlled toggle state indicating whether the control is checked.
   */
  checked: boolean;
  /**
   * Color of the switch thumb while checked.
   */
  checkedThumbColor?: ComposeColor;
  /**
   * Color of the switch track while checked.
   */
  checkedTrackColor?: ComposeColor;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Called with the requested checked state after a toggle interaction.
   */
  onCheckedChange: (arg0: boolean) => void;
  /**
   * Content rendered inside the switch thumb.
   */
  thumbContent?: ComposeChildren;
  /**
   * Color of the switch thumb while unchecked.
   */
  uncheckedThumbColor?: ComposeColor;
  /**
   * Color of the switch track while unchecked.
   */
  uncheckedTrackColor?: ComposeColor;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a binary Material checkbox with controlled checked state.
 */
export interface ComposeGeneratedCheckboxProps extends ComposeCommonProps {
  /**
   * Controlled toggle state indicating whether the control is checked.
   */
  checked: boolean;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Called with the requested checked state after a toggle interaction.
   */
  onCheckedChange: (arg0: boolean) => void;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a filled Material action button with configurable content, colors, padding, and click handling.
 */
export interface ComposeGeneratedButtonProps extends ComposeCommonProps {
  /**
   * Background color of the component's container.
   */
  containerColor?: ComposeColor;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Padding between the container boundary and its child content.
   */
  contentPadding?: ComposePadding;
  /**
   * Background color used when the component is disabled.
   */
  disabledContainerColor?: ComposeColor;
  /**
   * Foreground color supplied to content while the component is disabled.
   */
  disabledContentColor?: ComposeColor;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedButtonPropsOnClickOutput;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Text content displayed by the component.
   */
  text?: string;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a compact Material action button intended for an icon or custom content.
 */
export interface ComposeGeneratedIconButtonProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Icon resource or composed icon content displayed by the component.
   */
  icon?: string;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedIconButtonPropsOnClickOutput;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a Material card surface that groups related content with shape, border, color, and elevation.
 */
export interface ComposeGeneratedCardProps extends ComposeCommonProps {
  /**
   * Border drawn around the component's container.
   */
  border?: ComposeBorder;
  /**
   * Background color of the component's container.
   */
  containerColor?: ComposeColor;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Shadow elevation that visually raises the component above its surroundings.
   */
  elevation?: number;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a theme scope that applies Material styling to its child content.
 */
export interface ComposeGeneratedMaterialThemeProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a Material surface that provides color, shape, elevation, opacity, and optional click handling.
 */
export interface ComposeGeneratedSurfaceProps extends ComposeCommonProps {
  /**
   * Opacity applied to the rendered node, from transparent to fully opaque.
   */
  alpha?: number;
  /**
   * Primary color used to render the component.
   */
  color?: ComposeColor;
  /**
   * Background color of the component's container.
   */
  containerColor?: ComposeColor;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Called when the user activates the component.
   */
  onClick?: () => ComposeGeneratedSurfacePropsOnClickOutput;
  /**
   * Physical shadow elevation cast by the surface.
   */
  shadowElevation?: number;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Tonal elevation used to adjust the surface color against its background.
   */
  tonalElevation?: number;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a Material icon node with accessibility description, size, and tint.
 */
export interface ComposeGeneratedIconProps extends ComposeCommonProps {
  /**
   * Accessible description announced for non-text visual content.
   */
  contentDescription?: string;
  /**
   * Named image or icon resource used as the visual source.
   */
  name?: string;
  /**
   * Rendered width and height of the icon.
   */
  size?: number;
  /**
   * Color tint applied to the icon.
   */
  tint?: ComposeColor;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a horizontal Material progress indicator for determinate or indeterminate work.
 */
export interface ComposeGeneratedLinearProgressIndicatorProps extends ComposeCommonProps {
  /**
   * Primary color used to render the component.
   */
  color?: ComposeColor;
  /**
   * Determinate completion fraction displayed by the progress indicator.
   */
  progress?: number;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a circular Material progress indicator with configurable color and stroke width.
 */
export interface ComposeGeneratedCircularProgressIndicatorProps extends ComposeCommonProps {
  /**
   * Primary color used to render the component.
   */
  color?: ComposeColor;
  /**
   * Width of the circular progress indicator's painted stroke.
   */
  strokeWidth?: number;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring the layout host where queued snackbar messages are displayed.
 */
export interface ComposeGeneratedSnackbarHostProps extends ComposeCommonProps {
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a compact assist action chip with label, optional icons, enabled state, and click handling.
 */
export interface ComposeGeneratedAssistChipProps extends ComposeCommonProps {
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Label content identifying the control or destination.
   */
  label: ComposeChildren;
  /**
   * Icon content rendered before the component's label or value.
   */
  leadingIcon?: ComposeChildren;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedAssistChipPropsOnClickOutput;
  /**
   * Icon content rendered after the component's label or value.
   */
  trailingIcon?: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a small status or count indicator rendered over associated content.
 */
export interface ComposeGeneratedBadgeProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a container that positions a badge relative to its primary content.
 */
export interface ComposeGeneratedBadgedBoxProps extends ComposeCommonProps {
  /**
   * Badge content positioned over or beside the associated destination content.
   */
  badge: ComposeChildren;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring the sheet surface and content used inside a dismissible navigation drawer.
 */
export interface ComposeGeneratedDismissibleDrawerSheetProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Tonal elevation used to distinguish the drawer sheet from its surroundings.
   */
  drawerTonalElevation?: number;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a navigation drawer that can be opened, closed, and dismissed with gestures.
 */
export interface ComposeGeneratedDismissibleNavigationDrawerProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Navigation content rendered inside the drawer or rail panel.
   */
  drawerContent: ComposeChildren;
  /**
   * Allows pointer gestures to open and close the navigation container.
   */
  gesturesEnabled?: boolean;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a horizontal separator between adjacent regions of content.
 */
export interface ComposeGeneratedDividerProps extends ComposeCommonProps {
  /**
   * Primary color used to render the component.
   */
  color?: ComposeColor;
  /**
   * Thickness of the divider line.
   */
  thickness?: number;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an anchored popup menu with explicit visibility, positioning, dismissal, and window behavior.
 */
export interface ComposeGeneratedDropdownMenuProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Controls whether the popup menu is currently visible.
   */
  expanded: boolean;
  /**
   * Position offset applied to the popup relative to its anchor.
   */
  offset?: number;
  /**
   * Called when user interaction requests that the popup or dialog close.
   */
  onDismissRequest: () => ComposeGeneratedDropdownMenuPropsOnDismissRequestOutput;
  /**
   * Popup-window focus, dismissal, clipping, and sizing behavior.
   */
  properties?: ComposeGeneratedDropdownMenuPropsProperties;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an elevated assist action chip with label, optional icons, enabled state, and click handling.
 */
export interface ComposeGeneratedElevatedAssistChipProps extends ComposeCommonProps {
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Label content identifying the control or destination.
   */
  label: ComposeChildren;
  /**
   * Icon content rendered before the component's label or value.
   */
  leadingIcon?: ComposeChildren;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedElevatedAssistChipPropsOnClickOutput;
  /**
   * Icon content rendered after the component's label or value.
   */
  trailingIcon?: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an elevated Material action button with configurable content, colors, padding, shape, and click handling.
 */
export interface ComposeGeneratedElevatedButtonProps extends ComposeCommonProps {
  /**
   * Background color of the component's container.
   */
  containerColor?: ComposeColor;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Padding between the container boundary and its child content.
   */
  contentPadding?: ComposePadding;
  /**
   * Background color used when the component is disabled.
   */
  disabledContainerColor?: ComposeColor;
  /**
   * Foreground color supplied to content while the component is disabled.
   */
  disabledContentColor?: ComposeColor;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedElevatedButtonPropsOnClickOutput;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an elevated Material card that groups related content above its surrounding surface.
 */
export interface ComposeGeneratedElevatedCardProps extends ComposeCommonProps {
  /**
   * Border drawn around the component's container.
   */
  border?: ComposeBorder;
  /**
   * Background color of the component's container.
   */
  containerColor?: ComposeColor;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Shadow elevation that visually raises the component above its surroundings.
   */
  elevation?: number;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an elevated selectable filter chip with controlled selection, icons, and click handling.
 */
export interface ComposeGeneratedElevatedFilterChipProps extends ComposeCommonProps {
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Label content identifying the control or destination.
   */
  label: ComposeChildren;
  /**
   * Icon content rendered before the component's label or value.
   */
  leadingIcon?: ComposeChildren;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedElevatedFilterChipPropsOnClickOutput;
  /**
   * Controlled state indicating whether this option or destination is selected.
   */
  selected: boolean;
  /**
   * Icon content rendered after the component's label or value.
   */
  trailingIcon?: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an elevated suggestion chip that presents a recommended action.
 */
export interface ComposeGeneratedElevatedSuggestionChipProps extends ComposeCommonProps {
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Icon resource or composed icon content displayed by the component.
   */
  icon?: ComposeChildren;
  /**
   * Label content identifying the control or destination.
   */
  label: ComposeChildren;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedElevatedSuggestionChipPropsOnClickOutput;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an extended floating action button for a prominent screen-level action.
 */
export interface ComposeGeneratedExtendedFloatingActionButtonProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedExtendedFloatingActionButtonPropsOnClickOutput;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a filled compact Material action button intended for an icon or custom content.
 */
export interface ComposeGeneratedFilledIconButtonProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Icon resource or composed icon content displayed by the component.
   */
  icon?: string;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedFilledIconButtonPropsOnClickOutput;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a filled icon button with controlled checked state.
 */
export interface ComposeGeneratedFilledIconToggleButtonProps extends ComposeCommonProps {
  /**
   * Controlled toggle state indicating whether the control is checked.
   */
  checked: boolean;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Called with the requested checked state after a toggle interaction.
   */
  onCheckedChange: (arg0: boolean) => void;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a tonal filled Material action button with configurable content, colors, padding, and click handling.
 */
export interface ComposeGeneratedFilledTonalButtonProps extends ComposeCommonProps {
  /**
   * Background color of the component's container.
   */
  containerColor?: ComposeColor;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Padding between the container boundary and its child content.
   */
  contentPadding?: ComposePadding;
  /**
   * Background color used when the component is disabled.
   */
  disabledContainerColor?: ComposeColor;
  /**
   * Foreground color supplied to content while the component is disabled.
   */
  disabledContentColor?: ComposeColor;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedFilledTonalButtonPropsOnClickOutput;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a tonal filled compact Material action button intended for an icon or custom content.
 */
export interface ComposeGeneratedFilledTonalIconButtonProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Icon resource or composed icon content displayed by the component.
   */
  icon?: string;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedFilledTonalIconButtonPropsOnClickOutput;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a tonal filled icon button with controlled checked state.
 */
export interface ComposeGeneratedFilledTonalIconToggleButtonProps extends ComposeCommonProps {
  /**
   * Controlled toggle state indicating whether the control is checked.
   */
  checked: boolean;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Called with the requested checked state after a toggle interaction.
   */
  onCheckedChange: (arg0: boolean) => void;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a selectable filter chip with controlled selection, optional icons, and click handling.
 */
export interface ComposeGeneratedFilterChipProps extends ComposeCommonProps {
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Label content identifying the control or destination.
   */
  label: ComposeChildren;
  /**
   * Icon content rendered before the component's label or value.
   */
  leadingIcon?: ComposeChildren;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedFilterChipPropsOnClickOutput;
  /**
   * Controlled state indicating whether this option or destination is selected.
   */
  selected: boolean;
  /**
   * Icon content rendered after the component's label or value.
   */
  trailingIcon?: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a floating action button for a prominent screen-level action.
 */
export interface ComposeGeneratedFloatingActionButtonProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedFloatingActionButtonPropsOnClickOutput;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a horizontal separator with configurable thickness and color.
 */
export interface ComposeGeneratedHorizontalDividerProps extends ComposeCommonProps {
  /**
   * Primary color used to render the component.
   */
  color?: ComposeColor;
  /**
   * Thickness of the divider line.
   */
  thickness?: number;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an icon button with controlled checked state.
 */
export interface ComposeGeneratedIconToggleButtonProps extends ComposeCommonProps {
  /**
   * Controlled toggle state indicating whether the control is checked.
   */
  checked: boolean;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Icon resource or composed icon content displayed by the component.
   */
  icon?: string;
  /**
   * Called with the requested checked state after a toggle interaction.
   */
  onCheckedChange: (arg0: boolean) => void;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an input chip representing user-supplied information, with avatar, label, icons, and selection state.
 */
export interface ComposeGeneratedInputChipProps extends ComposeCommonProps {
  /**
   * Avatar content shown at the leading edge of the input chip.
   */
  avatar?: ComposeChildren;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Label content identifying the control or destination.
   */
  label: ComposeChildren;
  /**
   * Icon content rendered before the component's label or value.
   */
  leadingIcon?: ComposeChildren;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedInputChipPropsOnClickOutput;
  /**
   * Controlled state indicating whether this option or destination is selected.
   */
  selected: boolean;
  /**
   * Icon content rendered after the component's label or value.
   */
  trailingIcon?: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a large floating action button for a prominent screen-level action.
 */
export interface ComposeGeneratedLargeFloatingActionButtonProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedLargeFloatingActionButtonPropsOnClickOutput;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a selectable tab that displays a leading icon beside its label.
 */
export interface ComposeGeneratedLeadingIconTabProps extends ComposeCommonProps {
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Icon resource or composed icon content displayed by the component.
   */
  icon: ComposeChildren;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedLeadingIconTabPropsOnClickOutput;
  /**
   * Controlled state indicating whether this option or destination is selected.
   */
  selected: boolean;
  /**
   * Text content displayed by the component.
   */
  text: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a Material list row with headline, supporting, overline, leading, and trailing content slots.
 */
export interface ComposeGeneratedListItemProps extends ComposeCommonProps {
  /**
   * Primary headline content of the list item.
   */
  headlineContent: ComposeChildren;
  /**
   * Content rendered before the list item's headline.
   */
  leadingContent?: ComposeChildren;
  /**
   * Optional overline content rendered above the list item's headline.
   */
  overlineContent?: ComposeChildren;
  /**
   * Physical shadow elevation cast by the surface.
   */
  shadowElevation?: number;
  /**
   * Secondary supporting content of the list item.
   */
  supportingContent?: ComposeChildren;
  /**
   * Tonal elevation used to adjust the surface color against its background.
   */
  tonalElevation?: number;
  /**
   * Content rendered after the list item's headline and supporting text.
   */
  trailingContent?: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring the sheet surface and content used inside a modal navigation drawer.
 */
export interface ComposeGeneratedModalDrawerSheetProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Tonal elevation used to distinguish the drawer sheet from its surroundings.
   */
  drawerTonalElevation?: number;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a modal navigation drawer with gesture handling and separate drawer and body content.
 */
export interface ComposeGeneratedModalNavigationDrawerProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Navigation content rendered inside the drawer or rail panel.
   */
  drawerContent: ComposeChildren;
  /**
   * Allows pointer gestures to open and close the navigation container.
   */
  gesturesEnabled?: boolean;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a modal wide navigation rail with collapsible header and navigation content.
 */
export interface ComposeGeneratedModalWideNavigationRailProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Top padding applied to the header while the wide rail is expanded.
   */
  expandedHeaderTopPadding?: number;
  /**
   * Optional header content rendered before the main navigation destinations.
   */
  header?: ComposeChildren;
  /**
   * Hides the configured content when the wide navigation rail collapses.
   */
  hideOnCollapse?: boolean;
  /**
   * Vertical spacing and placement strategy for child nodes.
   */
  verticalArrangement?: ComposeArrangement;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a bottom navigation bar that lays out destination content.
 */
export interface ComposeGeneratedNavigationBarProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Tonal elevation used to adjust the surface color against its background.
   */
  tonalElevation?: number;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a selectable destination row within a navigation drawer.
 */
export interface ComposeGeneratedNavigationDrawerItemProps extends ComposeCommonProps {
  /**
   * Badge content positioned over or beside the associated destination content.
   */
  badge?: ComposeChildren;
  /**
   * Icon resource or composed icon content displayed by the component.
   */
  icon?: ComposeChildren;
  /**
   * Label content identifying the control or destination.
   */
  label: ComposeChildren;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedNavigationDrawerItemPropsOnClickOutput;
  /**
   * Controlled state indicating whether this option or destination is selected.
   */
  selected: boolean;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a vertical navigation rail with an optional header and destination content.
 */
export interface ComposeGeneratedNavigationRailProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Optional header content rendered before the main navigation destinations.
   */
  header?: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a selectable destination item within a navigation rail.
 */
export interface ComposeGeneratedNavigationRailItemProps extends ComposeCommonProps {
  /**
   * Keeps the destination label visible even when the item is not selected.
   */
  alwaysShowLabel?: boolean;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Icon resource or composed icon content displayed by the component.
   */
  icon: ComposeChildren;
  /**
   * Label content identifying the control or destination.
   */
  label?: ComposeChildren;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedNavigationRailItemPropsOnClickOutput;
  /**
   * Controlled state indicating whether this option or destination is selected.
   */
  selected: boolean;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an outlined Material action button with configurable content, colors, padding, shape, and click handling.
 */
export interface ComposeGeneratedOutlinedButtonProps extends ComposeCommonProps {
  /**
   * Background color of the component's container.
   */
  containerColor?: ComposeColor;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Padding between the container boundary and its child content.
   */
  contentPadding?: ComposePadding;
  /**
   * Background color used when the component is disabled.
   */
  disabledContainerColor?: ComposeColor;
  /**
   * Foreground color supplied to content while the component is disabled.
   */
  disabledContentColor?: ComposeColor;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedOutlinedButtonPropsOnClickOutput;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an outlined Material card that groups related content within a bordered surface.
 */
export interface ComposeGeneratedOutlinedCardProps extends ComposeCommonProps {
  /**
   * Border drawn around the component's container.
   */
  border?: ComposeBorder;
  /**
   * Background color of the component's container.
   */
  containerColor?: ComposeColor;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Shadow elevation that visually raises the component above its surroundings.
   */
  elevation?: number;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an outlined compact Material action button intended for an icon or custom content.
 */
export interface ComposeGeneratedOutlinedIconButtonProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Icon resource or composed icon content displayed by the component.
   */
  icon?: string;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedOutlinedIconButtonPropsOnClickOutput;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an outlined icon button with controlled checked state.
 */
export interface ComposeGeneratedOutlinedIconToggleButtonProps extends ComposeCommonProps {
  /**
   * Controlled toggle state indicating whether the control is checked.
   */
  checked: boolean;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Called with the requested checked state after a toggle interaction.
   */
  onCheckedChange: (arg0: boolean) => void;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring the always-visible sheet surface and content of a permanent navigation drawer.
 */
export interface ComposeGeneratedPermanentDrawerSheetProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Tonal elevation used to distinguish the drawer sheet from its surroundings.
   */
  drawerTonalElevation?: number;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an always-visible navigation drawer with separate drawer and body content.
 */
export interface ComposeGeneratedPermanentNavigationDrawerProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Navigation content rendered inside the drawer or rail panel.
   */
  drawerContent: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a primary tab strip that scrolls when its tabs exceed the available width.
 */
export interface ComposeGeneratedPrimaryScrollableTabRowProps extends ComposeCommonProps {
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Divider content separating the tab strip from adjacent content.
   */
  divider?: ComposeChildren;
  /**
   * Horizontal padding before the first and after the last scrollable tab.
   */
  edgePadding?: number;
  /**
   * Custom indicator content showing selection or refresh state.
   */
  indicator?: ComposeChildren;
  /**
   * Zero-based index of the tab whose indicator is active.
   */
  selectedTabIndex: number;
  /**
   * Tab nodes rendered by the tab row.
   */
  tabs: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a primary fixed-width tab strip with selection indicator and divider slots.
 */
export interface ComposeGeneratedPrimaryTabRowProps extends ComposeCommonProps {
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Divider content separating the tab strip from adjacent content.
   */
  divider?: ComposeChildren;
  /**
   * Custom indicator content showing selection or refresh state.
   */
  indicator?: ComposeChildren;
  /**
   * Zero-based index of the tab whose indicator is active.
   */
  selectedTabIndex: number;
  /**
   * Tab nodes rendered by the tab row.
   */
  tabs: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a typography scope that supplies a text style to descendant text nodes.
 */
export interface ComposeGeneratedProvideTextStyleProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Typography or component style applied to the rendered content.
   */
  style?: ComposeTextStyle;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a pull-to-refresh container with controlled refresh state, indicator, and refresh callback.
 */
export interface ComposeGeneratedPullToRefreshBoxProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Alignment used to position child content within the available bounds.
   */
  contentAlignment?: ComposeAlignment;
  /**
   * Custom indicator content showing selection or refresh state.
   */
  indicator?: ComposeChildren;
  /**
   * Controlled state indicating that refresh work is currently active.
   */
  isRefreshing: boolean;
  /**
   * Called when the pull gesture requests a content refresh.
   */
  onRefresh: () => ComposeGeneratedPullToRefreshBoxPropsOnRefreshOutput;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a selectable Material radio control for choosing one option from a group.
 */
export interface ComposeGeneratedRadioButtonProps extends ComposeCommonProps {
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedRadioButtonPropsOnClickOutput;
  /**
   * Controlled state indicating whether this option or destination is selected.
   */
  selected: boolean;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a Material screen layout coordinating top bar, bottom bar, body, snackbar host, and floating action button.
 */
export interface ComposeGeneratedScaffoldProps extends ComposeCommonProps {
  /**
   * Content rendered in the scaffold's bottom bar slot.
   */
  bottomBar?: ComposeChildren;
  /**
   * Background color of the component's container.
   */
  containerColor?: ComposeColor;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Content rendered in the scaffold's floating action button slot.
   */
  floatingActionButton?: ComposeChildren;
  /**
   * Host content responsible for displaying scaffold snackbar messages.
   */
  snackbarHost?: ComposeChildren;
  /**
   * Content rendered in the scaffold's top app bar slot.
   */
  topBar?: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a secondary tab strip that scrolls when its tabs exceed the available width.
 */
export interface ComposeGeneratedSecondaryScrollableTabRowProps extends ComposeCommonProps {
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Divider content separating the tab strip from adjacent content.
   */
  divider?: ComposeChildren;
  /**
   * Horizontal padding before the first and after the last scrollable tab.
   */
  edgePadding?: number;
  /**
   * Custom indicator content showing selection or refresh state.
   */
  indicator?: ComposeChildren;
  /**
   * Zero-based index of the tab whose indicator is active.
   */
  selectedTabIndex: number;
  /**
   * Tab nodes rendered by the tab row.
   */
  tabs: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a secondary fixed-width tab strip with selection indicator and divider slots.
 */
export interface ComposeGeneratedSecondaryTabRowProps extends ComposeCommonProps {
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Divider content separating the tab strip from adjacent content.
   */
  divider?: ComposeChildren;
  /**
   * Custom indicator content showing selection or refresh state.
   */
  indicator?: ComposeChildren;
  /**
   * Zero-based index of the tab whose indicator is active.
   */
  selectedTabIndex: number;
  /**
   * Tab nodes rendered by the tab row.
   */
  tabs: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a compact bottom navigation bar that lays out destination content.
 */
export interface ComposeGeneratedShortNavigationBarProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a selectable destination item within a compact navigation bar.
 */
export interface ComposeGeneratedShortNavigationBarItemProps extends ComposeCommonProps {
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Icon resource or composed icon content displayed by the component.
   */
  icon: ComposeChildren;
  /**
   * Label content identifying the control or destination.
   */
  label: ComposeChildren;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedShortNavigationBarItemPropsOnClickOutput;
  /**
   * Controlled state indicating whether this option or destination is selected.
   */
  selected: boolean;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a small floating action button for a prominent screen-level action.
 */
export interface ComposeGeneratedSmallFloatingActionButtonProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedSmallFloatingActionButtonPropsOnClickOutput;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a transient message surface with optional action and dismissal content.
 */
export interface ComposeGeneratedSnackbarProps extends ComposeCommonProps {
  /**
   * Action content displayed alongside the snackbar message.
   */
  action?: ComposeChildren;
  /**
   * Places the snackbar action on a separate line when true.
   */
  actionOnNewLine?: boolean;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Optional snackbar control that dismisses the current message.
   */
  dismissAction?: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a suggestion chip that presents a recommended action.
 */
export interface ComposeGeneratedSuggestionChipProps extends ComposeCommonProps {
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Icon resource or composed icon content displayed by the component.
   */
  icon?: ComposeChildren;
  /**
   * Label content identifying the control or destination.
   */
  label: ComposeChildren;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedSuggestionChipPropsOnClickOutput;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a selectable tab with custom content and controlled selection state.
 */
export interface ComposeGeneratedTabProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedTabPropsOnClickOutput;
  /**
   * Controlled state indicating whether this option or destination is selected.
   */
  selected: boolean;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a low-emphasis text action button with configurable content, colors, padding, and click handling.
 */
export interface ComposeGeneratedTextButtonProps extends ComposeCommonProps {
  /**
   * Background color of the component's container.
   */
  containerColor?: ComposeColor;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Default foreground color supplied to child content.
   */
  contentColor?: ComposeColor;
  /**
   * Padding between the container boundary and its child content.
   */
  contentPadding?: ComposePadding;
  /**
   * Background color used when the component is disabled.
   */
  disabledContainerColor?: ComposeColor;
  /**
   * Foreground color supplied to content while the component is disabled.
   */
  disabledContentColor?: ComposeColor;
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedTextButtonPropsOnClickOutput;
  /**
   * Shape used for the component's container outline.
   */
  shape?: ComposeShape;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a Material time-picker dialog with title, controls, confirmation, dismissal, and mode switching slots.
 */
export interface ComposeGeneratedTimePickerDialogProps extends ComposeCommonProps {
  /**
   * Confirmation action displayed by the time-picker dialog.
   */
  confirmButton: ComposeChildren;
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Optional dismissal action displayed by the time-picker dialog.
   */
  dismissButton?: ComposeChildren;
  /**
   * Optional control for switching the time picker's input mode.
   */
  modeToggleButton?: ComposeChildren;
  /**
   * Called when user interaction requests that the popup or dialog close.
   */
  onDismissRequest: () => ComposeGeneratedTimePickerDialogPropsOnDismissRequestOutput;
  /**
   * Title content displayed at the top of the time-picker dialog.
   */
  title: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a vertical separator with configurable thickness and color.
 */
export interface ComposeGeneratedVerticalDividerProps extends ComposeCommonProps {
  /**
   * Primary color used to render the component.
   */
  color?: ComposeColor;
  /**
   * Thickness of the divider line.
   */
  thickness?: number;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a visual handle indicating that a surface can be dragged vertically.
 */
export interface ComposeGeneratedVerticalDragHandleProps extends ComposeCommonProps {
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an expanded vertical navigation rail with header and destination content.
 */
export interface ComposeGeneratedWideNavigationRailProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Optional header content rendered before the main navigation destinations.
   */
  header?: ComposeChildren;
  /**
   * Vertical spacing and placement strategy for child nodes.
   */
  verticalArrangement?: ComposeArrangement;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a selectable destination item that adapts to an expanded or collapsed wide navigation rail.
 */
export interface ComposeGeneratedWideNavigationRailItemProps extends ComposeCommonProps {
  /**
   * Controls whether the component accepts user interaction.
   */
  enabled?: boolean;
  /**
   * Icon resource or composed icon content displayed by the component.
   */
  icon: ComposeChildren;
  /**
   * Label content identifying the control or destination.
   */
  label: ComposeChildren;
  /**
   * Called when the user activates the component.
   */
  onClick: () => ComposeGeneratedWideNavigationRailItemPropsOnClickOutput;
  /**
   * Indicates whether the containing wide navigation rail is expanded.
   */
  railExpanded: boolean;
  /**
   * Controlled state indicating whether this option or destination is selected.
   */
  selected: boolean;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a stacking layout that exposes its available constraints while aligning child content.
 */
export interface ComposeGeneratedBoxWithConstraintsProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Alignment used to position child content within the available bounds.
   */
  contentAlignment?: ComposeAlignment;
  /**
   * Passes the parent's minimum constraints through to child content when true.
   */
  propagateMinConstraints?: boolean;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a lightweight text node with typography, wrapping, and overflow controls.
 */
export interface ComposeGeneratedBasicTextProps extends ComposeCommonProps {
  /**
   * Font family used to render the text.
   */
  fontFamily?: string;
  /**
   * Text size used for the rendered content.
   */
  fontSize?: number;
  /**
   * Maximum number of text lines that may be displayed.
   */
  maxLines?: number;
  /**
   * Behavior used when text exceeds its available layout space.
   */
  overflow?: ComposeTextOverflow;
  /**
   * Allows text to wrap at soft line-breaking opportunities.
   */
  softWrap?: boolean;
  /**
   * Typography or component style applied to the rendered content.
   */
  style?: ComposeTextStyle;
  /**
   * Text content displayed by the component.
   */
  text: string;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a scope that prevents text selection within its child content.
 */
export interface ComposeGeneratedDisableSelectionProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring an image node supporting local, URI, URL, resource, and icon sources with scaling and accessibility controls.
 */
export interface ComposeGeneratedImageProps extends ComposeCommonProps {
  /**
   * Opacity applied to the rendered node, from transparent to fully opaque.
   */
  alpha?: number;
  /**
   * Alignment used to position child content within the available bounds.
   */
  contentAlignment?: ComposeAlignment;
  /**
   * Accessible description announced for non-text visual content.
   */
  contentDescription?: string;
  /**
   * Scaling strategy used to fit the image into its layout bounds.
   */
  contentScale?: ComposeContentScale;
  /**
   * File URI identifying the image source.
   */
  fileUri?: string;
  /**
   * Icon resource or composed icon content displayed by the component.
   */
  icon?: string;
  /**
   * Named image or icon resource used as the visual source.
   */
  name?: string;
  /**
   * Local filesystem path identifying the image source.
   */
  path?: string;
  /**
   * Source identifier used to load the image.
   */
  src?: string;
  /**
   * URI identifying the image source.
   */
  uri?: string;
  /**
   * Network URL identifying the image source.
   */
  url?: string;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a scope that enables text selection across its child content.
 */
export interface ComposeGeneratedSelectionContainerProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the component.
   */
  content?: ComposeChildren;
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
}

/**
 * Properties configuring a drawing surface that executes an ordered list of canvas commands.
 */
export interface ComposeGeneratedCanvasProps extends ComposeCommonProps {
  /**
   * Drawing order relative to sibling nodes; larger values render above smaller values.
   */
  zIndex?: number;
  /**
   * Ordered drawing commands executed by the canvas.
   */
  commands?: ComposeCanvasCommand[];
}

/**
 * Factories available for constructing every supported Material 3 and Foundation UI node.
 */
export interface ComposeMaterial3GeneratedUiFactoryRegistry {
  /**
   * Factory used to construct Column nodes from their typed properties.
   */
  Column: ComposeNodeFactory<ComposeGeneratedColumnProps>;
  /**
   * Factory used to construct Row nodes from their typed properties.
   */
  Row: ComposeNodeFactory<ComposeGeneratedRowProps>;
  /**
   * Factory used to construct Box nodes from their typed properties.
   */
  Box: ComposeNodeFactory<ComposeGeneratedBoxProps>;
  /**
   * Factory used to construct Spacer nodes from their typed properties.
   */
  Spacer: ComposeNodeFactory<ComposeGeneratedSpacerProps>;
  /**
   * Factory used to construct Lazy Column nodes from their typed properties.
   */
  LazyColumn: ComposeNodeFactory<ComposeGeneratedLazyColumnProps>;
  /**
   * Factory used to construct Lazy Row nodes from their typed properties.
   */
  LazyRow: ComposeNodeFactory<ComposeGeneratedLazyRowProps>;
  /**
   * Factory used to construct Text nodes from their typed properties.
   */
  Text: ComposeNodeFactory<ComposeGeneratedTextProps>;
  /**
   * Factory used to construct Text Field nodes from their typed properties.
   */
  TextField: ComposeNodeFactory<ComposeGeneratedTextFieldProps>;
  /**
   * Factory used to construct Switch nodes from their typed properties.
   */
  Switch: ComposeNodeFactory<ComposeGeneratedSwitchProps>;
  /**
   * Factory used to construct Checkbox nodes from their typed properties.
   */
  Checkbox: ComposeNodeFactory<ComposeGeneratedCheckboxProps>;
  /**
   * Factory used to construct Button nodes from their typed properties.
   */
  Button: ComposeNodeFactory<ComposeGeneratedButtonProps>;
  /**
   * Factory used to construct Icon Button nodes from their typed properties.
   */
  IconButton: ComposeNodeFactory<ComposeGeneratedIconButtonProps>;
  /**
   * Factory used to construct Card nodes from their typed properties.
   */
  Card: ComposeNodeFactory<ComposeGeneratedCardProps>;
  /**
   * Factory used to construct Material Theme nodes from their typed properties.
   */
  MaterialTheme: ComposeNodeFactory<ComposeGeneratedMaterialThemeProps>;
  /**
   * Factory used to construct Surface nodes from their typed properties.
   */
  Surface: ComposeNodeFactory<ComposeGeneratedSurfaceProps>;
  /**
   * Factory used to construct Icon nodes from their typed properties.
   */
  Icon: ComposeNodeFactory<ComposeGeneratedIconProps>;
  /**
   * Factory used to construct Linear Progress Indicator nodes from their typed properties.
   */
  LinearProgressIndicator: ComposeNodeFactory<ComposeGeneratedLinearProgressIndicatorProps>;
  /**
   * Factory used to construct Circular Progress Indicator nodes from their typed properties.
   */
  CircularProgressIndicator: ComposeNodeFactory<ComposeGeneratedCircularProgressIndicatorProps>;
  /**
   * Factory used to construct Snackbar Host nodes from their typed properties.
   */
  SnackbarHost: ComposeNodeFactory<ComposeGeneratedSnackbarHostProps>;
  /**
   * Factory used to construct Assist Chip nodes from their typed properties.
   */
  AssistChip: ComposeNodeFactory<ComposeGeneratedAssistChipProps>;
  /**
   * Factory used to construct Badge nodes from their typed properties.
   */
  Badge: ComposeNodeFactory<ComposeGeneratedBadgeProps>;
  /**
   * Factory used to construct Badged Box nodes from their typed properties.
   */
  BadgedBox: ComposeNodeFactory<ComposeGeneratedBadgedBoxProps>;
  /**
   * Factory used to construct Dismissible Drawer Sheet nodes from their typed properties.
   */
  DismissibleDrawerSheet: ComposeNodeFactory<ComposeGeneratedDismissibleDrawerSheetProps>;
  /**
   * Factory used to construct Dismissible Navigation Drawer nodes from their typed properties.
   */
  DismissibleNavigationDrawer: ComposeNodeFactory<ComposeGeneratedDismissibleNavigationDrawerProps>;
  /**
   * Factory used to construct Divider nodes from their typed properties.
   */
  Divider: ComposeNodeFactory<ComposeGeneratedDividerProps>;
  /**
   * Factory used to construct Dropdown Menu nodes from their typed properties.
   */
  DropdownMenu: ComposeNodeFactory<ComposeGeneratedDropdownMenuProps>;
  /**
   * Factory used to construct Elevated Assist Chip nodes from their typed properties.
   */
  ElevatedAssistChip: ComposeNodeFactory<ComposeGeneratedElevatedAssistChipProps>;
  /**
   * Factory used to construct Elevated Button nodes from their typed properties.
   */
  ElevatedButton: ComposeNodeFactory<ComposeGeneratedElevatedButtonProps>;
  /**
   * Factory used to construct Elevated Card nodes from their typed properties.
   */
  ElevatedCard: ComposeNodeFactory<ComposeGeneratedElevatedCardProps>;
  /**
   * Factory used to construct Elevated Filter Chip nodes from their typed properties.
   */
  ElevatedFilterChip: ComposeNodeFactory<ComposeGeneratedElevatedFilterChipProps>;
  /**
   * Factory used to construct Elevated Suggestion Chip nodes from their typed properties.
   */
  ElevatedSuggestionChip: ComposeNodeFactory<ComposeGeneratedElevatedSuggestionChipProps>;
  /**
   * Factory used to construct Extended Floating Action Button nodes from their typed properties.
   */
  ExtendedFloatingActionButton: ComposeNodeFactory<ComposeGeneratedExtendedFloatingActionButtonProps>;
  /**
   * Factory used to construct Filled Icon Button nodes from their typed properties.
   */
  FilledIconButton: ComposeNodeFactory<ComposeGeneratedFilledIconButtonProps>;
  /**
   * Factory used to construct Filled Icon Toggle Button nodes from their typed properties.
   */
  FilledIconToggleButton: ComposeNodeFactory<ComposeGeneratedFilledIconToggleButtonProps>;
  /**
   * Factory used to construct Filled Tonal Button nodes from their typed properties.
   */
  FilledTonalButton: ComposeNodeFactory<ComposeGeneratedFilledTonalButtonProps>;
  /**
   * Factory used to construct Filled Tonal Icon Button nodes from their typed properties.
   */
  FilledTonalIconButton: ComposeNodeFactory<ComposeGeneratedFilledTonalIconButtonProps>;
  /**
   * Factory used to construct Filled Tonal Icon Toggle Button nodes from their typed properties.
   */
  FilledTonalIconToggleButton: ComposeNodeFactory<ComposeGeneratedFilledTonalIconToggleButtonProps>;
  /**
   * Factory used to construct Filter Chip nodes from their typed properties.
   */
  FilterChip: ComposeNodeFactory<ComposeGeneratedFilterChipProps>;
  /**
   * Factory used to construct Floating Action Button nodes from their typed properties.
   */
  FloatingActionButton: ComposeNodeFactory<ComposeGeneratedFloatingActionButtonProps>;
  /**
   * Factory used to construct Horizontal Divider nodes from their typed properties.
   */
  HorizontalDivider: ComposeNodeFactory<ComposeGeneratedHorizontalDividerProps>;
  /**
   * Factory used to construct Icon Toggle Button nodes from their typed properties.
   */
  IconToggleButton: ComposeNodeFactory<ComposeGeneratedIconToggleButtonProps>;
  /**
   * Factory used to construct Input Chip nodes from their typed properties.
   */
  InputChip: ComposeNodeFactory<ComposeGeneratedInputChipProps>;
  /**
   * Factory used to construct Large Floating Action Button nodes from their typed properties.
   */
  LargeFloatingActionButton: ComposeNodeFactory<ComposeGeneratedLargeFloatingActionButtonProps>;
  /**
   * Factory used to construct Leading Icon Tab nodes from their typed properties.
   */
  LeadingIconTab: ComposeNodeFactory<ComposeGeneratedLeadingIconTabProps>;
  /**
   * Factory used to construct List Item nodes from their typed properties.
   */
  ListItem: ComposeNodeFactory<ComposeGeneratedListItemProps>;
  /**
   * Factory used to construct Modal Drawer Sheet nodes from their typed properties.
   */
  ModalDrawerSheet: ComposeNodeFactory<ComposeGeneratedModalDrawerSheetProps>;
  /**
   * Factory used to construct Modal Navigation Drawer nodes from their typed properties.
   */
  ModalNavigationDrawer: ComposeNodeFactory<ComposeGeneratedModalNavigationDrawerProps>;
  /**
   * Factory used to construct Modal Wide Navigation Rail nodes from their typed properties.
   */
  ModalWideNavigationRail: ComposeNodeFactory<ComposeGeneratedModalWideNavigationRailProps>;
  /**
   * Factory used to construct Navigation Bar nodes from their typed properties.
   */
  NavigationBar: ComposeNodeFactory<ComposeGeneratedNavigationBarProps>;
  /**
   * Factory used to construct Navigation Drawer Item nodes from their typed properties.
   */
  NavigationDrawerItem: ComposeNodeFactory<ComposeGeneratedNavigationDrawerItemProps>;
  /**
   * Factory used to construct Navigation Rail nodes from their typed properties.
   */
  NavigationRail: ComposeNodeFactory<ComposeGeneratedNavigationRailProps>;
  /**
   * Factory used to construct Navigation Rail Item nodes from their typed properties.
   */
  NavigationRailItem: ComposeNodeFactory<ComposeGeneratedNavigationRailItemProps>;
  /**
   * Factory used to construct Outlined Button nodes from their typed properties.
   */
  OutlinedButton: ComposeNodeFactory<ComposeGeneratedOutlinedButtonProps>;
  /**
   * Factory used to construct Outlined Card nodes from their typed properties.
   */
  OutlinedCard: ComposeNodeFactory<ComposeGeneratedOutlinedCardProps>;
  /**
   * Factory used to construct Outlined Icon Button nodes from their typed properties.
   */
  OutlinedIconButton: ComposeNodeFactory<ComposeGeneratedOutlinedIconButtonProps>;
  /**
   * Factory used to construct Outlined Icon Toggle Button nodes from their typed properties.
   */
  OutlinedIconToggleButton: ComposeNodeFactory<ComposeGeneratedOutlinedIconToggleButtonProps>;
  /**
   * Factory used to construct Permanent Drawer Sheet nodes from their typed properties.
   */
  PermanentDrawerSheet: ComposeNodeFactory<ComposeGeneratedPermanentDrawerSheetProps>;
  /**
   * Factory used to construct Permanent Navigation Drawer nodes from their typed properties.
   */
  PermanentNavigationDrawer: ComposeNodeFactory<ComposeGeneratedPermanentNavigationDrawerProps>;
  /**
   * Factory used to construct Primary Scrollable Tab Row nodes from their typed properties.
   */
  PrimaryScrollableTabRow: ComposeNodeFactory<ComposeGeneratedPrimaryScrollableTabRowProps>;
  /**
   * Factory used to construct Primary Tab Row nodes from their typed properties.
   */
  PrimaryTabRow: ComposeNodeFactory<ComposeGeneratedPrimaryTabRowProps>;
  /**
   * Factory used to construct Provide Text Style nodes from their typed properties.
   */
  ProvideTextStyle: ComposeNodeFactory<ComposeGeneratedProvideTextStyleProps>;
  /**
   * Factory used to construct Pull To Refresh Box nodes from their typed properties.
   */
  PullToRefreshBox: ComposeNodeFactory<ComposeGeneratedPullToRefreshBoxProps>;
  /**
   * Factory used to construct Radio Button nodes from their typed properties.
   */
  RadioButton: ComposeNodeFactory<ComposeGeneratedRadioButtonProps>;
  /**
   * Factory used to construct Scaffold nodes from their typed properties.
   */
  Scaffold: ComposeNodeFactory<ComposeGeneratedScaffoldProps>;
  /**
   * Factory used to construct Secondary Scrollable Tab Row nodes from their typed properties.
   */
  SecondaryScrollableTabRow: ComposeNodeFactory<ComposeGeneratedSecondaryScrollableTabRowProps>;
  /**
   * Factory used to construct Secondary Tab Row nodes from their typed properties.
   */
  SecondaryTabRow: ComposeNodeFactory<ComposeGeneratedSecondaryTabRowProps>;
  /**
   * Factory used to construct Short Navigation Bar nodes from their typed properties.
   */
  ShortNavigationBar: ComposeNodeFactory<ComposeGeneratedShortNavigationBarProps>;
  /**
   * Factory used to construct Short Navigation Bar Item nodes from their typed properties.
   */
  ShortNavigationBarItem: ComposeNodeFactory<ComposeGeneratedShortNavigationBarItemProps>;
  /**
   * Factory used to construct Small Floating Action Button nodes from their typed properties.
   */
  SmallFloatingActionButton: ComposeNodeFactory<ComposeGeneratedSmallFloatingActionButtonProps>;
  /**
   * Factory used to construct Snackbar nodes from their typed properties.
   */
  Snackbar: ComposeNodeFactory<ComposeGeneratedSnackbarProps>;
  /**
   * Factory used to construct Suggestion Chip nodes from their typed properties.
   */
  SuggestionChip: ComposeNodeFactory<ComposeGeneratedSuggestionChipProps>;
  /**
   * Factory used to construct Tab nodes from their typed properties.
   */
  Tab: ComposeNodeFactory<ComposeGeneratedTabProps>;
  /**
   * Factory used to construct Text Button nodes from their typed properties.
   */
  TextButton: ComposeNodeFactory<ComposeGeneratedTextButtonProps>;
  /**
   * Factory used to construct Time Picker Dialog nodes from their typed properties.
   */
  TimePickerDialog: ComposeNodeFactory<ComposeGeneratedTimePickerDialogProps>;
  /**
   * Factory used to construct Vertical Divider nodes from their typed properties.
   */
  VerticalDivider: ComposeNodeFactory<ComposeGeneratedVerticalDividerProps>;
  /**
   * Factory used to construct Vertical Drag Handle nodes from their typed properties.
   */
  VerticalDragHandle: ComposeNodeFactory<ComposeGeneratedVerticalDragHandleProps>;
  /**
   * Factory used to construct Wide Navigation Rail nodes from their typed properties.
   */
  WideNavigationRail: ComposeNodeFactory<ComposeGeneratedWideNavigationRailProps>;
  /**
   * Factory used to construct Wide Navigation Rail Item nodes from their typed properties.
   */
  WideNavigationRailItem: ComposeNodeFactory<ComposeGeneratedWideNavigationRailItemProps>;
  /**
   * Factory used to construct Box With Constraints nodes from their typed properties.
   */
  BoxWithConstraints: ComposeNodeFactory<ComposeGeneratedBoxWithConstraintsProps>;
  /**
   * Factory used to construct Basic Text nodes from their typed properties.
   */
  BasicText: ComposeNodeFactory<ComposeGeneratedBasicTextProps>;
  /**
   * Factory used to construct Disable Selection nodes from their typed properties.
   */
  DisableSelection: ComposeNodeFactory<ComposeGeneratedDisableSelectionProps>;
  /**
   * Factory used to construct Image nodes from their typed properties.
   */
  Image: ComposeNodeFactory<ComposeGeneratedImageProps>;
  /**
   * Factory used to construct Selection Container nodes from their typed properties.
   */
  SelectionContainer: ComposeNodeFactory<ComposeGeneratedSelectionContainerProps>;
  /**
   * Factory used to construct Canvas nodes from their typed properties.
   */
  Canvas: ComposeNodeFactory<ComposeGeneratedCanvasProps>;
}
