// Generated from operit-plugin-sdk Rust declarations.

import type { ComposeMaterial3GeneratedUiFactoryRegistry } from "./compose-dsl.material3.generated";

/**
 * Options for deriving a theme color token with adjusted opacity.
 */
export interface ComposeColorTokenMethodsCopyOptions {
  /**
   * Opacity multiplier applied to the derived token.
   */
  alpha: number;
}

/**
 * Gradient direction supported by a canvas brush.
 */
export type ComposeCanvasBrushType = "VerticalGradient";

/**
 * Completion mode returned by the primary-click handler.
 */
export type ComposeModifierCombinedClickableOptionsOnClickOutput = void | Promise<void>;

/**
 * Completion mode returned by the long-click handler.
 */
export type ComposeModifierCombinedClickableOptionsOnLongClickOutput = void | Promise<void>;

/**
 * Completion mode returned by the double-click handler.
 */
export type ComposeModifierCombinedClickableOptionsOnDoubleClickOutput = void | Promise<void>;

/**
 * Completion mode returned after a pointer press is recognized.
 */
export type ComposeModifierTapGesturesOptionsOnPressOutput = void | Promise<void>;

/**
 * Completion mode returned after a tap is recognized.
 */
export type ComposeModifierTapGesturesOptionsOnTapOutput = void | Promise<void>;

/**
 * Completion mode returned after a double tap is recognized.
 */
export type ComposeModifierTapGesturesOptionsOnDoubleTapOutput = void | Promise<void>;

/**
 * Completion mode returned after a long press is recognized.
 */
export type ComposeModifierTapGesturesOptionsOnLongPressOutput = void | Promise<void>;

/**
 * Completion mode returned when a drag begins.
 */
export type ComposeModifierDragGesturesOptionsOnDragStartOutput = void | Promise<void>;

/**
 * Completion mode returned for each drag update.
 */
export type ComposeModifierDragGesturesOptionsOnDragOutput = void | Promise<void>;

/**
 * Completion mode returned when a drag ends normally.
 */
export type ComposeModifierDragGesturesOptionsOnDragEndOutput = void | Promise<void>;

/**
 * Completion mode returned when a drag is cancelled.
 */
export type ComposeModifierDragGesturesOptionsOnDragCancelOutput = void | Promise<void>;

/**
 * Completion mode returned for a multi-touch transform update.
 */
export type ComposeModifierTransformGesturesOptionsOnGestureOutput = void | Promise<void>;

/**
 * Lifecycle transition reported by an embedded WebView.
 */
export type ComposeWebViewLifecycleEventType = "Created" | "Disposed" | "PageCommitVisible" | "RenderProcessGone";

/**
 * Discriminator for allowing a WebView navigation unchanged.
 */
export type ComposeWebViewNavigationDecisionVariant1Action = "allow";

/**
 * Navigation decision that permits the original request.
 */
export interface ComposeWebViewNavigationDecisionVariant1 {
  /**
   * Selects the allow branch of the navigation decision.
   */
  action: ComposeWebViewNavigationDecisionVariant1Action;
}

/**
 * Discriminator for cancelling a WebView navigation.
 */
export type ComposeWebViewNavigationDecisionVariant2Action = "cancel";

/**
 * Navigation decision that prevents the requested page from loading.
 */
export interface ComposeWebViewNavigationDecisionVariant2 {
  /**
   * Selects the cancel branch of the navigation decision.
   */
  action: ComposeWebViewNavigationDecisionVariant2Action;
}

/**
 * Discriminator for replacing a WebView navigation request.
 */
export type ComposeWebViewNavigationDecisionVariant3Action = "rewrite";

/**
 * Navigation decision that redirects the WebView to a replacement request.
 */
export interface ComposeWebViewNavigationDecisionVariant3 {
  /**
   * Selects the rewrite branch of the navigation decision.
   */
  action: ComposeWebViewNavigationDecisionVariant3Action;
  /**
   * Replacement URL loaded by the WebView.
   */
  url: string;
  /**
   * HTTP headers attached to the replacement request.
   */
  headers?: Record<string, string>;
}

/**
 * Discriminator for handing navigation to an external application.
 */
export type ComposeWebViewNavigationDecisionVariant4Action = "external";

/**
 * Navigation decision that delegates a target to the host platform.
 */
export interface ComposeWebViewNavigationDecisionVariant4 {
  /**
   * Selects the external-navigation branch.
   */
  action: ComposeWebViewNavigationDecisionVariant4Action;
  /**
   * Target to open externally, or the original request when omitted.
   */
  url?: string;
}

/**
 * Synthetic WebView resource response backed by decoded text.
 */
export interface ComposeWebViewResourceResponseVariant1 {
  /**
   * Media type reported for the response body.
   */
  mimeType?: string;
  /**
   * Character encoding used to convert the text body to bytes.
   */
  encoding?: string;
  /**
   * HTTP status code exposed to the page.
   */
  statusCode?: number;
  /**
   * HTTP reason phrase paired with the status code.
   */
  reasonPhrase?: string;
  /**
   * Response headers exposed to the page.
   */
  headers?: Record<string, string>;
  /**
   * Decoded text used as the response body.
   */
  text: string;
  /**
   * Uninhabited marker that excludes a base64 body from this branch.
   */
  base64?: never;
  /**
   * Uninhabited marker that excludes a file body from this branch.
   */
  filePath?: never;
}

/**
 * Synthetic WebView resource response backed by base64-encoded bytes.
 */
export interface ComposeWebViewResourceResponseVariant2 {
  /**
   * Media type reported for the decoded response bytes.
   */
  mimeType?: string;
  /**
   * Character encoding metadata reported with the response.
   */
  encoding?: string;
  /**
   * HTTP status code exposed to the page.
   */
  statusCode?: number;
  /**
   * HTTP reason phrase paired with the status code.
   */
  reasonPhrase?: string;
  /**
   * Response headers exposed to the page.
   */
  headers?: Record<string, string>;
  /**
   * Base64-encoded bytes used as the response body.
   */
  base64: string;
  /**
   * Uninhabited marker that excludes a text body from this branch.
   */
  text?: never;
  /**
   * Uninhabited marker that excludes a file body from this branch.
   */
  filePath?: never;
}

/**
 * Synthetic WebView resource response streamed from a local file.
 */
export interface ComposeWebViewResourceResponseVariant3 {
  /**
   * Media type reported for the file contents.
   */
  mimeType?: string;
  /**
   * Character encoding metadata reported with the response.
   */
  encoding?: string;
  /**
   * HTTP status code exposed to the page.
   */
  statusCode?: number;
  /**
   * HTTP reason phrase paired with the status code.
   */
  reasonPhrase?: string;
  /**
   * Response headers exposed to the page.
   */
  headers?: Record<string, string>;
  /**
   * Local path whose contents become the response body.
   */
  filePath: string;
  /**
   * Uninhabited marker that excludes a text body from this branch.
   */
  text?: never;
  /**
   * Uninhabited marker that excludes a base64 body from this branch.
   */
  base64?: never;
}

/**
 * Discriminator for allowing a resource request unchanged.
 */
export type ComposeWebViewResourceDecisionVariant1Action = "allow";

/**
 * Resource interception decision that permits the original request.
 */
export interface ComposeWebViewResourceDecisionVariant1 {
  /**
   * Selects the allow branch of the resource decision.
   */
  action: ComposeWebViewResourceDecisionVariant1Action;
}

/**
 * Discriminator for blocking a resource request.
 */
export type ComposeWebViewResourceDecisionVariant2Action = "block";

/**
 * Resource interception decision that suppresses a request.
 */
export interface ComposeWebViewResourceDecisionVariant2 {
  /**
   * Selects the block branch of the resource decision.
   */
  action: ComposeWebViewResourceDecisionVariant2Action;
}

/**
 * Discriminator for replacing a resource request.
 */
export type ComposeWebViewResourceDecisionVariant3Action = "rewrite";

/**
 * Resource interception decision that redirects a request.
 */
export interface ComposeWebViewResourceDecisionVariant3 {
  /**
   * Selects the rewrite branch of the resource decision.
   */
  action: ComposeWebViewResourceDecisionVariant3Action;
  /**
   * Replacement URL used for the intercepted resource.
   */
  url: string;
  /**
   * HTTP headers attached to the replacement request.
   */
  headers?: Record<string, string>;
}

/**
 * Discriminator for supplying a synthetic resource response.
 */
export type ComposeWebViewResourceDecisionVariant4Action = "respond";

/**
 * Resource interception decision that returns plugin-provided content.
 */
export interface ComposeWebViewResourceDecisionVariant4 {
  /**
   * Selects the synthetic-response branch.
   */
  action: ComposeWebViewResourceDecisionVariant4Action;
  /**
   * Response body and metadata returned to the page.
   */
  response: ComposeWebViewResourceResponse;
}

/**
 * Value returned by a JavaScript interface method exposed to WebView content.
 */
export type ComposeWebViewJavascriptInterfaceMethodOutput = unknown | Promise<unknown>;

/**
 * Discriminator for a straight-line canvas command.
 */
export type ComposeCanvasLineCommandType = "Line";

/**
 * Discriminator for an axis-aligned rectangle canvas command.
 */
export type ComposeCanvasRectCommandType = "Rect";

/**
 * Discriminator for a rounded-rectangle canvas command.
 */
export type ComposeCanvasRoundRectCommandType = "RoundRect";

/**
 * Discriminator for a circular canvas command.
 */
export type ComposeCanvasCircleCommandType = "Circle";

/**
 * Discriminator for a measured text canvas command.
 */
export type ComposeCanvasTextCommandType = "Text";

/**
 * Discriminator for starting a path contour at a point.
 */
export type ComposeCanvasMoveToOpType = "MoveTo";

/**
 * Discriminator for appending a straight segment to a path.
 */
export type ComposeCanvasLineToOpType = "LineTo";

/**
 * Discriminator for appending a cubic Bezier segment to a path.
 */
export type ComposeCanvasCubicToOpType = "CubicTo";

/**
 * Discriminator for appending a quadratic Bezier segment to a path.
 */
export type ComposeCanvasQuadToOpType = "QuadTo";

/**
 * Discriminator for closing the current path contour.
 */
export type ComposeCanvasCloseOpType = "Close";

/**
 * Discriminator for rendering a sequence of path operations.
 */
export type ComposeCanvasDrawPathCommandType = "DrawPath";

/**
 * Discriminator for a draw-scope rounded rectangle command.
 */
export type ComposeCanvasDrawRoundRectCommandType = "DrawRoundRect";

/**
 * Discriminator for a draw-scope text command.
 */
export type ComposeCanvasDrawTextCommandType = "DrawText";

/**
 * Discriminator for a draw-scope Material icon command.
 */
export type ComposeCanvasDrawIconCommandType = "DrawIcon";

/**
 * Completion mode returned by a modifier click handler.
 */
export type ComposeModifierProxyClickableOnClickOutput = void | Promise<void>;

/**
 * Completion mode returned by a modifier size-change handler.
 */
export type ComposeModifierProxyOnSizeChangedOnSizeChangedOutput = void | Promise<void>;

/**
 * Completion mode returned by a global-position handler.
 */
export type ComposeModifierProxyOnGloballyPositionedOnGloballyPositionedOutput = void | Promise<void>;

/**
 * Completion mode returned when a composed node finishes loading.
 */
export type ComposeCommonPropsOnLoadOutput = void | Promise<void>;

/**
 * Padding accepted as either one uniform inset or axis-specific insets.
 */
export type ComposeCommonPropsPadding = number | ComposePadding;

/**
 * Completion mode returned by a row click handler.
 */
export type RowPropsOnClickOutput = void | Promise<void>;

/**
 * Label content accepted by a text field.
 */
export type TextFieldPropsLabel = string | ComposeChildren;

/**
 * Placeholder content accepted by a text field.
 */
export type TextFieldPropsPlaceholder = string | ComposeChildren;

/**
 * Completion mode returned by a button click handler.
 */
export type ButtonPropsOnClickOutput = void | Promise<void>;

/**
 * Completion mode returned by an icon-button click handler.
 */
export type IconButtonPropsOnClickOutput = void | Promise<void>;

/**
 * Completion mode returned by a clickable surface.
 */
export type SurfacePropsOnClickOutput = void | Promise<void>;

/**
 * Completion mode returned when a WebView begins loading a page.
 */
export type WebViewPropsOnPageStartedOutput = void | Promise<void>;

/**
 * Completion mode returned when a WebView finishes loading a page.
 */
export type WebViewPropsOnPageFinishedOutput = void | Promise<void>;

/**
 * Completion mode returned after a WebView loading error is reported.
 */
export type WebViewPropsOnReceivedErrorOutput = void | Promise<void>;

/**
 * Completion mode returned after an HTTP error response is reported.
 */
export type WebViewPropsOnReceivedHttpErrorOutput = void | Promise<void>;

/**
 * Completion mode returned after a TLS certificate error is reported.
 */
export type WebViewPropsOnReceivedSslErrorOutput = void | Promise<void>;

/**
 * Completion mode returned when WebView content starts a download.
 */
export type WebViewPropsOnDownloadStartOutput = void | Promise<void>;

/**
 * Completion mode returned after a page console message is delivered.
 */
export type WebViewPropsOnConsoleMessageOutput = void | Promise<void>;

/**
 * Completion mode returned when the WebView URL changes.
 */
export type WebViewPropsOnUrlChangedOutput = void | Promise<void>;

/**
 * Completion mode returned when page loading progress changes.
 */
export type WebViewPropsOnProgressChangedOutput = void | Promise<void>;

/**
 * Completion mode returned after a WebView state snapshot changes.
 */
export type WebViewPropsOnStateChangedOutput = void | Promise<void>;

/**
 * Completion mode returned after a WebView lifecycle transition.
 */
export type WebViewPropsOnLifecycleEventOutput = void | Promise<void>;

/**
 * Navigation decision returned directly or after asynchronous evaluation.
 */
export type WebViewPropsOnShouldOverrideUrlLoadingOutput = ComposeWebViewNavigationDecision | Promise<ComposeWebViewNavigationDecision | null | undefined>;

/**
 * Resource interception decision returned directly or asynchronously.
 */
export type WebViewPropsOnInterceptRequestOutput = ComposeWebViewResourceDecision | Promise<ComposeWebViewResourceDecision | null | undefined>;

/**
 * Scalar value accepted when interpolating a Compose template.
 */
export type ComposeTemplateValuesAdditionalValue = string | number | boolean;

/**
 * Complete node-factory surface exposed through `ComposeDslContext::UI`.
 */
export type ComposeDslContextUIIntersection = ComposeUiFactoryRegistry & ComposeMaterial3GeneratedUiFactoryRegistry & Record<string, ComposeNodeFactory<Record<string, unknown>>>;

/**
 * Stable mutable reference retained across renders for a keyed screen instance.
 */
export interface ComposeDslContextMethodsUseRefReturn<T> {
  /**
   * Current value stored in the persistent reference.
   */
  current: T;
}

/**
 * Completion mode for updating one runtime environment entry.
 */
export type ComposeDslContextMethodsSetEnvReturn = Promise<void> | void;

/**
 * Completion mode for requesting route navigation.
 */
export type ComposeDslContextMethodsNavigateReturn = Promise<void> | void;

/**
 * Completion mode for displaying a host toast.
 */
export type ComposeDslContextMethodsShowToastReturn = Promise<void> | void;

/**
 * Completion mode for forwarding an error to the host.
 */
export type ComposeDslContextMethodsReportErrorReturn = Promise<void> | void;

/**
 * Completion mode for a batch of runtime environment updates.
 */
export type ComposeDslContextMethodsSetEnvsReturn = Promise<void> | void;

/**
 * Availability result for an imported package, returned immediately or asynchronously.
 */
export type ComposeDslContextMethodsIsPackageImportedReturn = Promise<boolean> | boolean;

/**
 * Package identifier produced when an import operation completes.
 */
export type ComposeDslContextMethodsImportPackageReturn = Promise<string> | string;

/**
 * Package identifier produced when an imported package is removed.
 */
export type ComposeDslContextMethodsRemovePackageReturn = Promise<string> | string;

/**
 * Package identifier produced when selecting an imported package for use.
 */
export type ComposeDslContextMethodsUsePackageReturn = Promise<string> | string;

/**
 * Imported package names returned immediately or asynchronously.
 */
export type ComposeDslContextMethodsListImportedPackagesReturn = Promise<string[]> | string[];

/**
 * Resolved runtime tool name returned immediately or asynchronously.
 */
export type ComposeDslContextMethodsResolveToolNameReturn = Promise<string> | string;

/**
 * Root node produced by a Compose screen renderer.
 */
export type ComposeDslScreenOutput = ComposeNode | Promise<ComposeNode>;

/**
 * Material typography role used to select themed text metrics.
 */
export type ComposeTextStyle = "headlineSmall" | "headlineMedium" | "titleLarge" | "titleMedium" | "titleSmall" | "bodyLarge" | "bodyMedium" | "bodySmall" | "labelLarge" | "labelMedium" | "labelSmall";

/**
 * Distribution strategy for children along a layout's main axis.
 */
export type ComposeArrangement = "start" | "center" | "end" | "spaceBetween" | "spaceAround" | "spaceEvenly";

/**
 * Cross-axis placement for children inside a layout container.
 */
export type ComposeAlignment = "start" | "center" | "end";

/**
 * Geometry used to clip or outline a Compose component.
 */
export type ComposeShapeType = "rounded" | "cut" | "circle" | "pill";

/**
 * Shape geometry with optional uniform, logical, or physical corner radii.
 */
export interface ComposeShape {
  /**
   * Geometry family used to interpret the radius fields.
   */
  type?: ComposeShapeType;
  /**
   * Uniform radius applied to every corner.
   */
  cornerRadius?: number;
  /**
   * Radius used by circular or pill geometry.
   */
  radius?: number;
  /**
   * Radius at the top corner on the layout-direction start side.
   */
  topStart?: number;
  /**
   * Radius at the top corner on the layout-direction end side.
   */
  topEnd?: number;
  /**
   * Radius at the bottom corner on the layout-direction start side.
   */
  bottomStart?: number;
  /**
   * Radius at the bottom corner on the layout-direction end side.
   */
  bottomEnd?: number;
  /**
   * Radius at the physical top-left corner.
   */
  topLeft?: number;
  /**
   * Radius at the physical top-right corner.
   */
  topRight?: number;
  /**
   * Radius at the physical bottom-left corner.
   */
  bottomLeft?: number;
  /**
   * Radius at the physical bottom-right corner.
   */
  bottomRight?: number;
}

/**
 * Stroke rendered around a component boundary.
 */
export interface ComposeBorder {
  /**
   * Thickness of the border stroke.
   */
  width?: number;
  /**
   * Solid or themed color used for the stroke.
   */
  color?: ComposeColor;
  /**
   * Opacity multiplier applied to the border color.
   */
  alpha?: number;
}

/**
 * Insets applied symmetrically on the horizontal and vertical axes.
 */
export interface ComposePadding {
  /**
   * Inset applied to the start and end edges.
   */
  horizontal?: number;
  /**
   * Inset applied to the top and bottom edges.
   */
  vertical?: number;
}

/**
 * Coordinate unit accepted by canvas drawing commands.
 */
export type ComposeCanvasUnit = "px" | "dp" | "fraction";

/**
 * Numeric canvas quantity paired with an explicit coordinate unit.
 */
export interface ComposeUnitValue {
  /**
   * Scalar magnitude before unit conversion.
   */
  value: number;
  /**
   * Coordinate space in which the magnitude is expressed.
   */
  unit: ComposeCanvasUnit;
}

/**
 * Canvas quantity expressed either in the command's default unit or explicitly.
 */
export type ComposeCanvasNumber = number | ComposeUnitValue;

/**
 * Rendering behavior when laid-out text exceeds its bounds.
 */
export type ComposeTextOverflow = "clip" | "ellipsis";

/**
 * Scaling strategy for fitting visual content into destination bounds.
 */
export type ComposeContentScale = "fit" | "crop" | "fillBounds" | "fillWidth" | "fillHeight" | "inside" | "none";

/**
 * Constraints used to measure text before constructing a canvas layout.
 */
export interface ComposeTextMeasureRequest {
  /**
   * Text whose rendered bounds are requested.
   */
  text: string;
  /**
   * Font size used for measurement.
   */
  fontSize?: number;
  /**
   * Maximum line width available to the text layout.
   */
  maxWidth: number;
  /**
   * Maximum total height available to the text layout.
   */
  maxHeight?: number;
  /**
   * Minimum width reserved for the measured layout.
   */
  minWidth?: number;
  /**
   * Minimum height reserved for the measured layout.
   */
  minHeight?: number;
  /**
   * Maximum number of laid-out lines.
   */
  maxLines?: number;
  /**
   * Clipping behavior when the text exceeds its constraints.
   */
  overflow?: ComposeTextOverflow;
}

/**
 * Final bounds of a text layout after applying measurement constraints.
 */
export interface ComposeTextMeasureResult {
  /**
   * Measured layout width.
   */
  width: number;
  /**
   * Measured layout height.
   */
  height: number;
}

/**
 * Reference to a named theme color with an optional opacity adjustment.
 */
export interface ComposeColorToken {
  /**
   * Host-resolved token name in the active Material color scheme.
   */
  __colorToken: string;
  /**
   * Opacity multiplier applied when resolving the token.
   */
  alpha?: number;
  /**
   * Derives the same theme color with a different opacity multiplier.
   */
  copy(options: ComposeColorTokenMethodsCopyOptions): ComposeColorToken;
}

/**
 * Color supplied either as a concrete CSS-style value or a theme token.
 */
export type ComposeColor = string | ComposeColorToken;

/**
 * Named semantic colors available from the active Material theme.
 */
export interface ComposeColorScheme {
  [key: string]: ComposeColorToken;
}

/**
 * Material theme values exposed while rendering a Compose DSL screen.
 */
export interface ComposeMaterialTheme {
  /**
   * Semantic color roles for the active light or dark theme.
   */
  colorScheme: ComposeColorScheme;
}

/**
 * Multi-color paint used to fill canvas geometry.
 */
export interface ComposeCanvasBrush {
  /**
   * Gradient direction used to interpolate the color stops.
   */
  type: ComposeCanvasBrushType;
  /**
   * Ordered color stops from the start edge to the end edge.
   */
  colors: ComposeColor[];
}

/**
 * Horizontal placement used by width and box-layout modifiers.
 */
export type ComposeHorizontalAlignment = "start" | "center" | "end" | "left" | "right" | "centerHorizontally";

/**
 * Vertical placement used by height and box-layout modifiers.
 */
export type ComposeVerticalAlignment = "top" | "center" | "bottom" | "start" | "end" | "centerVertically";

/**
 * Two-dimensional placement of content inside a box.
 */
export type ComposeBoxAlignment = "center" | "topStart" | "startTop" | "topCenter" | "centerTop" | "topEnd" | "endTop" | "centerStart" | "startCenter" | "centerEnd" | "endCenter" | "bottomStart" | "startBottom" | "bottomCenter" | "centerBottom" | "bottomEnd" | "endBottom";

/**
 * Alignment accepted by a modifier in horizontal, vertical, or box scope.
 */
export type ComposeModifierAlign = ComposeHorizontalAlignment | ComposeVerticalAlignment | ComposeBoxAlignment;

/**
 * Edge insets added by a padding modifier.
 */
export interface ComposeModifierPadding {
  /**
   * Uniform inset applied to every edge.
   */
  all?: number;
  /**
   * Inset applied to the logical start and end edges.
   */
  horizontal?: number;
  /**
   * Inset applied to the top and bottom edges.
   */
  vertical?: number;
  /**
   * Inset on the layout-direction start edge.
   */
  start?: number;
  /**
   * Inset on the top edge.
   */
  top?: number;
  /**
   * Inset on the layout-direction end edge.
   */
  end?: number;
  /**
   * Inset on the bottom edge.
   */
  bottom?: number;
}

/**
 * Translation applied to a node after layout measurement.
 */
export interface ComposeModifierOffset {
  /**
   * Horizontal displacement from the laid-out position.
   */
  x?: number;
  /**
   * Vertical displacement from the laid-out position.
   */
  y?: number;
}

/**
 * Minimum and maximum extent for one layout axis.
 */
export interface ComposeModifierAxisBounds {
  /**
   * Smallest permitted extent on the axis.
   */
  min?: number;
  /**
   * Largest permitted extent on the axis.
   */
  max?: number;
}

/**
 * Horizontal measurement constraints accepted by `widthIn`.
 */
export interface ComposeModifierWidthBounds extends ComposeModifierAxisBounds {
  /**
   * Explicit minimum measured width.
   */
  minWidth?: number;
  /**
   * Explicit maximum measured width.
   */
  maxWidth?: number;
}

/**
 * Vertical measurement constraints accepted by `heightIn`.
 */
export interface ComposeModifierHeightBounds extends ComposeModifierAxisBounds {
  /**
   * Explicit minimum measured height.
   */
  minHeight?: number;
  /**
   * Explicit maximum measured height.
   */
  maxHeight?: number;
}

/**
 * Independent width and height constraints accepted by `sizeIn`.
 */
export interface ComposeModifierSizeBounds {
  /**
   * Shorthand minimum applied to both dimensions.
   */
  min?: number;
  /**
   * Shorthand maximum applied to both dimensions.
   */
  max?: number;
  /**
   * Minimum measured width.
   */
  minWidth?: number;
  /**
   * Minimum measured height.
   */
  minHeight?: number;
  /**
   * Maximum measured width.
   */
  maxWidth?: number;
  /**
   * Maximum measured height.
   */
  maxHeight?: number;
}

/**
 * Default minimum dimensions used only when incoming constraints permit them.
 */
export interface ComposeModifierDefaultMinSize {
  /**
   * Shorthand minimum applied to both width and height.
   */
  all?: number;
  /**
   * Preferred minimum width.
   */
  minWidth?: number;
  /**
   * Preferred minimum height.
   */
  minHeight?: number;
}

/**
 * Horizontal placement when wrapping a node to its measured width.
 */
export interface ComposeModifierWrapContentWidthOptions {
  /**
   * Position of wrapped content within the available width.
   */
  align?: ComposeHorizontalAlignment;
  /**
   * Whether measurement may ignore the incoming maximum width.
   */
  unbounded?: boolean;
}

/**
 * Vertical placement when wrapping a node to its measured height.
 */
export interface ComposeModifierWrapContentHeightOptions {
  /**
   * Position of wrapped content within the available height.
   */
  align?: ComposeVerticalAlignment;
  /**
   * Whether measurement may ignore the incoming maximum height.
   */
  unbounded?: boolean;
}

/**
 * Two-dimensional placement when wrapping a node to its measured size.
 */
export interface ComposeModifierWrapContentSizeOptions {
  /**
   * Position of wrapped content within the available bounds.
   */
  align?: ComposeBoxAlignment;
  /**
   * Whether measurement may ignore incoming maximum dimensions.
   */
  unbounded?: boolean;
}

/**
 * Shadow elevation, outline, and clipping applied by a modifier.
 */
export interface ComposeModifierShadowOptions {
  /**
   * Elevation used to calculate blur and offset.
   */
  elevation: number;
  /**
   * Outline from which the shadow is cast.
   */
  shape?: ComposeShape;
  /**
   * Whether child drawing is clipped to the shadow outline.
   */
  clip?: boolean;
}

/**
 * Primary, long-press, and double-click callbacks installed as one gesture detector.
 */
export interface ComposeModifierCombinedClickableOptions {
  /**
   * Handles a recognized primary click.
   */
  onClick: () => ComposeModifierCombinedClickableOptionsOnClickOutput;
  /**
   * Handles a recognized long press when supplied.
   */
  onLongClick?: () => ComposeModifierCombinedClickableOptionsOnLongClickOutput;
  /**
   * Handles a recognized double click when supplied.
   */
  onDoubleClick?: () => ComposeModifierCombinedClickableOptionsOnDoubleClickOutput;
}

/**
 * Pointer location reported in the target node's local coordinate space.
 */
export interface ComposePointerOffsetEvent {
  /**
   * Horizontal pointer coordinate.
   */
  x: number;
  /**
   * Vertical pointer coordinate.
   */
  y: number;
}

/**
 * Pointer position and movement delta for one drag update.
 */
export interface ComposeDragGestureEvent extends ComposePointerOffsetEvent {
  /**
   * Horizontal movement since the preceding drag event.
   */
  deltaX: number;
  /**
   * Vertical movement since the preceding drag event.
   */
  deltaY: number;
}

/**
 * Measured dimensions reported after a node's layout size changes.
 */
export interface ComposeSizeChangedEvent {
  /**
   * New measured width.
   */
  width: number;
  /**
   * New measured height.
   */
  height: number;
}

/**
 * Node bounds expressed in both root and host-window coordinate spaces.
 */
export interface ComposeGloballyPositionedEvent {
  /**
   * Horizontal offset from the Compose root.
   */
  rootX: number;
  /**
   * Vertical offset from the Compose root.
   */
  rootY: number;
  /**
   * Measured node width.
   */
  width: number;
  /**
   * Measured node height.
   */
  height: number;
  /**
   * Horizontal offset from the host window.
   */
  windowX: number;
  /**
   * Vertical offset from the host window.
   */
  windowY: number;
}

/**
 * Callbacks for distinct press and tap gestures on a node.
 */
export interface ComposeModifierTapGesturesOptions {
  /**
   * Runs as soon as a pointer press is recognized.
   */
  onPress?: (arg0: ComposePointerOffsetEvent) => ComposeModifierTapGesturesOptionsOnPressOutput;
  /**
   * Runs after a single tap is recognized.
   */
  onTap?: (arg0: ComposePointerOffsetEvent) => ComposeModifierTapGesturesOptionsOnTapOutput;
  /**
   * Runs after two taps are recognized within the gesture interval.
   */
  onDoubleTap?: (arg0: ComposePointerOffsetEvent) => ComposeModifierTapGesturesOptionsOnDoubleTapOutput;
  /**
   * Runs when a press exceeds the long-press threshold.
   */
  onLongPress?: (arg0: ComposePointerOffsetEvent) => ComposeModifierTapGesturesOptionsOnLongPressOutput;
}

/**
 * Callbacks describing the start, updates, and termination of a drag gesture.
 */
export interface ComposeModifierDragGesturesOptions {
  /**
   * Receives the pointer location where dragging begins.
   */
  onDragStart?: (arg0: ComposePointerOffsetEvent) => ComposeModifierDragGesturesOptionsOnDragStartOutput;
  /**
   * Receives each pointer position and incremental movement during dragging.
   */
  onDrag?: (arg0: ComposeDragGestureEvent) => ComposeModifierDragGesturesOptionsOnDragOutput;
  /**
   * Runs when the pointer is released after a successful drag.
   */
  onDragEnd?: () => ComposeModifierDragGesturesOptionsOnDragEndOutput;
  /**
   * Runs when another gesture or lifecycle event cancels the drag.
   */
  onDragCancel?: () => ComposeModifierDragGesturesOptionsOnDragCancelOutput;
}

/**
 * Configuration for combined pan, pinch-zoom, and rotation gestures.
 */
export interface ComposeModifierTransformGesturesOptions {
  /**
   * Locks gesture recognition to pan and zoom after those motions win the slop race.
   */
  panZoomLock?: boolean;
  /**
   * Receives the centroid and incremental pan, zoom, and rotation deltas.
   */
  onGesture: (arg0: ComposeCanvasTransformEvent) => ComposeModifierTransformGesturesOptionsOnGestureOutput;
}

/**
 * Persistent scale and translation applied while drawing a canvas.
 */
export interface ComposeCanvasTransform {
  /**
   * Uniform scale factor applied to canvas content.
   */
  scale?: number;
  /**
   * Horizontal translation after scaling.
   */
  offsetX?: number;
  /**
   * Vertical translation after scaling.
   */
  offsetY?: number;
  /**
   * Horizontal coordinate around which scaling is performed.
   */
  pivotX?: number;
  /**
   * Vertical coordinate around which scaling is performed.
   */
  pivotY?: number;
}

/**
 * Incremental multi-touch transform reported by a canvas gesture detector.
 */
export interface ComposeCanvasTransformEvent {
  /**
   * Horizontal center of the active pointers.
   */
  centroidX: number;
  /**
   * Vertical center of the active pointers.
   */
  centroidY: number;
  /**
   * Horizontal translation since the preceding event.
   */
  panX: number;
  /**
   * Vertical translation since the preceding event.
   */
  panY: number;
  /**
   * Multiplicative scale delta since the preceding event.
   */
  zoom: number;
  /**
   * Rotation delta around the pointer centroid.
   */
  rotation: number;
}

/**
 * Canvas dimensions reported after layout measurement changes.
 */
export interface ComposeCanvasSizeEvent extends ComposeSizeChangedEvent {
}

/**
 * Page metadata captured at the start or completion of WebView navigation.
 */
export interface ComposeWebViewPageEvent {
  /**
   * Page URL known at the time of the callback.
   */
  url?: string | null;
  /**
   * Current document title, when available.
   */
  title?: string | null;
  /**
   * Whether backward history navigation is currently possible.
   */
  canGoBack?: boolean;
  /**
   * Whether forward history navigation is currently possible.
   */
  canGoForward?: boolean;
}

/**
 * Details emitted when the WebView's active URL changes.
 */
export interface ComposeWebViewNavigationEvent {
  /**
   * Newly active or requested URL.
   */
  url?: string | null;
  /**
   * Whether the navigation targets the top-level document.
   */
  isMainFrame?: boolean;
  /**
   * HTTP method used by the navigation request.
   */
  method?: string | null;
}

/**
 * Snapshot emitted as the current page advances through loading.
 */
export interface ComposeWebViewProgressEvent {
  /**
   * Host-reported loading completion value.
   */
  progress: number;
  /**
   * URL associated with the loading page.
   */
  url?: string | null;
  /**
   * Current document title, when available.
   */
  title?: string | null;
}

/**
 * Network or content-loading failure reported by a WebView.
 */
export interface ComposeWebViewErrorEvent {
  /**
   * Platform WebView error code.
   */
  errorCode: number;
  /**
   * Human-readable platform error description.
   */
  description?: string | null;
  /**
   * Resource URL that failed to load.
   */
  url?: string | null;
}

/**
 * Non-success HTTP response observed while loading WebView content.
 */
export interface ComposeWebViewHttpErrorEvent {
  /**
   * HTTP response status code.
   */
  statusCode?: number | null;
  /**
   * HTTP reason phrase supplied by the server.
   */
  reasonPhrase?: string | null;
  /**
   * URL whose response contained the error status.
   */
  url?: string | null;
  /**
   * Whether the failed response belongs to the top-level document.
   */
  isMainFrame?: boolean;
}

/**
 * TLS certificate validation failure observed by a WebView.
 */
export interface ComposeWebViewSslErrorEvent {
  /**
   * Platform code for the primary certificate failure.
   */
  primaryError?: number | null;
  /**
   * HTTPS URL whose certificate failed validation.
   */
  url?: string | null;
}

/**
 * Download request initiated by content inside a WebView.
 */
export interface ComposeWebViewDownloadEvent {
  /**
   * URL of the file requested for download.
   */
  url: string;
  /**
   * User-Agent associated with the request.
   */
  userAgent?: string | null;
  /**
   * Server content-disposition header used to infer a filename.
   */
  contentDisposition?: string | null;
  /**
   * Reported media type of the download.
   */
  mimeType?: string | null;
  /**
   * Reported download size in bytes.
   */
  contentLength?: number | null;
  /**
   * Filename derived by the host from URL and response metadata.
   */
  suggestedFileName?: string | null;
}

/**
 * Console entry emitted by JavaScript running in WebView content.
 */
export interface ComposeWebViewConsoleEvent {
  /**
   * Text written to the page console.
   */
  message?: string | null;
  /**
   * Script URL or source identifier that emitted the entry.
   */
  sourceId?: string | null;
  /**
   * Source line associated with the entry.
   */
  lineNumber?: number | null;
  /**
   * Console severity such as debug, warning, or error.
   */
  level?: string | null;
}

/**
 * Current navigation and loading state of a controlled WebView.
 */
export interface ComposeWebViewState {
  /**
   * URL currently displayed or being loaded.
   */
  url?: string | null;
  /**
   * Current document title.
   */
  title?: string | null;
  /**
   * Whether the WebView is actively loading content.
   */
  loading: boolean;
  /**
   * Host-reported loading completion value.
   */
  progress: number;
  /**
   * Whether backward navigation is available in history.
   */
  canGoBack: boolean;
  /**
   * Whether forward navigation is available in history.
   */
  canGoForward: boolean;
}

/**
 * WebView lifecycle transition accompanied by the latest page state.
 */
export interface ComposeWebViewLifecycleEvent {
  /**
   * Native lifecycle transition that triggered the event.
   */
  type: ComposeWebViewLifecycleEventType;
  /**
   * Active URL when the transition occurred.
   */
  url?: string | null;
  /**
   * Active document title when available.
   */
  title?: string | null;
  /**
   * Whether loading was active during the transition.
   */
  loading?: boolean;
  /**
   * Loading completion value captured with the transition.
   */
  progress?: number;
  /**
   * Whether backward history navigation was available.
   */
  canGoBack?: boolean;
  /**
   * Whether forward history navigation was available.
   */
  canGoForward?: boolean;
  /**
   * Whether renderer termination was caused by a crash.
   */
  didCrash?: boolean;
  /**
   * Platform renderer priority recorded when the process exited.
   */
  rendererPriorityAtExit?: number | null;
}

/**
 * Top-level or subframe navigation request offered to plugin interception.
 */
export interface ComposeWebViewNavigationRequest {
  /**
   * Destination requested by the page or user.
   */
  url: string;
  /**
   * HTTP method used for the navigation.
   */
  method?: string | null;
  /**
   * Request headers visible to the WebView host.
   */
  headers?: Record<string, string>;
  /**
   * Whether the request targets the top-level document.
   */
  isMainFrame?: boolean;
  /**
   * Whether a user gesture initiated the request.
   */
  hasGesture?: boolean;
  /**
   * Whether the request follows an HTTP or script redirect.
   */
  isRedirect?: boolean;
  /**
   * Parsed URL scheme used for protocol-specific decisions.
   */
  scheme?: string | null;
}

/**
 * Action selected by a plugin after inspecting a navigation request.
 */
export type ComposeWebViewNavigationDecision = ComposeWebViewNavigationDecisionVariant1 | ComposeWebViewNavigationDecisionVariant2 | ComposeWebViewNavigationDecisionVariant3 | ComposeWebViewNavigationDecisionVariant4;

/**
 * Document or subresource request offered to WebView resource interception.
 */
export interface ComposeWebViewResourceRequest {
  /**
   * URL of the requested resource.
   */
  url: string;
  /**
   * HTTP method used for the request.
   */
  method?: string | null;
  /**
   * Request headers visible to the WebView host.
   */
  headers?: Record<string, string>;
  /**
   * Whether the resource is the top-level document.
   */
  isMainFrame?: boolean;
  /**
   * Whether a user gesture initiated the request.
   */
  hasGesture?: boolean;
  /**
   * Whether the request follows a redirect.
   */
  isRedirect?: boolean;
  /**
   * Parsed URL scheme used for protocol-specific handling.
   */
  scheme?: string | null;
}

/**
 * Body source used when a plugin supplies a synthetic WebView response.
 */
export type ComposeWebViewResourceResponse = ComposeWebViewResourceResponseVariant1 | ComposeWebViewResourceResponseVariant2 | ComposeWebViewResourceResponseVariant3;

/**
 * Action selected by a plugin after inspecting a WebView resource request.
 */
export type ComposeWebViewResourceDecision = ComposeWebViewResourceDecisionVariant1 | ComposeWebViewResourceDecisionVariant2 | ComposeWebViewResourceDecisionVariant3 | ComposeWebViewResourceDecisionVariant4;

/**
 * Callable method exposed by the host object to JavaScript inside a WebView.
 */
export type ComposeWebViewJavascriptInterfaceMethod = (arg0: unknown[]) => ComposeWebViewJavascriptInterfaceMethodOutput;

/**
 * Named methods installed together as one WebView JavaScript interface object.
 */
export type ComposeWebViewJavascriptInterface = Record<string, ComposeWebViewJavascriptInterfaceMethod>;

/**
 * Origin and decoding metadata used when loading an inline HTML document.
 */
export interface ComposeWebViewLoadHtmlOptions {
  /**
   * Base URL used to resolve relative links and establish document origin.
   */
  baseUrl?: string;
  /**
   * Media type assigned to the inline document.
   */
  mimeType?: string;
  /**
   * Character encoding used to decode the HTML string.
   */
  encoding?: string;
}

/**
 * Stable handle used to issue imperative commands to a rendered WebView.
 */
export interface ComposeWebViewController {
  /**
   * Identity linking the controller to its WebView node across renders.
   */
  key: string;
  /**
   * Loads a URL with optional request headers in the controlled WebView.
   */
  loadUrl(url: string, headers?: Record<string, string>): void;
  /**
   * Loads an inline HTML document using optional origin and encoding metadata.
   */
  loadHtml(html: string, options?: ComposeWebViewLoadHtmlOptions): void;
  /**
   * Reloads the currently active document.
   */
  reload(): void;
  /**
   * Cancels the current page load.
   */
  stopLoading(): void;
  /**
   * Navigates to the preceding WebView history entry.
   */
  goBack(): void;
  /**
   * Navigates to the following WebView history entry.
   */
  goForward(): void;
  /**
   * Removes stored back and forward history entries.
   */
  clearHistory(): void;
  /**
   * Evaluates a script in the active page and resolves its decoded result.
   */
  evaluateJavascript<TResult>(script: string): Promise<TResult | null | undefined>;
  /**
   * Returns the latest navigation and loading state known by the controller.
   */
  getState(): ComposeWebViewState | null | undefined;
  /**
   * Installs a named host object callable by JavaScript in the page.
   */
  addJavascriptInterface(name: string, object: ComposeWebViewJavascriptInterface): void;
  /**
   * Removes a previously installed JavaScript interface object.
   */
  removeJavascriptInterface(name: string): void;
}

/**
 * Policy for HTTP subresources requested by an HTTPS page.
 */
export type ComposeWebViewMixedContentMode = "alwaysAllow" | "neverAllow" | "compatibilityMode";

/**
 * Cache policy applied to WebView network requests.
 */
export type ComposeWebViewCacheMode = "default" | "noCache" | "cacheElseNetwork" | "cacheOnly";

/**
 * Command that strokes a straight segment between two canvas points.
 */
export interface ComposeCanvasLineCommand {
  /**
   * Selects straight-line command decoding.
   */
  type: ComposeCanvasLineCommandType;
  /**
   * Horizontal coordinate of the segment start.
   */
  x1: ComposeCanvasNumber;
  /**
   * Vertical coordinate of the segment start.
   */
  y1: ComposeCanvasNumber;
  /**
   * Horizontal coordinate of the segment end.
   */
  x2: ComposeCanvasNumber;
  /**
   * Vertical coordinate of the segment end.
   */
  y2: ComposeCanvasNumber;
  /**
   * Color used to stroke the segment.
   */
  color?: ComposeColor;
  /**
   * Opacity multiplier applied to the stroke color.
   */
  alpha?: number;
  /**
   * Thickness of the stroked segment.
   */
  strokeWidth?: ComposeCanvasNumber;
  /**
   * Default unit for scalar coordinates and stroke width.
   */
  unit?: ComposeCanvasUnit;
}

/**
 * Command that fills or strokes an axis-aligned canvas rectangle.
 */
export interface ComposeCanvasRectCommand {
  /**
   * Selects rectangle command decoding.
   */
  type: ComposeCanvasRectCommandType;
  /**
   * Horizontal coordinate of the rectangle origin.
   */
  x: ComposeCanvasNumber;
  /**
   * Vertical coordinate of the rectangle origin.
   */
  y: ComposeCanvasNumber;
  /**
   * Rectangle width.
   */
  width: ComposeCanvasNumber;
  /**
   * Rectangle height.
   */
  height: ComposeCanvasNumber;
  /**
   * Gradient paint used instead of a solid color.
   */
  brush?: ComposeCanvasBrush;
  /**
   * Solid paint used when no brush is supplied.
   */
  color?: ComposeColor;
  /**
   * Opacity multiplier applied to the selected paint.
   */
  alpha?: number;
  /**
   * Outline thickness when the rectangle is not filled.
   */
  strokeWidth?: ComposeCanvasNumber;
  /**
   * Whether to fill the interior instead of drawing only the outline.
   */
  filled?: boolean;
  /**
   * Default unit for scalar geometry and stroke width.
   */
  unit?: ComposeCanvasUnit;
}

/**
 * Command that fills or strokes a rectangle with rounded corners.
 */
export interface ComposeCanvasRoundRectCommand {
  /**
   * Selects rounded-rectangle command decoding.
   */
  type: ComposeCanvasRoundRectCommandType;
  /**
   * Horizontal coordinate of the rectangle origin.
   */
  x: ComposeCanvasNumber;
  /**
   * Vertical coordinate of the rectangle origin.
   */
  y: ComposeCanvasNumber;
  /**
   * Rectangle width.
   */
  width: ComposeCanvasNumber;
  /**
   * Rectangle height.
   */
  height: ComposeCanvasNumber;
  /**
   * Radius applied to each corner.
   */
  radius?: ComposeCanvasNumber;
  /**
   * Gradient paint used instead of a solid color.
   */
  brush?: ComposeCanvasBrush;
  /**
   * Solid paint used when no brush is supplied.
   */
  color?: ComposeColor;
  /**
   * Opacity multiplier applied to the selected paint.
   */
  alpha?: number;
  /**
   * Outline thickness when the shape is not filled.
   */
  strokeWidth?: ComposeCanvasNumber;
  /**
   * Whether to fill the interior instead of drawing only the outline.
   */
  filled?: boolean;
  /**
   * Default unit for scalar geometry and stroke width.
   */
  unit?: ComposeCanvasUnit;
}

/**
 * Command that fills or strokes a circle on the canvas.
 */
export interface ComposeCanvasCircleCommand {
  /**
   * Selects circle command decoding.
   */
  type: ComposeCanvasCircleCommandType;
  /**
   * Horizontal coordinate of the circle center.
   */
  cx: ComposeCanvasNumber;
  /**
   * Vertical coordinate of the circle center.
   */
  cy: ComposeCanvasNumber;
  /**
   * Distance from the center to the circle edge.
   */
  radius: ComposeCanvasNumber;
  /**
   * Color used to fill or stroke the circle.
   */
  color?: ComposeColor;
  /**
   * Opacity multiplier applied to the circle color.
   */
  alpha?: number;
  /**
   * Outline thickness when the circle is not filled.
   */
  strokeWidth?: ComposeCanvasNumber;
  /**
   * Whether to fill the interior instead of drawing only the outline.
   */
  filled?: boolean;
  /**
   * Default unit for scalar geometry and stroke width.
   */
  unit?: ComposeCanvasUnit;
}

/**
 * Command that lays out constrained text and draws it at a canvas position.
 */
export interface ComposeCanvasTextCommand {
  /**
   * Selects measured-text command decoding.
   */
  type: ComposeCanvasTextCommandType;
  /**
   * Horizontal coordinate of the text layout origin.
   */
  x: ComposeCanvasNumber;
  /**
   * Vertical coordinate of the text layout origin.
   */
  y: ComposeCanvasNumber;
  /**
   * Text content to lay out and render.
   */
  text: string;
  /**
   * Color used to render glyphs.
   */
  color?: ComposeColor;
  /**
   * Opacity multiplier applied to the glyph color.
   */
  alpha?: number;
  /**
   * Font size used by the text layout.
   */
  fontSize?: ComposeCanvasNumber;
  /**
   * Minimum width reserved for the text layout.
   */
  minWidth?: ComposeCanvasNumber;
  /**
   * Maximum width before lines wrap or overflow.
   */
  maxWidth?: ComposeCanvasNumber;
  /**
   * Minimum height reserved for the text layout.
   */
  minHeight?: ComposeCanvasNumber;
  /**
   * Maximum height available to the text layout.
   */
  maxHeight?: ComposeCanvasNumber;
  /**
   * Maximum number of rendered lines.
   */
  maxLines?: number;
  /**
   * Clipping behavior when text exceeds its layout bounds.
   */
  overflow?: ComposeTextOverflow;
  /**
   * Default unit for scalar positions, dimensions, and font size.
   */
  unit?: ComposeCanvasUnit;
}

/**
 * Path operation that starts a new contour at a canvas point.
 */
export interface ComposeCanvasMoveToOp {
  /**
   * Selects move-to operation decoding.
   */
  type: ComposeCanvasMoveToOpType;
  /**
   * Horizontal coordinate of the new current point.
   */
  x: ComposeCanvasNumber;
  /**
   * Vertical coordinate of the new current point.
   */
  y: ComposeCanvasNumber;
}

/**
 * Path operation that appends a straight segment to a point.
 */
export interface ComposeCanvasLineToOp {
  /**
   * Selects line-to operation decoding.
   */
  type: ComposeCanvasLineToOpType;
  /**
   * Horizontal coordinate of the segment endpoint.
   */
  x: ComposeCanvasNumber;
  /**
   * Vertical coordinate of the segment endpoint.
   */
  y: ComposeCanvasNumber;
}

/**
 * Path operation that appends a cubic Bezier segment.
 */
export interface ComposeCanvasCubicToOp {
  /**
   * Selects cubic-curve operation decoding.
   */
  type: ComposeCanvasCubicToOpType;
  /**
   * Horizontal coordinate of the first control point.
   */
  x1: ComposeCanvasNumber;
  /**
   * Vertical coordinate of the first control point.
   */
  y1: ComposeCanvasNumber;
  /**
   * Horizontal coordinate of the second control point.
   */
  x2: ComposeCanvasNumber;
  /**
   * Vertical coordinate of the second control point.
   */
  y2: ComposeCanvasNumber;
  /**
   * Horizontal coordinate of the curve endpoint.
   */
  x3: ComposeCanvasNumber;
  /**
   * Vertical coordinate of the curve endpoint.
   */
  y3: ComposeCanvasNumber;
}

/**
 * Path operation that appends a quadratic Bezier segment.
 */
export interface ComposeCanvasQuadToOp {
  /**
   * Selects quadratic-curve operation decoding.
   */
  type: ComposeCanvasQuadToOpType;
  /**
   * Horizontal coordinate of the control point.
   */
  x1: ComposeCanvasNumber;
  /**
   * Vertical coordinate of the control point.
   */
  y1: ComposeCanvasNumber;
  /**
   * Horizontal coordinate of the curve endpoint.
   */
  x2: ComposeCanvasNumber;
  /**
   * Vertical coordinate of the curve endpoint.
   */
  y2: ComposeCanvasNumber;
}

/**
 * Path operation that closes the active contour.
 */
export interface ComposeCanvasCloseOp {
  /**
   * Selects close-contour operation decoding.
   */
  type: ComposeCanvasCloseOpType;
}

/**
 * Operation used to construct the geometry of a drawable canvas path.
 */
export type ComposeCanvasPathOp = ComposeCanvasMoveToOp | ComposeCanvasLineToOp | ComposeCanvasCubicToOp | ComposeCanvasQuadToOp | ComposeCanvasCloseOp;

/**
 * Whether canvas path geometry is filled or outlined.
 */
export type ComposeCanvasDrawStyle = "fill" | "stroke";

/**
 * Command that assembles path operations and renders the resulting geometry.
 */
export interface ComposeCanvasDrawPathCommand {
  /**
   * Selects draw-path command decoding.
   */
  type: ComposeCanvasDrawPathCommandType;
  /**
   * Ordered operations that construct one or more path contours.
   */
  path: ComposeCanvasPathOp[];
  /**
   * Color used to fill or stroke the path.
   */
  color?: ComposeColor;
  /**
   * Opacity multiplier applied to the path color.
   */
  alpha?: number;
  /**
   * Outline thickness when using stroke style.
   */
  strokeWidth?: ComposeCanvasNumber;
  /**
   * Chooses interior fill or boundary stroke rendering.
   */
  style?: ComposeCanvasDrawStyle;
  /**
   * Default unit for scalar path coordinates and stroke width.
   */
  unit?: ComposeCanvasUnit;
}

/**
 * Draw-scope command for a filled or stroked rounded rectangle.
 */
export interface ComposeCanvasDrawRoundRectCommand {
  /**
   * Selects draw-rounded-rectangle command decoding.
   */
  type: ComposeCanvasDrawRoundRectCommandType;
  /**
   * Horizontal coordinate of the rectangle origin.
   */
  x: ComposeCanvasNumber;
  /**
   * Vertical coordinate of the rectangle origin.
   */
  y: ComposeCanvasNumber;
  /**
   * Rectangle width.
   */
  width: ComposeCanvasNumber;
  /**
   * Rectangle height.
   */
  height: ComposeCanvasNumber;
  /**
   * Radius applied to each corner.
   */
  cornerRadius?: ComposeCanvasNumber;
  /**
   * Gradient paint used instead of a solid color.
   */
  brush?: ComposeCanvasBrush;
  /**
   * Solid paint used when no brush is supplied.
   */
  color?: ComposeColor;
  /**
   * Opacity multiplier applied to the selected paint.
   */
  alpha?: number;
  /**
   * Outline thickness when using stroke style.
   */
  strokeWidth?: ComposeCanvasNumber;
  /**
   * Chooses interior fill or boundary stroke rendering.
   */
  style?: ComposeCanvasDrawStyle;
  /**
   * Default unit for scalar geometry and stroke width.
   */
  unit?: ComposeCanvasUnit;
}

/**
 * Draw-scope command that lays out and paints text at a canvas position.
 */
export interface ComposeCanvasDrawTextCommand {
  /**
   * Selects draw-text command decoding.
   */
  type: ComposeCanvasDrawTextCommandType;
  /**
   * Text content to lay out and render.
   */
  text: string;
  /**
   * Horizontal coordinate of the text layout origin.
   */
  x: ComposeCanvasNumber;
  /**
   * Vertical coordinate of the text layout origin.
   */
  y: ComposeCanvasNumber;
  /**
   * Color used to render glyphs.
   */
  color?: ComposeColor;
  /**
   * Opacity multiplier applied to the glyph color.
   */
  alpha?: number;
  /**
   * Font size used by the text layout.
   */
  fontSize?: ComposeCanvasNumber;
  /**
   * Font weight name or numeric weight understood by the renderer.
   */
  fontWeight?: string;
  /**
   * Minimum width reserved for the text layout.
   */
  minWidth?: ComposeCanvasNumber;
  /**
   * Maximum width before lines wrap or overflow.
   */
  maxWidth?: ComposeCanvasNumber;
  /**
   * Minimum height reserved for the text layout.
   */
  minHeight?: ComposeCanvasNumber;
  /**
   * Maximum height available to the text layout.
   */
  maxHeight?: ComposeCanvasNumber;
  /**
   * Maximum number of rendered lines.
   */
  maxLines?: number;
  /**
   * Clipping behavior when text exceeds its layout bounds.
   */
  overflow?: ComposeTextOverflow;
  /**
   * Default unit for scalar positions, dimensions, and font size.
   */
  unit?: ComposeCanvasUnit;
}

/**
 * Draw-scope command that paints a named Material icon.
 */
export interface ComposeCanvasDrawIconCommand {
  /**
   * Selects draw-icon command decoding.
   */
  type: ComposeCanvasDrawIconCommandType;
  /**
   * Material icon name resolved by the host icon registry.
   */
  icon: string;
  /**
   * Horizontal coordinate of the icon bounds.
   */
  x: ComposeCanvasNumber;
  /**
   * Vertical coordinate of the icon bounds.
   */
  y: ComposeCanvasNumber;
  /**
   * Width and height of the square icon bounds.
   */
  size?: ComposeCanvasNumber;
  /**
   * Tint applied to the icon glyph.
   */
  color?: ComposeColor;
  /**
   * Opacity multiplier applied to the tint.
   */
  alpha?: number;
  /**
   * Default unit for scalar position and size values.
   */
  unit?: ComposeCanvasUnit;
}

/**
 * Drawing operation rendered in order by a Compose canvas node.
 */
export type ComposeCanvasCommand = ComposeCanvasLineCommand | ComposeCanvasRectCommand | ComposeCanvasRoundRectCommand | ComposeCanvasCircleCommand | ComposeCanvasTextCommand | ComposeCanvasDrawPathCommand | ComposeCanvasDrawRoundRectCommand | ComposeCanvasDrawTextCommand | ComposeCanvasDrawIconCommand;

/**
 * Serialized operation name stored in a Compose modifier chain.
 */
export type ComposeModifierName = "fillMaxSize" | "fillMaxWidth" | "fillMaxHeight" | "width" | "height" | "requiredWidth" | "requiredHeight" | "size" | "requiredSize" | "padding" | "offset" | "widthIn" | "heightIn" | "sizeIn" | "requiredWidthIn" | "requiredHeightIn" | "requiredSizeIn" | "defaultMinSize" | "wrapContentWidth" | "wrapContentHeight" | "wrapContentSize" | "aspectRatio" | "alpha" | "rotate" | "scale" | "zIndex" | "background" | "border" | "clip" | "clipToBounds" | "shadow" | "clickable" | "combinedClickable" | "tapGestures" | "dragGestures" | "transformGestures" | "onSizeChanged" | "onGloballyPositioned" | "imePadding" | "statusBarsPadding" | "navigationBarsPadding" | "systemBarsPadding" | "safeDrawingPadding" | "weight" | "align" | "matchParentSize";

/**
 * One serialized operation in an immutable Compose modifier chain.
 */
export interface ComposeModifierOp {
  /**
   * Operation applied by the host during node layout or drawing.
   */
  name: ComposeModifierName;
  /**
   * Positional arguments encoded for the selected operation.
   */
  args?: unknown[];
}

/**
 * Serializable sequence of modifier operations attached to a Compose node.
 */
export interface ComposeModifierValue {
  /**
   * Operations applied in chain order by the host renderer.
   */
  __modifierOps: ComposeModifierOp[];
}

/**
 * Stores a dynamically dispatched Compose modifier implementation.
 */
export type ComposeModifierProxy = ComposeModifierProxyApi;

/**
 * Chainable layout, drawing, input, and positioning operations for Compose nodes.
 */
export interface ComposeModifierProxyApi {
  /**
   * Occupies the requested fraction of both available dimensions.
   */
  fillMaxSize(fraction?: number): ComposeModifierProxy;
  /**
   * Occupies the requested fraction of the available width.
   */
  fillMaxWidth(fraction?: number): ComposeModifierProxy;
  /**
   * Occupies the requested fraction of the available height.
   */
  fillMaxHeight(fraction?: number): ComposeModifierProxy;
  /**
   * Requests a width that remains subject to parent constraints.
   */
  width(value: number): ComposeModifierProxy;
  /**
   * Requests a height that remains subject to parent constraints.
   */
  height(value: number): ComposeModifierProxy;
  /**
   * Forces the measured width even when it conflicts with parent constraints.
   */
  requiredWidth(value: number): ComposeModifierProxy;
  /**
   * Forces the measured height even when it conflicts with parent constraints.
   */
  requiredHeight(value: number): ComposeModifierProxy;
  /**
   * Requests the same constrained extent for width and height.
   */
  size(value: number): ComposeModifierProxy;
  /**
   * Requests independent constrained width and height values.
   */
  size(width: number, height: number): ComposeModifierProxy;
  /**
   * Forces the same extent for width and height.
   */
  requiredSize(value: number): ComposeModifierProxy;
  /**
   * Forces independent width and height values.
   */
  requiredSize(width: number, height: number): ComposeModifierProxy;
  /**
   * Adds one uniform inset between the node boundary and its content.
   */
  padding(value: number): ComposeModifierProxy;
  /**
   * Adds separate horizontal and vertical content insets.
   */
  padding(horizontal: number, vertical: number): ComposeModifierProxy;
  /**
   * Adds explicit logical-start, top, logical-end, and bottom insets.
   */
  padding(start: number, top: number, end: number, bottom: number): ComposeModifierProxy;
  /**
   * Adds insets from a structure that can mix uniform, axis, and edge values.
   */
  padding(values: ComposeModifierPadding): ComposeModifierProxy;
  /**
   * Translates the laid-out node by horizontal and optional vertical offsets.
   */
  offset(x: number, y?: number): ComposeModifierProxy;
  /**
   * Translates the laid-out node using structured axis offsets.
   */
  offset(values: ComposeModifierOffset): ComposeModifierProxy;
  /**
   * Constrains measured width to optional minimum and maximum values.
   */
  widthIn(min?: number, max?: number): ComposeModifierProxy;
  /**
   * Constrains measured width using shorthand and explicit width bounds.
   */
  widthIn(bounds: ComposeModifierWidthBounds): ComposeModifierProxy;
  /**
   * Constrains measured height to optional minimum and maximum values.
   */
  heightIn(min?: number, max?: number): ComposeModifierProxy;
  /**
   * Constrains measured height using shorthand and explicit height bounds.
   */
  heightIn(bounds: ComposeModifierHeightBounds): ComposeModifierProxy;
  /**
   * Constrains width and height with four explicit bounds.
   */
  sizeIn(minWidth: number, minHeight: number, maxWidth: number, maxHeight: number): ComposeModifierProxy;
  /**
   * Constrains both dimensions using a structured bounds object.
   */
  sizeIn(bounds: ComposeModifierSizeBounds): ComposeModifierProxy;
  /**
   * Forces width into the supplied optional range despite parent constraints.
   */
  requiredWidthIn(min?: number, max?: number): ComposeModifierProxy;
  /**
   * Forces width into structured shorthand and explicit bounds.
   */
  requiredWidthIn(bounds: ComposeModifierWidthBounds): ComposeModifierProxy;
  /**
   * Forces height into the supplied optional range despite parent constraints.
   */
  requiredHeightIn(min?: number, max?: number): ComposeModifierProxy;
  /**
   * Forces height into structured shorthand and explicit bounds.
   */
  requiredHeightIn(bounds: ComposeModifierHeightBounds): ComposeModifierProxy;
  /**
   * Forces both dimensions into four explicit bounds.
   */
  requiredSizeIn(minWidth: number, minHeight: number, maxWidth: number, maxHeight: number): ComposeModifierProxy;
  /**
   * Forces both dimensions into a structured set of bounds.
   */
  requiredSizeIn(bounds: ComposeModifierSizeBounds): ComposeModifierProxy;
  /**
   * Supplies preferred minimum width and optional minimum height.
   */
  defaultMinSize(minWidth: number, minHeight?: number): ComposeModifierProxy;
  /**
   * Supplies preferred minimum dimensions from structured values.
   */
  defaultMinSize(values: ComposeModifierDefaultMinSize): ComposeModifierProxy;
  /**
   * Wraps to measured width using the host's default horizontal alignment.
   */
  wrapContentWidth(): ComposeModifierProxy;
  /**
   * Wraps to measured width with explicit alignment and constraint handling.
   */
  wrapContentWidth(align: ComposeHorizontalAlignment, unbounded?: boolean): ComposeModifierProxy;
  /**
   * Wraps to measured width using structured alignment options.
   */
  wrapContentWidth(options: ComposeModifierWrapContentWidthOptions): ComposeModifierProxy;
  /**
   * Wraps to measured height using the host's default vertical alignment.
   */
  wrapContentHeight(): ComposeModifierProxy;
  /**
   * Wraps to measured height with explicit alignment and constraint handling.
   */
  wrapContentHeight(align: ComposeVerticalAlignment, unbounded?: boolean): ComposeModifierProxy;
  /**
   * Wraps to measured height using structured alignment options.
   */
  wrapContentHeight(options: ComposeModifierWrapContentHeightOptions): ComposeModifierProxy;
  /**
   * Wraps both dimensions using the host's default box alignment.
   */
  wrapContentSize(): ComposeModifierProxy;
  /**
   * Wraps both dimensions with explicit box alignment and constraint handling.
   */
  wrapContentSize(align: ComposeBoxAlignment, unbounded?: boolean): ComposeModifierProxy;
  /**
   * Wraps both dimensions using structured box-alignment options.
   */
  wrapContentSize(options: ComposeModifierWrapContentSizeOptions): ComposeModifierProxy;
  /**
   * Derives one measured dimension from the other using a width-to-height ratio.
   */
  aspectRatio(ratio: number): ComposeModifierProxy;
  /**
   * Multiplies the opacity of the node and its rendered descendants.
   */
  alpha(value: number): ComposeModifierProxy;
  /**
   * Rotates rendered content by the supplied angle.
   */
  rotate(value: number): ComposeModifierProxy;
  /**
   * Uniformly scales rendered content around its transform origin.
   */
  scale(value: number): ComposeModifierProxy;
  /**
   * Sets sibling draw order, with larger values rendered above smaller ones.
   */
  zIndex(value: number): ComposeModifierProxy;
  /**
   * Paints a solid color behind the node using an optional shape outline.
   */
  background(value: ComposeColor, shape?: ComposeShape): ComposeModifierProxy;
  /**
   * Paints a gradient brush behind the node using an optional shape outline.
   */
  background(value: ComposeCanvasBrush, shape?: ComposeShape): ComposeModifierProxy;
  /**
   * Strokes a solid-color border around an optional shape outline.
   */
  border(width: number, value: ComposeColor, shape?: ComposeShape): ComposeModifierProxy;
  /**
   * Strokes a gradient border around an optional shape outline.
   */
  border(width: number, value: ComposeCanvasBrush, shape?: ComposeShape): ComposeModifierProxy;
  /**
   * Clips the node and descendant drawing to the supplied shape.
   */
  clip(shape: ComposeShape): ComposeModifierProxy;
  /**
   * Clips descendant drawing to the node's rectangular layout bounds.
   */
  clipToBounds(): ComposeModifierProxy;
  /**
   * Casts a shadow from explicit elevation, shape, and clipping values.
   */
  shadow(elevation: number, shape?: ComposeShape, clip?: boolean): ComposeModifierProxy;
  /**
   * Casts a shadow using structured elevation and outline options.
   */
  shadow(options: ComposeModifierShadowOptions): ComposeModifierProxy;
  /**
   * Makes the node respond to a primary click callback.
   */
  clickable(onClick: () => ComposeModifierProxyClickableOnClickOutput): ComposeModifierProxy;
  /**
   * Installs primary, long-press, and double-click recognition together.
   */
  combinedClickable(options: ComposeModifierCombinedClickableOptions): ComposeModifierProxy;
  /**
   * Installs callbacks for press, tap, double-tap, and long-press gestures.
   */
  tapGestures(options: ComposeModifierTapGesturesOptions): ComposeModifierProxy;
  /**
   * Installs callbacks for drag start, updates, completion, and cancellation.
   */
  dragGestures(options: ComposeModifierDragGesturesOptions): ComposeModifierProxy;
  /**
   * Installs combined pan, pinch-zoom, and rotation recognition.
   */
  transformGestures(options: ComposeModifierTransformGesturesOptions): ComposeModifierProxy;
  /**
   * Observes changes to the node's measured width and height.
   */
  onSizeChanged(onSizeChanged: (arg0: ComposeSizeChangedEvent) => ComposeModifierProxyOnSizeChangedOnSizeChangedOutput): ComposeModifierProxy;
  /**
   * Observes node bounds in root and host-window coordinates after layout.
   */
  onGloballyPositioned(onGloballyPositioned: (arg0: ComposeGloballyPositionedEvent) => ComposeModifierProxyOnGloballyPositionedOnGloballyPositionedOutput): ComposeModifierProxy;
  /**
   * Adds padding matching the visible on-screen keyboard inset.
   */
  imePadding(): ComposeModifierProxy;
  /**
   * Adds padding matching the system status-bar inset.
   */
  statusBarsPadding(): ComposeModifierProxy;
  /**
   * Adds padding matching the system navigation-bar inset.
   */
  navigationBarsPadding(): ComposeModifierProxy;
  /**
   * Adds padding matching combined status and navigation bar insets.
   */
  systemBarsPadding(): ComposeModifierProxy;
  /**
   * Insets content from all system areas that can obscure drawing.
   */
  safeDrawingPadding(): ComposeModifierProxy;
  /**
   * Allocates remaining main-axis space proportionally within a row or column.
   */
  weight(weight: number, fill?: boolean): ComposeModifierProxy;
  /**
   * Overrides this child's placement within the parent layout scope.
   */
  align(alignment: ComposeModifierAlign): ComposeModifierProxy;
  /**
   * Matches the final size of the containing box without affecting its measurement.
   */
  matchParentSize(): ComposeModifierProxy;
  /**
   * Serializes the accumulated modifier operations for transport to the host.
   */
  toJSON(): ComposeModifierValue;
}

/**
 * Typography and color overrides applied to editable text.
 */
export interface ComposeTextFieldStyle {
  /**
   * Font size used for the entered value.
   */
  fontSize?: number;
  /**
   * Font weight name or numeric weight used for the entered value.
   */
  fontWeight?: string;
  /**
   * Font family used for the entered value.
   */
  fontFamily?: string;
  /**
   * Foreground color used for the entered value.
   */
  color?: ComposeColor;
}

/**
 * Layout, drawing, identity, and lifecycle properties shared by Compose nodes.
 */
export interface ComposeCommonProps {
  /**
   * Stable identity used to preserve node state across render passes.
   */
  key?: string;
  /**
   * Runs after the host has created and loaded the node.
   */
  onLoad?: () => ComposeCommonPropsOnLoadOutput;
  /**
   * Content presented as the host screen's top-bar title.
   */
  topBarTitle?: ComposeChildren;
  /**
   * Ordered modifier operations applied to layout, drawing, and input.
   */
  modifier?: ComposeModifierValue;
  /**
   * Sibling draw order, with larger values rendered above smaller ones.
   */
  zIndex?: number;
  /**
   * Share of remaining main-axis space inside a row or column.
   */
  weight?: number;
  /**
   * Whether weighted content expands to occupy its entire allocated share.
   */
  weightFill?: boolean;
  /**
   * Requested node width before parent constraints are applied.
   */
  width?: number;
  /**
   * Requested node height before parent constraints are applied.
   */
  height?: number;
  /**
   * Whether the node expands to the maximum available height.
   */
  fillMaxHeight?: boolean;
  /**
   * Uniform or axis-specific space between node bounds and content.
   */
  padding?: ComposeCommonPropsPadding;
  /**
   * Content inset on the layout-direction start edge.
   */
  paddingStart?: number;
  /**
   * Content inset on the top edge.
   */
  paddingTop?: number;
  /**
   * Content inset on the layout-direction end edge.
   */
  paddingEnd?: number;
  /**
   * Content inset applied to both start and end edges.
   */
  paddingHorizontal?: number;
  /**
   * Content inset applied to both top and bottom edges.
   */
  paddingVertical?: number;
  /**
   * Content inset on the bottom edge.
   */
  paddingBottom?: number;
  /**
   * Gap inserted between adjacent child nodes.
   */
  spacing?: number;
  /**
   * Whether the node expands to the maximum available width.
   */
  fillMaxWidth?: boolean;
  /**
   * Whether the node expands to both maximum available dimensions.
   */
  fillMaxSize?: boolean;
  /**
   * Solid background paint behind node content.
   */
  background?: ComposeColor;
  /**
   * Alternate explicit background color accepted by legacy component props.
   */
  backgroundColor?: ComposeColor;
  /**
   * Material container color used by surface-like components.
   */
  containerColor?: ComposeColor;
  /**
   * Opacity multiplier applied to background paint.
   */
  backgroundAlpha?: number;
  /**
   * Gradient brush used instead of a solid background color.
   */
  backgroundBrush?: ComposeCanvasBrush;
  /**
   * Shape that bounds and optionally clips the background paint.
   */
  backgroundShape?: ComposeShape;
}

/**
 * Properties for laying out child nodes vertically.
 */
export interface ColumnProps extends ComposeCommonProps {
  /**
   * Child nodes placed from top to bottom.
   */
  content?: ComposeChildren;
  /**
   * Cross-axis alignment of children within the column width.
   */
  horizontalAlignment?: ComposeAlignment;
  /**
   * Main-axis distribution of children within the column height.
   */
  verticalArrangement?: ComposeArrangement;
}

/**
 * Properties for laying out child nodes horizontally.
 */
export interface RowProps extends ComposeCommonProps {
  /**
   * Child nodes placed from start to end.
   */
  content?: ComposeChildren;
  /**
   * Main-axis distribution of children within the row width.
   */
  horizontalArrangement?: ComposeArrangement;
  /**
   * Cross-axis alignment of children within the row height.
   */
  verticalAlignment?: ComposeAlignment;
  /**
   * Optional click action for the row as one interactive target.
   */
  onClick?: () => RowPropsOnClickOutput;
}

/**
 * Properties for stacking child nodes in the same layout bounds.
 */
export interface BoxProps extends ComposeCommonProps {
  /**
   * Child nodes drawn in stacking order.
   */
  content?: ComposeChildren;
  /**
   * Default placement of children within the box bounds.
   */
  contentAlignment?: ComposeAlignment;
}

/**
 * Empty layout element that reserves explicit width and height.
 */
export interface SpacerProps {
  /**
   * Horizontal space reserved by the element.
   */
  width?: number;
  /**
   * Vertical space reserved by the element.
   */
  height?: number;
}

/**
 * Properties for rendering one styled block of plain text.
 */
export interface TextProps extends ComposeCommonProps {
  /**
   * String rendered by the text layout.
   */
  text: string;
  /**
   * Material typography role supplying default metrics.
   */
  style?: ComposeTextStyle;
  /**
   * Foreground color of rendered glyphs.
   */
  color?: ComposeColor;
  /**
   * Font weight override applied after the typography role.
   */
  fontWeight?: string;
  /**
   * Font size override applied after the typography role.
   */
  fontSize?: number;
  /**
   * Font family override used to resolve glyphs.
   */
  fontFamily?: string;
  /**
   * Maximum number of lines before overflow handling.
   */
  maxLines?: number;
  /**
   * Whether text may wrap at soft line-break opportunities.
   */
  softWrap?: boolean;
  /**
   * Rendering behavior for text beyond the available lines or bounds.
   */
  overflow?: ComposeTextOverflow;
  /**
   * Share of remaining main-axis space when placed in a row or column.
   */
  weight?: number;
}

/**
 * Properties for parsing and rendering Markdown content.
 */
export interface MarkdownProps extends ComposeCommonProps {
  /**
   * Markdown source rendered by the component.
   */
  text: string;
  /**
   * Default foreground color for Markdown text.
   */
  color?: ComposeColor;
  /**
   * Base font size from which Markdown styles are derived.
   */
  fontSize?: number;
  /**
   * Whether links or embedded actions may open host dialogs.
   */
  enableDialogs?: boolean;
  /**
   * Tag used to associate incremental Markdown updates with one stream.
   */
  streamTagName?: string;
}

/**
 * State, decoration, validation, and typography for an editable text field.
 */
export interface TextFieldProps extends ComposeCommonProps {
  /**
   * Plain or composed label displayed with the field.
   */
  label?: TextFieldPropsLabel;
  /**
   * Plain or composed hint shown while the value is empty.
   */
  placeholder?: TextFieldPropsPlaceholder;
  /**
   * Decoration placed before the editable text.
   */
  leadingIcon?: ComposeChildren;
  /**
   * Decoration placed after the editable text.
   */
  trailingIcon?: ComposeChildren;
  /**
   * Content rendered immediately before the entered value.
   */
  prefix?: ComposeChildren;
  /**
   * Content rendered immediately after the entered value.
   */
  suffix?: ComposeChildren;
  /**
   * Helper or validation content rendered beneath the field.
   */
  supportingText?: ComposeChildren;
  /**
   * Current controlled text value.
   */
  value: string;
  /**
   * Receives each user-proposed value for controlled-state updates.
   */
  onValueChange: (arg0: string) => void;
  /**
   * Restricts input and layout to one visual line.
   */
  singleLine?: boolean;
  /**
   * Minimum visible line count reserved by the field.
   */
  minLines?: number;
  /**
   * Maximum visible line count before internal scrolling.
   */
  maxLines?: number;
  /**
   * Allows selection without accepting user edits.
   */
  readOnly?: boolean;
  /**
   * Applies error-state semantics and styling.
   */
  isError?: boolean;
  /**
   * Obscures entered characters as sensitive password input.
   */
  isPassword?: boolean;
  /**
   * Typography and foreground overrides for the entered value.
   */
  style?: ComposeTextFieldStyle;
}

/**
 * Controlled state and colors for a binary sliding switch.
 */
export interface SwitchProps extends ComposeCommonProps {
  /**
   * Current on or off state.
   */
  checked: boolean;
  /**
   * Receives the state requested by user interaction.
   */
  onCheckedChange: (arg0: boolean) => void;
  /**
   * Whether the switch accepts pointer and accessibility actions.
   */
  enabled?: boolean;
  /**
   * Optional content rendered inside the movable thumb.
   */
  thumbContent?: ComposeChildren;
  /**
   * Thumb color while the switch is on.
   */
  checkedThumbColor?: ComposeColor;
  /**
   * Track color while the switch is on.
   */
  checkedTrackColor?: ComposeColor;
  /**
   * Thumb color while the switch is off.
   */
  uncheckedThumbColor?: ComposeColor;
  /**
   * Track color while the switch is off.
   */
  uncheckedTrackColor?: ComposeColor;
}

/**
 * Controlled state and interaction for a binary checkbox.
 */
export interface CheckboxProps extends ComposeCommonProps {
  /**
   * Current selected or unselected state.
   */
  checked: boolean;
  /**
   * Receives the state requested by user interaction.
   */
  onCheckedChange: (arg0: boolean) => void;
  /**
   * Whether the checkbox accepts pointer and accessibility actions.
   */
  enabled?: boolean;
}

/**
 * Content, shape, state, and action for a standard Material button.
 */
export interface ButtonProps extends ComposeCommonProps {
  /**
   * Arbitrary composed content rendered inside the button.
   */
  content?: ComposeChildren;
  /**
   * Plain label used when custom content is unnecessary.
   */
  text?: string;
  /**
   * Whether the button accepts pointer and accessibility actions.
   */
  enabled?: boolean;
  /**
   * Action invoked when the button is activated.
   */
  onClick: () => ButtonPropsOnClickOutput;
  /**
   * Horizontal and vertical inset around the button content.
   */
  contentPadding?: ComposePadding;
  /**
   * Outline used for button background, border, and clipping.
   */
  shape?: ComposeShape;
}

/**
 * Icon or custom content rendered as a compact clickable control.
 */
export interface IconButtonProps extends ComposeCommonProps {
  /**
   * Arbitrary content rendered inside the icon-button bounds.
   */
  content?: ComposeChildren;
  /**
   * Material icon name used when custom content is absent.
   */
  icon?: string;
  /**
   * Whether the control accepts pointer and accessibility actions.
   */
  enabled?: boolean;
  /**
   * Action invoked when the icon button is activated.
   */
  onClick: () => IconButtonPropsOnClickOutput;
  /**
   * Outline used for interaction feedback and clipping.
   */
  shape?: ComposeShape;
}

/**
 * Elevated or outlined Material container for grouped content.
 */
export interface CardProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the card container.
   */
  content?: ComposeChildren;
  /**
   * Background color of the card surface.
   */
  containerColor?: ComposeColor;
  /**
   * Opacity multiplier applied to the card background.
   */
  containerAlpha?: number;
  /**
   * Default foreground color inherited by card content.
   */
  contentColor?: ComposeColor;
  /**
   * Opacity multiplier inherited by card content.
   */
  contentAlpha?: number;
  /**
   * Outline used for card background, border, and clipping.
   */
  shape?: ComposeShape;
  /**
   * Optional stroke around the card outline.
   */
  border?: ComposeBorder;
  /**
   * Shadow elevation separating the card from its background.
   */
  elevation?: number;
}

/**
 * Material surface that supplies container color, content color, and shape.
 */
export interface SurfaceProps extends ComposeCommonProps {
  /**
   * Child nodes rendered inside the surface.
   */
  content?: ComposeChildren;
  /**
   * Background color of the surface.
   */
  containerColor?: ComposeColor;
  /**
   * Default foreground color inherited by surface content.
   */
  contentColor?: ComposeColor;
  /**
   * Outline used for surface painting, clipping, and interaction feedback.
   */
  shape?: ComposeShape;
  /**
   * Opacity multiplier applied to the surface.
   */
  alpha?: number;
  /**
   * Optional action that turns the surface into an interactive target.
   */
  onClick?: () => SurfacePropsOnClickOutput;
}

/**
 * Properties for rendering and optionally animating a Material icon.
 */
export interface IconProps extends ComposeCommonProps {
  /**
   * Material icon name resolved by the host icon registry.
   */
  name?: string;
  /**
   * Color applied to the icon glyph.
   */
  tint?: ComposeColor;
  /**
   * Width and height of the square icon bounds.
   */
  size?: number;
  /**
   * Whether the icon continuously rotates as a loading affordance.
   */
  spin?: boolean;
  /**
   * Duration in milliseconds of one full rotation.
   */
  spinDurationMs?: number;
}

/**
 * Vertically scrolling list whose child content is hosted by a lazy column.
 */
export interface LazyColumnProps extends ComposeCommonProps {
  /**
   * List content rendered in vertical order.
   */
  content?: ComposeChildren;
  /**
   * Vertical gap between adjacent list children.
   */
  spacing?: number;
}

/**
 * Properties for a horizontal determinate or indeterminate progress indicator.
 */
export interface LinearProgressIndicatorProps extends ComposeCommonProps {
  /**
   * Determinate completion fraction; omission selects indeterminate animation.
   */
  progress?: number;
}

/**
 * Appearance of an indeterminate circular progress indicator.
 */
export interface CircularProgressIndicatorProps extends ComposeCommonProps {
  /**
   * Thickness of the animated circular arc.
   */
  strokeWidth?: number;
  /**
   * Color used to paint the animated arc.
   */
  color?: ComposeColor;
}

/**
 * Host slot where queued snackbar messages are presented.
 */
export interface SnackbarHostProps extends ComposeCommonProps {
}

/**
 * Drawing commands, viewport transform, and gesture callbacks for a canvas node.
 */
export interface CanvasProps extends ComposeCommonProps {
  /**
   * Drawing operations rendered sequentially into the canvas.
   */
  commands?: ComposeCanvasCommand[];
  /**
   * Scale, translation, and pivot applied to canvas content.
   */
  transform?: ComposeCanvasTransform;
  /**
   * Receives incremental pan, zoom, and rotation gestures over the canvas.
   */
  onTransform?: (arg0: ComposeCanvasTransformEvent) => void;
  /**
   * Receives measured canvas dimensions after layout changes.
   */
  onSizeChanged?: (arg0: ComposeCanvasSizeEvent) => void;
}

/**
 * Content source, platform settings, controller, and event hooks for an embedded WebView.
 */
export interface WebViewProps extends ComposeCommonProps {
  /**
   * Remote or local URL loaded as the primary document.
   */
  url?: string;
  /**
   * Inline HTML loaded instead of a URL.
   */
  html?: string;
  /**
   * Origin and relative-link base for inline HTML content.
   */
  baseUrl?: string;
  /**
   * Media type assigned to inline content.
   */
  mimeType?: string;
  /**
   * Character encoding used to decode inline content.
   */
  encoding?: string;
  /**
   * Additional HTTP headers attached to the initial URL request.
   */
  headers?: Record<string, string>;
  /**
   * Whether page scripts may execute.
   */
  javaScriptEnabled?: boolean;
  /**
   * Whether DOM local and session storage APIs are available.
   */
  domStorageEnabled?: boolean;
  /**
   * Whether page database storage is enabled by the platform WebView.
   */
  databaseEnabled?: boolean;
  /**
   * Whether scripts may open windows without a user gesture.
   */
  javaScriptCanOpenWindowsAutomatically?: boolean;
  /**
   * Whether the WebView handles requests for additional browser windows.
   */
  supportMultipleWindows?: boolean;
  /**
   * Whether pages may read resources through file URLs.
   */
  allowFileAccess?: boolean;
  /**
   * Whether pages may access platform content-provider URLs.
   */
  allowContentAccess?: boolean;
  /**
   * Whether a file-origin page may read other file URLs.
   */
  allowFileAccessFromFileURLs?: boolean;
  /**
   * Whether a file-origin page may request resources from any origin.
   */
  allowUniversalAccessFromFileURLs?: boolean;
  /**
   * User-Agent string sent with WebView requests.
   */
  userAgent?: string;
  /**
   * Whether WebView scrolling participates in the surrounding Compose scroll chain.
   */
  nestedScrollInterop?: boolean;
  /**
   * Whether the page supports zoom gestures and controls.
   */
  supportZoom?: boolean;
  /**
   * Whether platform-provided zoom controls are enabled.
   */
  builtInZoomControls?: boolean;
  /**
   * Whether platform zoom controls are visibly overlaid on the page.
   */
  displayZoomControls?: boolean;
  /**
   * Whether layout uses a wide viewport based on page metadata.
   */
  useWideViewPort?: boolean;
  /**
   * Whether an oversized page initially scales down to fit the viewport.
   */
  loadWithOverviewMode?: boolean;
  /**
   * Policy for HTTP subresources requested by HTTPS pages.
   */
  mixedContentMode?: ComposeWebViewMixedContentMode;
  /**
   * Whether audio and video playback must begin from a user gesture.
   */
  mediaPlaybackRequiresUserGesture?: boolean;
  /**
   * Percentage scale applied to page text independently of page zoom.
   */
  textZoom?: number;
  /**
   * Network cache policy for page and resource requests.
   */
  cacheMode?: ComposeWebViewCacheMode;
  /**
   * Whether platform safe-browsing checks protect navigation.
   */
  safeBrowsingEnabled?: boolean;
  /**
   * Whether embedded third-party origins may store and send cookies.
   */
  acceptThirdPartyCookies?: boolean;
  /**
   * Imperative handle associated with this WebView across renders.
   */
  controller?: ComposeWebViewController;
  /**
   * Runs when the main page begins navigating.
   */
  onPageStarted?: (arg0: ComposeWebViewPageEvent) => WebViewPropsOnPageStartedOutput;
  /**
   * Runs when the main page finishes loading.
   */
  onPageFinished?: (arg0: ComposeWebViewPageEvent) => WebViewPropsOnPageFinishedOutput;
  /**
   * Runs for network or content-loading failures.
   */
  onReceivedError?: (arg0: ComposeWebViewErrorEvent) => WebViewPropsOnReceivedErrorOutput;
  /**
   * Runs when a requested resource returns an HTTP error status.
   */
  onReceivedHttpError?: (arg0: ComposeWebViewHttpErrorEvent) => WebViewPropsOnReceivedHttpErrorOutput;
  /**
   * Runs when TLS certificate validation fails.
   */
  onReceivedSslError?: (arg0: ComposeWebViewSslErrorEvent) => WebViewPropsOnReceivedSslErrorOutput;
  /**
   * Runs when page content initiates a file download.
   */
  onDownloadStart?: (arg0: ComposeWebViewDownloadEvent) => WebViewPropsOnDownloadStartOutput;
  /**
   * Receives messages emitted by the page's JavaScript console.
   */
  onConsoleMessage?: (arg0: ComposeWebViewConsoleEvent) => WebViewPropsOnConsoleMessageOutput;
  /**
   * Runs when the active or requested URL changes.
   */
  onUrlChanged?: (arg0: ComposeWebViewNavigationEvent) => WebViewPropsOnUrlChangedOutput;
  /**
   * Runs as page loading completion advances.
   */
  onProgressChanged?: (arg0: ComposeWebViewProgressEvent) => WebViewPropsOnProgressChangedOutput;
  /**
   * Receives complete navigation and loading state snapshots.
   */
  onStateChanged?: (arg0: ComposeWebViewState) => WebViewPropsOnStateChangedOutput;
  /**
   * Receives native WebView creation, disposal, commit, and renderer events.
   */
  onLifecycleEvent?: (arg0: ComposeWebViewLifecycleEvent) => WebViewPropsOnLifecycleEventOutput;
  /**
   * Selects whether a navigation is allowed, cancelled, rewritten, or opened externally.
   */
  onShouldOverrideUrlLoading?: (arg0: ComposeWebViewNavigationRequest) => WebViewPropsOnShouldOverrideUrlLoadingOutput | null | undefined;
  /**
   * Selects whether a resource request is allowed, blocked, rewritten, or answered locally.
   */
  onInterceptRequest?: (arg0: ComposeWebViewResourceRequest) => WebViewPropsOnInterceptRequestOutput | null | undefined;
}

/**
 * Serializable element in the Compose DSL render tree.
 */
export interface ComposeNode {
  /**
   * Component factory name resolved by the host renderer.
   */
  type: string;
  /**
   * Component-specific properties encoded for the selected node type.
   */
  props?: Record<string, unknown>;
  /**
   * Ordered descendant nodes rendered inside this element.
   */
  children?: ComposeNode[];
}

/**
 * Child content accepted by a Compose node factory.
 */
export type ComposeChildren = ComposeNode | ComposeNode[] | null | undefined;

/**
 * Factory that encodes component properties and children as a Compose node.
 */
export type ComposeNodeFactory<TProps = Record<string, unknown>> = (arg0?: TProps, arg1?: ComposeChildren) => ComposeNode;

/**
 * Core component factories available through `ComposeDslContext::UI`.
 */
export interface ComposeUiFactoryRegistry {
  /**
   * Creates a vertical layout container.
   */
  Column: ComposeNodeFactory<ColumnProps>;
  /**
   * Creates a horizontal layout container.
   */
  Row: ComposeNodeFactory<RowProps>;
  /**
   * Creates a stacking layout container.
   */
  Box: ComposeNodeFactory<BoxProps>;
  /**
   * Creates an empty element that reserves layout space.
   */
  Spacer: ComposeNodeFactory<SpacerProps>;
  /**
   * Creates a plain text element.
   */
  Text: ComposeNodeFactory<TextProps>;
  /**
   * Creates a Markdown-rendering element.
   */
  Markdown: ComposeNodeFactory<MarkdownProps>;
  /**
   * Creates a controlled editable text field.
   */
  TextField: ComposeNodeFactory<TextFieldProps>;
  /**
   * Creates a controlled binary switch.
   */
  Switch: ComposeNodeFactory<SwitchProps>;
  /**
   * Creates a controlled checkbox.
   */
  Checkbox: ComposeNodeFactory<CheckboxProps>;
  /**
   * Creates a standard Material button.
   */
  Button: ComposeNodeFactory<ButtonProps>;
  /**
   * Creates a compact icon button.
   */
  IconButton: ComposeNodeFactory<IconButtonProps>;
  /**
   * Creates an elevated or outlined card container.
   */
  Card: ComposeNodeFactory<CardProps>;
  /**
   * Creates a Material surface container.
   */
  Surface: ComposeNodeFactory<SurfaceProps>;
  /**
   * Creates a Material icon element.
   */
  Icon: ComposeNodeFactory<IconProps>;
  /**
   * Creates a vertically scrolling lazy list.
   */
  LazyColumn: ComposeNodeFactory<LazyColumnProps>;
  /**
   * Creates a horizontal progress indicator.
   */
  LinearProgressIndicator: ComposeNodeFactory<LinearProgressIndicatorProps>;
  /**
   * Creates a circular progress indicator.
   */
  CircularProgressIndicator: ComposeNodeFactory<CircularProgressIndicatorProps>;
  /**
   * Creates the presentation slot for queued snackbars.
   */
  SnackbarHost: ComposeNodeFactory<SnackbarHostProps>;
  /**
   * Creates a command-driven drawing surface.
   */
  Canvas: ComposeNodeFactory<CanvasProps>;
  /**
   * Creates an embedded platform WebView.
   */
  WebView: ComposeNodeFactory<WebViewProps>;
}

/**
 * Named scalar substitutions used by runtime message templates.
 */
export interface ComposeTemplateValues {
  [key: string]: ComposeTemplateValuesAdditionalValue | null | undefined;
}

/**
 * Runtime-module metadata available to a Compose DSL screen.
 */
export interface ComposeUiModuleSpec {
  /**
   * Identifier of the registered UI module.
   */
  id?: string;
  /**
   * Runtime implementation selected for the module.
   */
  runtime?: string;
  [key: string]: unknown;
}

/**
 * Object-form request for invoking a tool from a Compose screen.
 */
export interface ComposeToolCallConfig {
  /**
   * Optional tool category or namespace used during resolution.
   */
  type?: string;
  /**
   * Tool name requested by the screen.
   */
  name: string;
  /**
   * Named arguments passed to the tool implementation.
   */
  params?: Record<string, unknown>;
}

/**
 * Package context used to resolve a callable runtime tool name.
 */
export interface ComposeResolveToolNameRequest {
  /**
   * Package that owns or imports the requested tool.
   */
  packageName?: string;
  /**
   * Optional subpackage containing the requested tool.
   */
  subpackageId?: string;
  /**
   * Public tool name to resolve.
   */
  toolName: string;
  /**
   * Whether an imported package binding should win over a local definition.
   */
  preferImported?: boolean;
}

/**
 * Selection filters and permission behavior for the host file picker.
 */
export interface ComposeFilePickerOptions {
  /**
   * Accepted media types shown by the picker.
   */
  mimeTypes?: string[];
  /**
   * Whether the user may select more than one file.
   */
  allowMultiple?: boolean;
  /**
   * Whether the host should retain URI access beyond the current session.
   */
  persistPermission?: boolean;
}

/**
 * File metadata returned by the host picker.
 */
export interface ComposePickedFile {
  /**
   * Platform URI granting access to the selected content.
   */
  uri: string;
  /**
   * Resolved filesystem path when the provider exposes one.
   */
  path?: string;
  /**
   * Display name reported by the content provider.
   */
  name?: string;
  /**
   * Media type reported for the selected file.
   */
  mimeType?: string;
  /**
   * File size in bytes when reported by the provider.
   */
  size?: number | null;
}

/**
 * Outcome of a host file-picker request.
 */
export interface ComposeFilePickerResult {
  /**
   * Whether the picker closed without an accepted selection.
   */
  cancelled: boolean;
  /**
   * Files selected by the user; empty when the request was cancelled.
   */
  files: ComposePickedFile[];
}

/**
 * Discoverable navigation target exposed to a Compose DSL screen.
 */
export interface ComposeRouteInfo {
  /**
   * Stable route identifier accepted by `navigate`.
   */
  routeId: string;
  /**
   * Runtime responsible for rendering the destination.
   */
  runtime: string;
  /**
   * Human-readable destination title when available.
   */
  title?: string | null;
  /**
   * Package that owns the destination.
   */
  ownerPackageName?: string | null;
  /**
   * Tool-package UI module rendered by the destination.
   */
  toolPkgUiModuleId?: string | null;
}

/**
 * Theme, modifier builder, and component factories supplied to a screen renderer.
 */
export interface ComposeDslContext {
  /**
   * Active Material theme values resolved by the host.
   */
  MaterialTheme: ComposeMaterialTheme;
  /**
   * Empty modifier chain from which node modifiers are built.
   */
  Modifier: ComposeModifierProxy;
  /**
   * Core, Material 3, and host-registered component factories.
   */
  UI: ComposeDslContextUIIntersection;
  /**
   * Retains a keyed value across renders and returns a setter that schedules updates.
   */
  useState<T>(key: string, initialValue: T): [T, (arg0: T) => void];
  /**
   * Retains a keyed mutable value and returns a callback for replacing it.
   */
  useMutable<T>(key: string, initialValue: T): [T, (arg0: T) => void];
  /**
   * Returns a keyed stable reference whose current value survives rerenders.
   */
  useRef<T>(key: string, initialValue: T): ComposeDslContextMethodsUseRefReturn<T>;
  /**
   * Reuses a keyed computed value until its dependency list changes.
   */
  useMemo<T>(key: string, factory: () => T, deps?: unknown[]): T;
  /**
   * Measures text using host font metrics and the supplied layout constraints.
   */
  measureText(request: ComposeTextMeasureRequest): ComposeTextMeasureResult;
  /**
   * Invokes a resolved tool by name and asynchronously decodes its result.
   */
  callTool<T>(toolName: string, params?: Record<string, unknown>): Promise<T>;
  /**
   * Reads one runtime environment value by key.
   */
  getEnv(key: string): string | undefined;
  /**
   * Writes one runtime environment value for subsequent screen operations.
   */
  setEnv(key: string, value: string): ComposeDslContextMethodsSetEnvReturn;
  /**
   * Requests navigation to a route with optional destination arguments.
   */
  navigate(route: string, args?: Record<string, unknown>): ComposeDslContextMethodsNavigateReturn;
  /**
   * Presents a short host toast message.
   */
  showToast(message: string): ComposeDslContextMethodsShowToastReturn;
  /**
   * Forwards a screen error to host diagnostics and user-facing handling.
   */
  reportError(error: unknown): ComposeDslContextMethodsReportErrorReturn;
  /**
   * Creates or retrieves the keyed imperative controller for a WebView node.
   */
  createWebViewController(key: string): ComposeWebViewController;
  /**
   * Opens the host file picker and resolves selected file metadata.
   */
  openFilePicker(options?: ComposeFilePickerOptions): Promise<ComposeFilePickerResult>;
  /**
   * Returns the registered runtime-module specification decoded as the requested type.
   */
  getModuleSpec<TSpec>(): TSpec;
  /**
   * Returns the package name that owns the current Compose DSL module.
   */
  getCurrentPackageName(): string | undefined;
  /**
   * Returns the identifier of the active tool package.
   */
  getCurrentToolPkgId(): string | undefined;
  /**
   * Returns the identifier of the UI module currently being rendered.
   */
  getCurrentUiModuleId(): string | undefined;
  /**
   * Interpolates named scalar values into placeholders in a message template.
   */
  formatTemplate(template: string, values: ComposeTemplateValues): string;
  /**
   * Writes a batch of runtime environment entries.
   */
  setEnvs(values: Record<string, string>): ComposeDslContextMethodsSetEnvsReturn;
  /**
   * Lists navigation targets currently available to the screen.
   */
  listRoutes(): ComposeRouteInfo[];
  /**
   * Lists routes owned directly by the host rather than plugin packages.
   */
  getHostRoutes(): ComposeRouteInfo[];
  /**
   * Calls a tool by resolved name using package-tool compatible parameters.
   */
  toolCall<T>(toolName: string, toolParams?: Record<string, unknown>): Promise<T>;
  /**
   * Calls a tool selected by explicit category and name.
   */
  toolCall<T>(toolType: string, toolName: string, toolParams?: Record<string, unknown>): Promise<T>;
  /**
   * Calls a tool using an object-form request.
   */
  toolCall<T>(config: ComposeToolCallConfig): Promise<T>;
  /**
   * Reports whether the named package is already imported into this runtime.
   */
  isPackageImported(packageName: string): ComposeDslContextMethodsIsPackageImportedReturn;
  /**
   * Imports a package and returns the resulting package identifier.
   */
  importPackage(packageName: string): ComposeDslContextMethodsImportPackageReturn;
  /**
   * Removes an imported package and returns the removed identifier.
   */
  removePackage(packageName: string): ComposeDslContextMethodsRemovePackageReturn;
  /**
   * Selects an imported package for subsequent resolution and returns its identifier.
   */
  usePackage(packageName: string): ComposeDslContextMethodsUsePackageReturn;
  /**
   * Lists package names imported into the current runtime.
   */
  listImportedPackages(): ComposeDslContextMethodsListImportedPackagesReturn;
  /**
   * Resolves a package-scoped public tool name to its callable runtime name.
   */
  resolveToolName(request: ComposeResolveToolNameRequest): ComposeDslContextMethodsResolveToolNameReturn;
  /**
   * Constructs a raw Compose node for a component name, props, and children.
   */
  h<TProps>(type: string, props?: TProps, children?: ComposeChildren): ComposeNode;
}

/**
 * Screen renderer that receives a host context and produces a root node tree.
 */
export type ComposeDslScreen = (arg0: ComposeDslContext) => ComposeDslScreenOutput;

declare global {
  /**
   * Defines numeric unit accessors installed on the JavaScript `Number` interface.
   */
  export interface Number {
    /**
     * Converts the number to a pixel unit value.
     */
    px: ComposeUnitValue;
    /**
     * Converts the number to a density-independent pixel unit value.
     */
    dp: ComposeUnitValue;
    /**
     * Converts the number to a fractional unit value.
     */
    fraction: ComposeUnitValue;
  }

}
