//! Types and host contracts for constructing Compose UI trees in plugin runtime modules.
use super::compose_dsl_material3_generated::*;
use super::{JsDate, JsFuture, JsOptional};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
/// Options for deriving a theme color token with adjusted opacity.
pub struct ComposeColorTokenMethodsCopyOptions {
    /// Opacity multiplier applied to the derived token.
    pub alpha: f64,
}
/// Gradient direction supported by a canvas brush.
pub enum ComposeCanvasBrushType {
    /// Interpolates colors from the top edge toward the bottom edge.
    VerticalGradient,
}
/// Completion mode returned by the primary-click handler.
pub enum ComposeModifierCombinedClickableOptionsOnClickOutput {
    /// The handler completed during the callback.
    Variant1(()),
    /// The handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned by the long-click handler.
pub enum ComposeModifierCombinedClickableOptionsOnLongClickOutput {
    /// The handler completed during the callback.
    Variant1(()),
    /// The handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned by the double-click handler.
pub enum ComposeModifierCombinedClickableOptionsOnDoubleClickOutput {
    /// The handler completed during the callback.
    Variant1(()),
    /// The handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned after a pointer press is recognized.
pub enum ComposeModifierTapGesturesOptionsOnPressOutput {
    /// The press handler completed immediately.
    Variant1(()),
    /// The press handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned after a tap is recognized.
pub enum ComposeModifierTapGesturesOptionsOnTapOutput {
    /// The tap handler completed immediately.
    Variant1(()),
    /// The tap handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned after a double tap is recognized.
pub enum ComposeModifierTapGesturesOptionsOnDoubleTapOutput {
    /// The double-tap handler completed immediately.
    Variant1(()),
    /// The double-tap handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned after a long press is recognized.
pub enum ComposeModifierTapGesturesOptionsOnLongPressOutput {
    /// The long-press handler completed immediately.
    Variant1(()),
    /// The long-press handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned when a drag begins.
pub enum ComposeModifierDragGesturesOptionsOnDragStartOutput {
    /// The drag-start handler completed immediately.
    Variant1(()),
    /// The drag-start handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned for each drag update.
pub enum ComposeModifierDragGesturesOptionsOnDragOutput {
    /// The drag-update handler completed immediately.
    Variant1(()),
    /// The drag-update handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned when a drag ends normally.
pub enum ComposeModifierDragGesturesOptionsOnDragEndOutput {
    /// The drag-end handler completed immediately.
    Variant1(()),
    /// The drag-end handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned when a drag is cancelled.
pub enum ComposeModifierDragGesturesOptionsOnDragCancelOutput {
    /// The cancellation handler completed immediately.
    Variant1(()),
    /// The cancellation handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned for a multi-touch transform update.
pub enum ComposeModifierTransformGesturesOptionsOnGestureOutput {
    /// The transform handler completed immediately.
    Variant1(()),
    /// The transform handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Lifecycle transition reported by an embedded WebView.
pub enum ComposeWebViewLifecycleEventType {
    /// The native WebView instance was created.
    Created,
    /// The native WebView instance was released.
    Disposed,
    /// New page content became visible after navigation committed.
    PageCommitVisible,
    /// The WebView renderer process exited or crashed.
    RenderProcessGone,
}
/// Discriminator for allowing a WebView navigation unchanged.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeWebViewNavigationDecisionVariant1Action {
    /// Lets the WebView continue with the requested navigation.
    #[serde(rename = "allow")]
    Allow,
}
/// Navigation decision that permits the original request.
pub struct ComposeWebViewNavigationDecisionVariant1 {
    /// Selects the allow branch of the navigation decision.
    pub action: ComposeWebViewNavigationDecisionVariant1Action,
}
/// Discriminator for cancelling a WebView navigation.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeWebViewNavigationDecisionVariant2Action {
    /// Stops the requested navigation.
    #[serde(rename = "cancel")]
    Cancel,
}
/// Navigation decision that prevents the requested page from loading.
pub struct ComposeWebViewNavigationDecisionVariant2 {
    /// Selects the cancel branch of the navigation decision.
    pub action: ComposeWebViewNavigationDecisionVariant2Action,
}
/// Discriminator for replacing a WebView navigation request.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeWebViewNavigationDecisionVariant3Action {
    /// Loads a replacement URL instead of the original request.
    #[serde(rename = "rewrite")]
    Rewrite,
}
/// Navigation decision that redirects the WebView to a replacement request.
pub struct ComposeWebViewNavigationDecisionVariant3 {
    /// Selects the rewrite branch of the navigation decision.
    pub action: ComposeWebViewNavigationDecisionVariant3Action,
    /// Replacement URL loaded by the WebView.
    pub url: String,
    /// HTTP headers attached to the replacement request.
    pub headers: Option<BTreeMap<String, String>>,
}
/// Discriminator for handing navigation to an external application.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeWebViewNavigationDecisionVariant4Action {
    /// Opens the target outside the embedded WebView.
    #[serde(rename = "external")]
    External,
}
/// Navigation decision that delegates a target to the host platform.
pub struct ComposeWebViewNavigationDecisionVariant4 {
    /// Selects the external-navigation branch.
    pub action: ComposeWebViewNavigationDecisionVariant4Action,
    /// Target to open externally, or the original request when omitted.
    pub url: Option<String>,
}
/// Synthetic WebView resource response backed by decoded text.
pub struct ComposeWebViewResourceResponseVariant1 {
    /// Media type reported for the response body.
    pub mimeType: Option<String>,
    /// Character encoding used to convert the text body to bytes.
    pub encoding: Option<String>,
    /// HTTP status code exposed to the page.
    pub statusCode: Option<f64>,
    /// HTTP reason phrase paired with the status code.
    pub reasonPhrase: Option<String>,
    /// Response headers exposed to the page.
    pub headers: Option<BTreeMap<String, String>>,
    /// Decoded text used as the response body.
    pub text: String,
    /// Uninhabited marker that excludes a base64 body from this branch.
    pub base64: Option<super::JsNever>,
    /// Uninhabited marker that excludes a file body from this branch.
    pub filePath: Option<super::JsNever>,
}
/// Synthetic WebView resource response backed by base64-encoded bytes.
pub struct ComposeWebViewResourceResponseVariant2 {
    /// Media type reported for the decoded response bytes.
    pub mimeType: Option<String>,
    /// Character encoding metadata reported with the response.
    pub encoding: Option<String>,
    /// HTTP status code exposed to the page.
    pub statusCode: Option<f64>,
    /// HTTP reason phrase paired with the status code.
    pub reasonPhrase: Option<String>,
    /// Response headers exposed to the page.
    pub headers: Option<BTreeMap<String, String>>,
    /// Base64-encoded bytes used as the response body.
    pub base64: String,
    /// Uninhabited marker that excludes a text body from this branch.
    pub text: Option<super::JsNever>,
    /// Uninhabited marker that excludes a file body from this branch.
    pub filePath: Option<super::JsNever>,
}
/// Synthetic WebView resource response streamed from a local file.
pub struct ComposeWebViewResourceResponseVariant3 {
    /// Media type reported for the file contents.
    pub mimeType: Option<String>,
    /// Character encoding metadata reported with the response.
    pub encoding: Option<String>,
    /// HTTP status code exposed to the page.
    pub statusCode: Option<f64>,
    /// HTTP reason phrase paired with the status code.
    pub reasonPhrase: Option<String>,
    /// Response headers exposed to the page.
    pub headers: Option<BTreeMap<String, String>>,
    /// Local path whose contents become the response body.
    pub filePath: String,
    /// Uninhabited marker that excludes a text body from this branch.
    pub text: Option<super::JsNever>,
    /// Uninhabited marker that excludes a base64 body from this branch.
    pub base64: Option<super::JsNever>,
}
/// Discriminator for allowing a resource request unchanged.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeWebViewResourceDecisionVariant1Action {
    /// Lets the WebView fetch the original resource.
    #[serde(rename = "allow")]
    Allow,
}
/// Resource interception decision that permits the original request.
pub struct ComposeWebViewResourceDecisionVariant1 {
    /// Selects the allow branch of the resource decision.
    pub action: ComposeWebViewResourceDecisionVariant1Action,
}
/// Discriminator for blocking a resource request.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeWebViewResourceDecisionVariant2Action {
    /// Prevents the WebView from loading the resource.
    #[serde(rename = "block")]
    Block,
}
/// Resource interception decision that suppresses a request.
pub struct ComposeWebViewResourceDecisionVariant2 {
    /// Selects the block branch of the resource decision.
    pub action: ComposeWebViewResourceDecisionVariant2Action,
}
/// Discriminator for replacing a resource request.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeWebViewResourceDecisionVariant3Action {
    /// Fetches a replacement URL in place of the original resource.
    #[serde(rename = "rewrite")]
    Rewrite,
}
/// Resource interception decision that redirects a request.
pub struct ComposeWebViewResourceDecisionVariant3 {
    /// Selects the rewrite branch of the resource decision.
    pub action: ComposeWebViewResourceDecisionVariant3Action,
    /// Replacement URL used for the intercepted resource.
    pub url: String,
    /// HTTP headers attached to the replacement request.
    pub headers: Option<BTreeMap<String, String>>,
}
/// Discriminator for supplying a synthetic resource response.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeWebViewResourceDecisionVariant4Action {
    /// Completes the request with plugin-provided response data.
    #[serde(rename = "respond")]
    Respond,
}
/// Resource interception decision that returns plugin-provided content.
pub struct ComposeWebViewResourceDecisionVariant4 {
    /// Selects the synthetic-response branch.
    pub action: ComposeWebViewResourceDecisionVariant4Action,
    /// Response body and metadata returned to the page.
    pub response: ComposeWebViewResourceResponse,
}
/// Value returned by a JavaScript interface method exposed to WebView content.
pub enum ComposeWebViewJavascriptInterfaceMethodOutput {
    /// A result available before the callback returns.
    Variant1(serde_json::Value),
    /// A result produced asynchronously.
    Variant2(JsFuture<serde_json::Value>),
}
/// Discriminator for a straight-line canvas command.
pub enum ComposeCanvasLineCommandType {
    /// Draws a segment between two points.
    Line,
}
/// Discriminator for an axis-aligned rectangle canvas command.
pub enum ComposeCanvasRectCommandType {
    /// Draws or fills a rectangle.
    Rect,
}
/// Discriminator for a rounded-rectangle canvas command.
pub enum ComposeCanvasRoundRectCommandType {
    /// Draws or fills a rectangle with rounded corners.
    RoundRect,
}
/// Discriminator for a circular canvas command.
pub enum ComposeCanvasCircleCommandType {
    /// Draws or fills a circle.
    Circle,
}
/// Discriminator for a measured text canvas command.
pub enum ComposeCanvasTextCommandType {
    /// Draws text using the command's layout constraints.
    Text,
}
/// Discriminator for starting a path contour at a point.
pub enum ComposeCanvasMoveToOpType {
    /// Moves the current path position without drawing.
    MoveTo,
}
/// Discriminator for appending a straight segment to a path.
pub enum ComposeCanvasLineToOpType {
    /// Connects the current path position to the supplied point.
    LineTo,
}
/// Discriminator for appending a cubic Bezier segment to a path.
pub enum ComposeCanvasCubicToOpType {
    /// Uses two control points and an end point for the curve.
    CubicTo,
}
/// Discriminator for appending a quadratic Bezier segment to a path.
pub enum ComposeCanvasQuadToOpType {
    /// Uses one control point and an end point for the curve.
    QuadTo,
}
/// Discriminator for closing the current path contour.
pub enum ComposeCanvasCloseOpType {
    /// Connects the current point back to the contour's start.
    Close,
}
/// Discriminator for rendering a sequence of path operations.
pub enum ComposeCanvasDrawPathCommandType {
    /// Draws or fills the assembled path.
    DrawPath,
}
/// Discriminator for a draw-scope rounded rectangle command.
pub enum ComposeCanvasDrawRoundRectCommandType {
    /// Draws or fills a rounded rectangle in the current canvas scope.
    DrawRoundRect,
}
/// Discriminator for a draw-scope text command.
pub enum ComposeCanvasDrawTextCommandType {
    /// Draws text at the supplied canvas position.
    DrawText,
}
/// Discriminator for a draw-scope Material icon command.
pub enum ComposeCanvasDrawIconCommandType {
    /// Draws a named icon at the supplied canvas position.
    DrawIcon,
}
/// Completion mode returned by a modifier click handler.
pub enum ComposeModifierProxyClickableOnClickOutput {
    /// The click handler completed immediately.
    Variant1(()),
    /// The click handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned by a modifier size-change handler.
pub enum ComposeModifierProxyOnSizeChangedOnSizeChangedOutput {
    /// The size-change handler completed immediately.
    Variant1(()),
    /// The size-change handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned by a global-position handler.
pub enum ComposeModifierProxyOnGloballyPositionedOnGloballyPositionedOutput {
    /// The position handler completed immediately.
    Variant1(()),
    /// The position handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned when a composed node finishes loading.
pub enum ComposeCommonPropsOnLoadOutput {
    /// The load handler completed immediately.
    Variant1(()),
    /// The load handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Padding accepted as either one uniform inset or axis-specific insets.
pub enum ComposeCommonPropsPadding {
    /// Applies one inset to both axes.
    Variant1(f64),
    /// Applies independent horizontal and vertical insets.
    Variant2(ComposePadding),
}
/// Completion mode returned by a row click handler.
pub enum RowPropsOnClickOutput {
    /// The click handler completed immediately.
    Variant1(()),
    /// The click handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Label content accepted by a text field.
pub enum TextFieldPropsLabel {
    /// Renders a plain text label.
    Variant1(String),
    /// Renders an arbitrary composed label node.
    Variant2(ComposeChildren),
}
/// Placeholder content accepted by a text field.
pub enum TextFieldPropsPlaceholder {
    /// Renders plain placeholder text.
    Variant1(String),
    /// Renders arbitrary composed placeholder content.
    Variant2(ComposeChildren),
}
/// Completion mode returned by a button click handler.
pub enum ButtonPropsOnClickOutput {
    /// The click handler completed immediately.
    Variant1(()),
    /// The click handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned by an icon-button click handler.
pub enum IconButtonPropsOnClickOutput {
    /// The click handler completed immediately.
    Variant1(()),
    /// The click handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned by a clickable surface.
pub enum SurfacePropsOnClickOutput {
    /// The click handler completed immediately.
    Variant1(()),
    /// The click handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned when a WebView begins loading a page.
pub enum WebViewPropsOnPageStartedOutput {
    /// The page-start handler completed immediately.
    Variant1(()),
    /// The page-start handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned when a WebView finishes loading a page.
pub enum WebViewPropsOnPageFinishedOutput {
    /// The page-finished handler completed immediately.
    Variant1(()),
    /// The page-finished handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned after a WebView loading error is reported.
pub enum WebViewPropsOnReceivedErrorOutput {
    /// The error handler completed immediately.
    Variant1(()),
    /// The error handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned after an HTTP error response is reported.
pub enum WebViewPropsOnReceivedHttpErrorOutput {
    /// The HTTP-error handler completed immediately.
    Variant1(()),
    /// The HTTP-error handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned after a TLS certificate error is reported.
pub enum WebViewPropsOnReceivedSslErrorOutput {
    /// The TLS-error handler completed immediately.
    Variant1(()),
    /// The TLS-error handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned when WebView content starts a download.
pub enum WebViewPropsOnDownloadStartOutput {
    /// The download handler completed immediately.
    Variant1(()),
    /// The download handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned after a page console message is delivered.
pub enum WebViewPropsOnConsoleMessageOutput {
    /// The console handler completed immediately.
    Variant1(()),
    /// The console handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned when the WebView URL changes.
pub enum WebViewPropsOnUrlChangedOutput {
    /// The navigation handler completed immediately.
    Variant1(()),
    /// The navigation handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned when page loading progress changes.
pub enum WebViewPropsOnProgressChangedOutput {
    /// The progress handler completed immediately.
    Variant1(()),
    /// The progress handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned after a WebView state snapshot changes.
pub enum WebViewPropsOnStateChangedOutput {
    /// The state handler completed immediately.
    Variant1(()),
    /// The state handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Completion mode returned after a WebView lifecycle transition.
pub enum WebViewPropsOnLifecycleEventOutput {
    /// The lifecycle handler completed immediately.
    Variant1(()),
    /// The lifecycle handler continues asynchronously.
    Variant2(JsFuture<()>),
}
/// Navigation decision returned directly or after asynchronous evaluation.
pub enum WebViewPropsOnShouldOverrideUrlLoadingOutput {
    /// Applies a navigation decision immediately.
    Variant1(ComposeWebViewNavigationDecision),
    /// Resolves an optional navigation decision asynchronously.
    Variant2(JsFuture<JsOptional<ComposeWebViewNavigationDecision>>),
}
/// Resource interception decision returned directly or asynchronously.
pub enum WebViewPropsOnInterceptRequestOutput {
    /// Applies a resource decision immediately.
    Variant1(ComposeWebViewResourceDecision),
    /// Resolves an optional resource decision asynchronously.
    Variant2(JsFuture<JsOptional<ComposeWebViewResourceDecision>>),
}
/// Scalar value accepted when interpolating a Compose template.
pub enum ComposeTemplateValuesAdditionalValue {
    /// Inserts text into the template.
    Variant1(String),
    /// Inserts a numeric value into the template.
    Variant2(f64),
    /// Inserts a boolean value into the template.
    Variant3(bool),
}
/// Complete node-factory surface exposed through `ComposeDslContext::UI`.
pub struct ComposeDslContextUIIntersection {
    /// Factories for the core Compose DSL components.
    pub member_1: ComposeUiFactoryRegistry,
    /// Factories generated for Material 3 components.
    pub member_2: ComposeMaterial3GeneratedUiFactoryRegistry,
    /// Additional host-registered component factories keyed by element name.
    pub member_3: BTreeMap<String, ComposeNodeFactory<BTreeMap<String, serde_json::Value>>>,
}
/// Stable mutable reference retained across renders for a keyed screen instance.
pub struct ComposeDslContextMethodsUseRefReturn<T> {
    /// Current value stored in the persistent reference.
    pub current: T,
}
/// Completion mode for updating one runtime environment entry.
pub enum ComposeDslContextMethodsSetEnvReturn {
    /// The host persists the value asynchronously.
    Variant1(JsFuture<()>),
    /// The value was persisted before the call returned.
    Variant2(()),
}
/// Completion mode for requesting route navigation.
pub enum ComposeDslContextMethodsNavigateReturn {
    /// Navigation completes asynchronously.
    Variant1(JsFuture<()>),
    /// Navigation was accepted immediately.
    Variant2(()),
}
/// Completion mode for displaying a host toast.
pub enum ComposeDslContextMethodsShowToastReturn {
    /// Toast presentation is scheduled asynchronously.
    Variant1(JsFuture<()>),
    /// Toast presentation was scheduled immediately.
    Variant2(()),
}
/// Completion mode for forwarding an error to the host.
pub enum ComposeDslContextMethodsReportErrorReturn {
    /// Error reporting completes asynchronously.
    Variant1(JsFuture<()>),
    /// The host accepted the error immediately.
    Variant2(()),
}
/// Completion mode for a batch of runtime environment updates.
pub enum ComposeDslContextMethodsSetEnvsReturn {
    /// The host persists the batch asynchronously.
    Variant1(JsFuture<()>),
    /// The batch was persisted before the call returned.
    Variant2(()),
}
/// Availability result for an imported package, returned immediately or asynchronously.
pub enum ComposeDslContextMethodsIsPackageImportedReturn {
    /// Resolves package availability asynchronously.
    Variant1(JsFuture<bool>),
    /// Reports package availability immediately.
    Variant2(bool),
}
/// Package identifier produced when an import operation completes.
pub enum ComposeDslContextMethodsImportPackageReturn {
    /// Resolves the imported package identifier asynchronously.
    Variant1(JsFuture<String>),
    /// Returns the imported package identifier immediately.
    Variant2(String),
}
/// Package identifier produced when an imported package is removed.
pub enum ComposeDslContextMethodsRemovePackageReturn {
    /// Resolves the removed package identifier asynchronously.
    Variant1(JsFuture<String>),
    /// Returns the removed package identifier immediately.
    Variant2(String),
}
/// Package identifier produced when selecting an imported package for use.
pub enum ComposeDslContextMethodsUsePackageReturn {
    /// Resolves the selected package identifier asynchronously.
    Variant1(JsFuture<String>),
    /// Returns the selected package identifier immediately.
    Variant2(String),
}
/// Imported package names returned immediately or asynchronously.
pub enum ComposeDslContextMethodsListImportedPackagesReturn {
    /// Resolves the package list asynchronously.
    Variant1(JsFuture<Vec<String>>),
    /// Returns the package list immediately.
    Variant2(Vec<String>),
}
/// Resolved runtime tool name returned immediately or asynchronously.
pub enum ComposeDslContextMethodsResolveToolNameReturn {
    /// Resolves the runtime name asynchronously.
    Variant1(JsFuture<String>),
    /// Returns the runtime name immediately.
    Variant2(String),
}
/// Root node produced by a Compose screen renderer.
pub enum ComposeDslScreenOutput {
    /// Supplies the completed node tree immediately.
    Variant1(ComposeNode),
    /// Builds the node tree asynchronously.
    Variant2(JsFuture<ComposeNode>),
}
/// Material typography role used to select themed text metrics.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeTextStyle {
    /// Small headline typography.
    #[serde(rename = "headlineSmall")]
    HeadlineSmall,
    /// Medium headline typography.
    #[serde(rename = "headlineMedium")]
    HeadlineMedium,
    /// Large title typography.
    #[serde(rename = "titleLarge")]
    TitleLarge,
    /// Medium title typography.
    #[serde(rename = "titleMedium")]
    TitleMedium,
    /// Small title typography.
    #[serde(rename = "titleSmall")]
    TitleSmall,
    /// Large body-copy typography.
    #[serde(rename = "bodyLarge")]
    BodyLarge,
    /// Medium body-copy typography.
    #[serde(rename = "bodyMedium")]
    BodyMedium,
    /// Small body-copy typography.
    #[serde(rename = "bodySmall")]
    BodySmall,
    /// Large label typography for prominent controls.
    #[serde(rename = "labelLarge")]
    LabelLarge,
    /// Medium label typography for controls and annotations.
    #[serde(rename = "labelMedium")]
    LabelMedium,
    /// Small label typography for compact annotations.
    #[serde(rename = "labelSmall")]
    LabelSmall,
}
/// Distribution strategy for children along a layout's main axis.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeArrangement {
    /// Packs children against the main-axis start.
    #[serde(rename = "start")]
    Start,
    /// Packs children around the main-axis center.
    #[serde(rename = "center")]
    Center,
    /// Packs children against the main-axis end.
    #[serde(rename = "end")]
    End,
    /// Places equal gaps only between adjacent children.
    #[serde(rename = "spaceBetween")]
    SpaceBetween,
    /// Places equal space around every child, including outer edges.
    #[serde(rename = "spaceAround")]
    SpaceAround,
    /// Places equal gaps between children and both outer edges.
    #[serde(rename = "spaceEvenly")]
    SpaceEvenly,
}
/// Cross-axis placement for children inside a layout container.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeAlignment {
    /// Aligns children with the cross-axis start edge.
    #[serde(rename = "start")]
    Start,
    /// Centers children on the cross axis.
    #[serde(rename = "center")]
    Center,
    /// Aligns children with the cross-axis end edge.
    #[serde(rename = "end")]
    End,
}
/// Geometry used to clip or outline a Compose component.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeShapeType {
    /// Uses smoothly rounded corners.
    #[serde(rename = "rounded")]
    Rounded,
    /// Uses diagonally cut corners.
    #[serde(rename = "cut")]
    Cut,
    /// Clips content to a circle.
    #[serde(rename = "circle")]
    Circle,
    /// Uses fully rounded ends based on component height.
    #[serde(rename = "pill")]
    Pill,
}
/// Shape geometry with optional uniform, logical, or physical corner radii.
pub struct ComposeShape {
    /// Geometry family used to interpret the radius fields.
    pub r#type: Option<ComposeShapeType>,
    /// Uniform radius applied to every corner.
    pub cornerRadius: Option<f64>,
    /// Radius used by circular or pill geometry.
    pub radius: Option<f64>,
    /// Radius at the top corner on the layout-direction start side.
    pub topStart: Option<f64>,
    /// Radius at the top corner on the layout-direction end side.
    pub topEnd: Option<f64>,
    /// Radius at the bottom corner on the layout-direction start side.
    pub bottomStart: Option<f64>,
    /// Radius at the bottom corner on the layout-direction end side.
    pub bottomEnd: Option<f64>,
    /// Radius at the physical top-left corner.
    pub topLeft: Option<f64>,
    /// Radius at the physical top-right corner.
    pub topRight: Option<f64>,
    /// Radius at the physical bottom-left corner.
    pub bottomLeft: Option<f64>,
    /// Radius at the physical bottom-right corner.
    pub bottomRight: Option<f64>,
}
/// Stroke rendered around a component boundary.
pub struct ComposeBorder {
    /// Thickness of the border stroke.
    pub width: Option<f64>,
    /// Solid or themed color used for the stroke.
    pub color: Option<ComposeColor>,
    /// Opacity multiplier applied to the border color.
    pub alpha: Option<f64>,
}
/// Insets applied symmetrically on the horizontal and vertical axes.
pub struct ComposePadding {
    /// Inset applied to the start and end edges.
    pub horizontal: Option<f64>,
    /// Inset applied to the top and bottom edges.
    pub vertical: Option<f64>,
}
/// Coordinate unit accepted by canvas drawing commands.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeCanvasUnit {
    /// Physical display pixels.
    #[serde(rename = "px")]
    Px,
    /// Density-independent pixels.
    #[serde(rename = "dp")]
    Dp,
    /// Fraction of the relevant canvas dimension.
    #[serde(rename = "fraction")]
    Fraction,
}
/// Numeric canvas quantity paired with an explicit coordinate unit.
pub struct ComposeUnitValue {
    /// Scalar magnitude before unit conversion.
    pub value: f64,
    /// Coordinate space in which the magnitude is expressed.
    pub unit: ComposeCanvasUnit,
}
/// Canvas quantity expressed either in the command's default unit or explicitly.
pub enum ComposeCanvasNumber {
    /// Uses the surrounding command's unit setting.
    Variant1(f64),
    /// Carries its own pixel, density-independent, or fractional unit.
    Variant2(ComposeUnitValue),
}
/// Rendering behavior when laid-out text exceeds its bounds.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeTextOverflow {
    /// Cuts off glyphs outside the layout bounds.
    #[serde(rename = "clip")]
    Clip,
    /// Replaces the truncated tail with an ellipsis.
    #[serde(rename = "ellipsis")]
    Ellipsis,
}
/// Scaling strategy for fitting visual content into destination bounds.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeContentScale {
    /// Preserves aspect ratio while fitting entirely inside the bounds.
    #[serde(rename = "fit")]
    Fit,
    /// Preserves aspect ratio while covering the bounds and cropping overflow.
    #[serde(rename = "crop")]
    Crop,
    /// Stretches independently on both axes to fill the bounds.
    #[serde(rename = "fillBounds")]
    FillBounds,
    /// Scales to fill the width while preserving aspect ratio.
    #[serde(rename = "fillWidth")]
    FillWidth,
    /// Scales to fill the height while preserving aspect ratio.
    #[serde(rename = "fillHeight")]
    FillHeight,
    /// Shrinks oversized content to fit without enlarging smaller content.
    #[serde(rename = "inside")]
    Inside,
    /// Draws content at its intrinsic size.
    #[serde(rename = "none")]
    None,
}
/// Constraints used to measure text before constructing a canvas layout.
pub struct ComposeTextMeasureRequest {
    /// Text whose rendered bounds are requested.
    pub text: String,
    /// Font size used for measurement.
    pub fontSize: Option<f64>,
    /// Maximum line width available to the text layout.
    pub maxWidth: f64,
    /// Maximum total height available to the text layout.
    pub maxHeight: Option<f64>,
    /// Minimum width reserved for the measured layout.
    pub minWidth: Option<f64>,
    /// Minimum height reserved for the measured layout.
    pub minHeight: Option<f64>,
    /// Maximum number of laid-out lines.
    pub maxLines: Option<f64>,
    /// Clipping behavior when the text exceeds its constraints.
    pub overflow: Option<ComposeTextOverflow>,
}
/// Final bounds of a text layout after applying measurement constraints.
pub struct ComposeTextMeasureResult {
    /// Measured layout width.
    pub width: f64,
    /// Measured layout height.
    pub height: f64,
}
/// Reference to a named theme color with an optional opacity adjustment.
pub struct ComposeColorToken {
    /// Host-resolved token name in the active Material color scheme.
    pub __colorToken: String,
    /// Opacity multiplier applied when resolving the token.
    pub alpha: Option<f64>,
}
/// Operations available on a theme color token.
pub trait ComposeColorTokenMethods: Send + Sync {
    /// Derives the same theme color with a different opacity multiplier.
    fn copy(&self, options: ComposeColorTokenMethodsCopyOptions) -> ComposeColorToken;
}
/// Color supplied either as a concrete CSS-style value or a theme token.
pub enum ComposeColor {
    /// Uses a concrete color string understood by the host renderer.
    Variant1(String),
    /// Resolves a semantic color from the active Material theme.
    Variant2(ComposeColorToken),
}
/// Named semantic colors available from the active Material theme.
pub struct ComposeColorScheme {
    /// Maps Material color-role names to host-resolved tokens.
    pub additional_properties: BTreeMap<String, ComposeColorToken>,
}
/// Material theme values exposed while rendering a Compose DSL screen.
pub struct ComposeMaterialTheme {
    /// Semantic color roles for the active light or dark theme.
    pub colorScheme: ComposeColorScheme,
}
/// Multi-color paint used to fill canvas geometry.
pub struct ComposeCanvasBrush {
    /// Gradient direction used to interpolate the color stops.
    pub r#type: ComposeCanvasBrushType,
    /// Ordered color stops from the start edge to the end edge.
    pub colors: Vec<ComposeColor>,
}
/// Horizontal placement used by width and box-layout modifiers.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeHorizontalAlignment {
    /// Aligns to the logical start edge.
    #[serde(rename = "start")]
    Start,
    /// Centers on the horizontal axis.
    #[serde(rename = "center")]
    Center,
    /// Aligns to the logical end edge.
    #[serde(rename = "end")]
    End,
    /// Aligns to the physical left edge.
    #[serde(rename = "left")]
    Left,
    /// Aligns to the physical right edge.
    #[serde(rename = "right")]
    Right,
    /// Uses Compose's explicit horizontal-center alignment.
    #[serde(rename = "centerHorizontally")]
    CenterHorizontally,
}
/// Vertical placement used by height and box-layout modifiers.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeVerticalAlignment {
    /// Aligns to the top edge.
    #[serde(rename = "top")]
    Top,
    /// Centers on the vertical axis.
    #[serde(rename = "center")]
    Center,
    /// Aligns to the bottom edge.
    #[serde(rename = "bottom")]
    Bottom,
    /// Aligns to the logical start of the vertical axis.
    #[serde(rename = "start")]
    Start,
    /// Aligns to the logical end of the vertical axis.
    #[serde(rename = "end")]
    End,
    /// Uses Compose's explicit vertical-center alignment.
    #[serde(rename = "centerVertically")]
    CenterVertically,
}
/// Two-dimensional placement of content inside a box.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeBoxAlignment {
    /// Centers content on both axes.
    #[serde(rename = "center")]
    Center,
    /// Aligns content to the top-start corner.
    #[serde(rename = "topStart")]
    TopStart,
    /// Alias for top-start alignment.
    #[serde(rename = "startTop")]
    StartTop,
    /// Centers content along the top edge.
    #[serde(rename = "topCenter")]
    TopCenter,
    /// Alias for top-center alignment.
    #[serde(rename = "centerTop")]
    CenterTop,
    /// Aligns content to the top-end corner.
    #[serde(rename = "topEnd")]
    TopEnd,
    /// Alias for top-end alignment.
    #[serde(rename = "endTop")]
    EndTop,
    /// Centers content along the start edge.
    #[serde(rename = "centerStart")]
    CenterStart,
    /// Alias for center-start alignment.
    #[serde(rename = "startCenter")]
    StartCenter,
    /// Centers content along the end edge.
    #[serde(rename = "centerEnd")]
    CenterEnd,
    /// Alias for center-end alignment.
    #[serde(rename = "endCenter")]
    EndCenter,
    /// Aligns content to the bottom-start corner.
    #[serde(rename = "bottomStart")]
    BottomStart,
    /// Alias for bottom-start alignment.
    #[serde(rename = "startBottom")]
    StartBottom,
    /// Centers content along the bottom edge.
    #[serde(rename = "bottomCenter")]
    BottomCenter,
    /// Alias for bottom-center alignment.
    #[serde(rename = "centerBottom")]
    CenterBottom,
    /// Aligns content to the bottom-end corner.
    #[serde(rename = "bottomEnd")]
    BottomEnd,
    /// Alias for bottom-end alignment.
    #[serde(rename = "endBottom")]
    EndBottom,
}
/// Alignment accepted by a modifier in horizontal, vertical, or box scope.
pub enum ComposeModifierAlign {
    /// Uses a horizontal-axis alignment.
    Variant1(ComposeHorizontalAlignment),
    /// Uses a vertical-axis alignment.
    Variant2(ComposeVerticalAlignment),
    /// Uses a two-dimensional box alignment.
    Variant3(ComposeBoxAlignment),
}
/// Edge insets added by a padding modifier.
pub struct ComposeModifierPadding {
    /// Uniform inset applied to every edge.
    pub all: Option<f64>,
    /// Inset applied to the logical start and end edges.
    pub horizontal: Option<f64>,
    /// Inset applied to the top and bottom edges.
    pub vertical: Option<f64>,
    /// Inset on the layout-direction start edge.
    pub start: Option<f64>,
    /// Inset on the top edge.
    pub top: Option<f64>,
    /// Inset on the layout-direction end edge.
    pub end: Option<f64>,
    /// Inset on the bottom edge.
    pub bottom: Option<f64>,
}
/// Translation applied to a node after layout measurement.
pub struct ComposeModifierOffset {
    /// Horizontal displacement from the laid-out position.
    pub x: Option<f64>,
    /// Vertical displacement from the laid-out position.
    pub y: Option<f64>,
}
/// Minimum and maximum extent for one layout axis.
pub struct ComposeModifierAxisBounds {
    /// Smallest permitted extent on the axis.
    pub min: Option<f64>,
    /// Largest permitted extent on the axis.
    pub max: Option<f64>,
}
/// Horizontal measurement constraints accepted by `widthIn`.
pub struct ComposeModifierWidthBounds {
    /// Generic axis bounds accepted as shorthand width limits.
    pub base_compose_modifier_axis_bounds: ComposeModifierAxisBounds,
    /// Explicit minimum measured width.
    pub minWidth: Option<f64>,
    /// Explicit maximum measured width.
    pub maxWidth: Option<f64>,
}
/// Vertical measurement constraints accepted by `heightIn`.
pub struct ComposeModifierHeightBounds {
    /// Generic axis bounds accepted as shorthand height limits.
    pub base_compose_modifier_axis_bounds: ComposeModifierAxisBounds,
    /// Explicit minimum measured height.
    pub minHeight: Option<f64>,
    /// Explicit maximum measured height.
    pub maxHeight: Option<f64>,
}
/// Independent width and height constraints accepted by `sizeIn`.
pub struct ComposeModifierSizeBounds {
    /// Shorthand minimum applied to both dimensions.
    pub min: Option<f64>,
    /// Shorthand maximum applied to both dimensions.
    pub max: Option<f64>,
    /// Minimum measured width.
    pub minWidth: Option<f64>,
    /// Minimum measured height.
    pub minHeight: Option<f64>,
    /// Maximum measured width.
    pub maxWidth: Option<f64>,
    /// Maximum measured height.
    pub maxHeight: Option<f64>,
}
/// Default minimum dimensions used only when incoming constraints permit them.
pub struct ComposeModifierDefaultMinSize {
    /// Shorthand minimum applied to both width and height.
    pub all: Option<f64>,
    /// Preferred minimum width.
    pub minWidth: Option<f64>,
    /// Preferred minimum height.
    pub minHeight: Option<f64>,
}
/// Horizontal placement when wrapping a node to its measured width.
pub struct ComposeModifierWrapContentWidthOptions {
    /// Position of wrapped content within the available width.
    pub align: Option<ComposeHorizontalAlignment>,
    /// Whether measurement may ignore the incoming maximum width.
    pub unbounded: Option<bool>,
}
/// Vertical placement when wrapping a node to its measured height.
pub struct ComposeModifierWrapContentHeightOptions {
    /// Position of wrapped content within the available height.
    pub align: Option<ComposeVerticalAlignment>,
    /// Whether measurement may ignore the incoming maximum height.
    pub unbounded: Option<bool>,
}
/// Two-dimensional placement when wrapping a node to its measured size.
pub struct ComposeModifierWrapContentSizeOptions {
    /// Position of wrapped content within the available bounds.
    pub align: Option<ComposeBoxAlignment>,
    /// Whether measurement may ignore incoming maximum dimensions.
    pub unbounded: Option<bool>,
}
/// Shadow elevation, outline, and clipping applied by a modifier.
pub struct ComposeModifierShadowOptions {
    /// Elevation used to calculate blur and offset.
    pub elevation: f64,
    /// Outline from which the shadow is cast.
    pub shape: Option<ComposeShape>,
    /// Whether child drawing is clipped to the shadow outline.
    pub clip: Option<bool>,
}
/// Primary, long-press, and double-click callbacks installed as one gesture detector.
pub struct ComposeModifierCombinedClickableOptions {
    /// Handles a recognized primary click.
    pub onClick:
        Arc<dyn Fn() -> ComposeModifierCombinedClickableOptionsOnClickOutput + Send + Sync>,
    /// Handles a recognized long press when supplied.
    pub onLongClick: Option<
        Arc<dyn Fn() -> ComposeModifierCombinedClickableOptionsOnLongClickOutput + Send + Sync>,
    >,
    /// Handles a recognized double click when supplied.
    pub onDoubleClick: Option<
        Arc<dyn Fn() -> ComposeModifierCombinedClickableOptionsOnDoubleClickOutput + Send + Sync>,
    >,
}
/// Pointer location reported in the target node's local coordinate space.
pub struct ComposePointerOffsetEvent {
    /// Horizontal pointer coordinate.
    pub x: f64,
    /// Vertical pointer coordinate.
    pub y: f64,
}
/// Pointer position and movement delta for one drag update.
pub struct ComposeDragGestureEvent {
    /// Current pointer location in local coordinates.
    pub base_compose_pointer_offset_event: ComposePointerOffsetEvent,
    /// Horizontal movement since the preceding drag event.
    pub deltaX: f64,
    /// Vertical movement since the preceding drag event.
    pub deltaY: f64,
}
/// Measured dimensions reported after a node's layout size changes.
pub struct ComposeSizeChangedEvent {
    /// New measured width.
    pub width: f64,
    /// New measured height.
    pub height: f64,
}
/// Node bounds expressed in both root and host-window coordinate spaces.
pub struct ComposeGloballyPositionedEvent {
    /// Horizontal offset from the Compose root.
    pub rootX: f64,
    /// Vertical offset from the Compose root.
    pub rootY: f64,
    /// Measured node width.
    pub width: f64,
    /// Measured node height.
    pub height: f64,
    /// Horizontal offset from the host window.
    pub windowX: f64,
    /// Vertical offset from the host window.
    pub windowY: f64,
}
/// Callbacks for distinct press and tap gestures on a node.
pub struct ComposeModifierTapGesturesOptions {
    /// Runs as soon as a pointer press is recognized.
    pub onPress: Option<
        Arc<
            dyn Fn(ComposePointerOffsetEvent) -> ComposeModifierTapGesturesOptionsOnPressOutput
                + Send
                + Sync,
        >,
    >,
    /// Runs after a single tap is recognized.
    pub onTap: Option<
        Arc<
            dyn Fn(ComposePointerOffsetEvent) -> ComposeModifierTapGesturesOptionsOnTapOutput
                + Send
                + Sync,
        >,
    >,
    /// Runs after two taps are recognized within the gesture interval.
    pub onDoubleTap: Option<
        Arc<
            dyn Fn(ComposePointerOffsetEvent) -> ComposeModifierTapGesturesOptionsOnDoubleTapOutput
                + Send
                + Sync,
        >,
    >,
    /// Runs when a press exceeds the long-press threshold.
    pub onLongPress: Option<
        Arc<
            dyn Fn(ComposePointerOffsetEvent) -> ComposeModifierTapGesturesOptionsOnLongPressOutput
                + Send
                + Sync,
        >,
    >,
}
/// Callbacks describing the start, updates, and termination of a drag gesture.
pub struct ComposeModifierDragGesturesOptions {
    /// Receives the pointer location where dragging begins.
    pub onDragStart: Option<
        Arc<
            dyn Fn(ComposePointerOffsetEvent) -> ComposeModifierDragGesturesOptionsOnDragStartOutput
                + Send
                + Sync,
        >,
    >,
    /// Receives each pointer position and incremental movement during dragging.
    pub onDrag: Option<
        Arc<
            dyn Fn(ComposeDragGestureEvent) -> ComposeModifierDragGesturesOptionsOnDragOutput
                + Send
                + Sync,
        >,
    >,
    /// Runs when the pointer is released after a successful drag.
    pub onDragEnd:
        Option<Arc<dyn Fn() -> ComposeModifierDragGesturesOptionsOnDragEndOutput + Send + Sync>>,
    /// Runs when another gesture or lifecycle event cancels the drag.
    pub onDragCancel:
        Option<Arc<dyn Fn() -> ComposeModifierDragGesturesOptionsOnDragCancelOutput + Send + Sync>>,
}
/// Configuration for combined pan, pinch-zoom, and rotation gestures.
pub struct ComposeModifierTransformGesturesOptions {
    /// Locks gesture recognition to pan and zoom after those motions win the slop race.
    pub panZoomLock: Option<bool>,
    /// Receives the centroid and incremental pan, zoom, and rotation deltas.
    pub onGesture: Arc<
        dyn Fn(
                ComposeCanvasTransformEvent,
            ) -> ComposeModifierTransformGesturesOptionsOnGestureOutput
            + Send
            + Sync,
    >,
}
/// Persistent scale and translation applied while drawing a canvas.
pub struct ComposeCanvasTransform {
    /// Uniform scale factor applied to canvas content.
    pub scale: Option<f64>,
    /// Horizontal translation after scaling.
    pub offsetX: Option<f64>,
    /// Vertical translation after scaling.
    pub offsetY: Option<f64>,
    /// Horizontal coordinate around which scaling is performed.
    pub pivotX: Option<f64>,
    /// Vertical coordinate around which scaling is performed.
    pub pivotY: Option<f64>,
}
/// Incremental multi-touch transform reported by a canvas gesture detector.
pub struct ComposeCanvasTransformEvent {
    /// Horizontal center of the active pointers.
    pub centroidX: f64,
    /// Vertical center of the active pointers.
    pub centroidY: f64,
    /// Horizontal translation since the preceding event.
    pub panX: f64,
    /// Vertical translation since the preceding event.
    pub panY: f64,
    /// Multiplicative scale delta since the preceding event.
    pub zoom: f64,
    /// Rotation delta around the pointer centroid.
    pub rotation: f64,
}
/// Canvas dimensions reported after layout measurement changes.
pub struct ComposeCanvasSizeEvent {
    /// New measured width and height of the drawing surface.
    pub base_compose_size_changed_event: ComposeSizeChangedEvent,
}
/// Page metadata captured at the start or completion of WebView navigation.
pub struct ComposeWebViewPageEvent {
    /// Page URL known at the time of the callback.
    pub url: JsOptional<String>,
    /// Current document title, when available.
    pub title: JsOptional<String>,
    /// Whether backward history navigation is currently possible.
    pub canGoBack: Option<bool>,
    /// Whether forward history navigation is currently possible.
    pub canGoForward: Option<bool>,
}
/// Details emitted when the WebView's active URL changes.
pub struct ComposeWebViewNavigationEvent {
    /// Newly active or requested URL.
    pub url: JsOptional<String>,
    /// Whether the navigation targets the top-level document.
    pub isMainFrame: Option<bool>,
    /// HTTP method used by the navigation request.
    pub method: JsOptional<String>,
}
/// Snapshot emitted as the current page advances through loading.
pub struct ComposeWebViewProgressEvent {
    /// Host-reported loading completion value.
    pub progress: f64,
    /// URL associated with the loading page.
    pub url: JsOptional<String>,
    /// Current document title, when available.
    pub title: JsOptional<String>,
}
/// Network or content-loading failure reported by a WebView.
pub struct ComposeWebViewErrorEvent {
    /// Platform WebView error code.
    pub errorCode: f64,
    /// Human-readable platform error description.
    pub description: JsOptional<String>,
    /// Resource URL that failed to load.
    pub url: JsOptional<String>,
}
/// Non-success HTTP response observed while loading WebView content.
pub struct ComposeWebViewHttpErrorEvent {
    /// HTTP response status code.
    pub statusCode: JsOptional<f64>,
    /// HTTP reason phrase supplied by the server.
    pub reasonPhrase: JsOptional<String>,
    /// URL whose response contained the error status.
    pub url: JsOptional<String>,
    /// Whether the failed response belongs to the top-level document.
    pub isMainFrame: Option<bool>,
}
/// TLS certificate validation failure observed by a WebView.
pub struct ComposeWebViewSslErrorEvent {
    /// Platform code for the primary certificate failure.
    pub primaryError: JsOptional<f64>,
    /// HTTPS URL whose certificate failed validation.
    pub url: JsOptional<String>,
}
/// Download request initiated by content inside a WebView.
pub struct ComposeWebViewDownloadEvent {
    /// URL of the file requested for download.
    pub url: String,
    /// User-Agent associated with the request.
    pub userAgent: JsOptional<String>,
    /// Server content-disposition header used to infer a filename.
    pub contentDisposition: JsOptional<String>,
    /// Reported media type of the download.
    pub mimeType: JsOptional<String>,
    /// Reported download size in bytes.
    pub contentLength: JsOptional<f64>,
    /// Filename derived by the host from URL and response metadata.
    pub suggestedFileName: JsOptional<String>,
}
/// Console entry emitted by JavaScript running in WebView content.
pub struct ComposeWebViewConsoleEvent {
    /// Text written to the page console.
    pub message: JsOptional<String>,
    /// Script URL or source identifier that emitted the entry.
    pub sourceId: JsOptional<String>,
    /// Source line associated with the entry.
    pub lineNumber: JsOptional<f64>,
    /// Console severity such as debug, warning, or error.
    pub level: JsOptional<String>,
}
/// Current navigation and loading state of a controlled WebView.
pub struct ComposeWebViewState {
    /// URL currently displayed or being loaded.
    pub url: JsOptional<String>,
    /// Current document title.
    pub title: JsOptional<String>,
    /// Whether the WebView is actively loading content.
    pub loading: bool,
    /// Host-reported loading completion value.
    pub progress: f64,
    /// Whether backward navigation is available in history.
    pub canGoBack: bool,
    /// Whether forward navigation is available in history.
    pub canGoForward: bool,
}
/// WebView lifecycle transition accompanied by the latest page state.
pub struct ComposeWebViewLifecycleEvent {
    /// Native lifecycle transition that triggered the event.
    pub r#type: ComposeWebViewLifecycleEventType,
    /// Active URL when the transition occurred.
    pub url: JsOptional<String>,
    /// Active document title when available.
    pub title: JsOptional<String>,
    /// Whether loading was active during the transition.
    pub loading: Option<bool>,
    /// Loading completion value captured with the transition.
    pub progress: Option<f64>,
    /// Whether backward history navigation was available.
    pub canGoBack: Option<bool>,
    /// Whether forward history navigation was available.
    pub canGoForward: Option<bool>,
    /// Whether renderer termination was caused by a crash.
    pub didCrash: Option<bool>,
    /// Platform renderer priority recorded when the process exited.
    pub rendererPriorityAtExit: JsOptional<f64>,
}
/// Top-level or subframe navigation request offered to plugin interception.
pub struct ComposeWebViewNavigationRequest {
    /// Destination requested by the page or user.
    pub url: String,
    /// HTTP method used for the navigation.
    pub method: JsOptional<String>,
    /// Request headers visible to the WebView host.
    pub headers: Option<BTreeMap<String, String>>,
    /// Whether the request targets the top-level document.
    pub isMainFrame: Option<bool>,
    /// Whether a user gesture initiated the request.
    pub hasGesture: Option<bool>,
    /// Whether the request follows an HTTP or script redirect.
    pub isRedirect: Option<bool>,
    /// Parsed URL scheme used for protocol-specific decisions.
    pub scheme: JsOptional<String>,
}
/// Action selected by a plugin after inspecting a navigation request.
pub enum ComposeWebViewNavigationDecision {
    /// Continues the original navigation.
    Variant1(ComposeWebViewNavigationDecisionVariant1),
    /// Cancels the original navigation.
    Variant2(ComposeWebViewNavigationDecisionVariant2),
    /// Replaces the original navigation with another request.
    Variant3(ComposeWebViewNavigationDecisionVariant3),
    /// Delegates the target to an external application.
    Variant4(ComposeWebViewNavigationDecisionVariant4),
}
/// Document or subresource request offered to WebView resource interception.
pub struct ComposeWebViewResourceRequest {
    /// URL of the requested resource.
    pub url: String,
    /// HTTP method used for the request.
    pub method: JsOptional<String>,
    /// Request headers visible to the WebView host.
    pub headers: Option<BTreeMap<String, String>>,
    /// Whether the resource is the top-level document.
    pub isMainFrame: Option<bool>,
    /// Whether a user gesture initiated the request.
    pub hasGesture: Option<bool>,
    /// Whether the request follows a redirect.
    pub isRedirect: Option<bool>,
    /// Parsed URL scheme used for protocol-specific handling.
    pub scheme: JsOptional<String>,
}
/// Body source used when a plugin supplies a synthetic WebView response.
pub enum ComposeWebViewResourceResponse {
    /// Serves decoded text.
    Variant1(ComposeWebViewResourceResponseVariant1),
    /// Serves base64-encoded binary content.
    Variant2(ComposeWebViewResourceResponseVariant2),
    /// Serves the contents of a local file.
    Variant3(ComposeWebViewResourceResponseVariant3),
}
/// Action selected by a plugin after inspecting a WebView resource request.
pub enum ComposeWebViewResourceDecision {
    /// Lets the WebView perform the original request.
    Variant1(ComposeWebViewResourceDecisionVariant1),
    /// Blocks the request without loading content.
    Variant2(ComposeWebViewResourceDecisionVariant2),
    /// Replaces the request URL and optional headers.
    Variant3(ComposeWebViewResourceDecisionVariant3),
    /// Responds with plugin-provided content.
    Variant4(ComposeWebViewResourceDecisionVariant4),
}
/// Callable method exposed by the host object to JavaScript inside a WebView.
pub type ComposeWebViewJavascriptInterfaceMethod = Arc<
    dyn Fn(Vec<serde_json::Value>) -> ComposeWebViewJavascriptInterfaceMethodOutput + Send + Sync,
>;
/// Named methods installed together as one WebView JavaScript interface object.
pub type ComposeWebViewJavascriptInterface =
    BTreeMap<String, ComposeWebViewJavascriptInterfaceMethod>;
/// Origin and decoding metadata used when loading an inline HTML document.
pub struct ComposeWebViewLoadHtmlOptions {
    /// Base URL used to resolve relative links and establish document origin.
    pub baseUrl: Option<String>,
    /// Media type assigned to the inline document.
    pub mimeType: Option<String>,
    /// Character encoding used to decode the HTML string.
    pub encoding: Option<String>,
}
/// Stable handle used to issue imperative commands to a rendered WebView.
pub struct ComposeWebViewController {
    /// Identity linking the controller to its WebView node across renders.
    pub key: String,
}
/// Imperative navigation, script, and bridge operations for a controlled WebView.
pub trait ComposeWebViewControllerMethods: Send + Sync {
    /// Loads a URL with optional request headers in the controlled WebView.
    fn loadUrl(&self, url: String, headers: Option<BTreeMap<String, String>>) -> ();
    /// Loads an inline HTML document using optional origin and encoding metadata.
    fn loadHtml(&self, html: String, options: Option<ComposeWebViewLoadHtmlOptions>) -> ();
    /// Reloads the currently active document.
    fn reload(&self) -> ();
    /// Cancels the current page load.
    fn stopLoading(&self) -> ();
    /// Navigates to the preceding WebView history entry.
    fn goBack(&self) -> ();
    /// Navigates to the following WebView history entry.
    fn goForward(&self) -> ();
    /// Removes stored back and forward history entries.
    fn clearHistory(&self) -> ();
    /// Evaluates a script in the active page and resolves its decoded result.
    fn evaluateJavascript<TResult>(&self, script: String) -> JsFuture<JsOptional<TResult>>;
    /// Returns the latest navigation and loading state known by the controller.
    fn getState(&self) -> JsOptional<ComposeWebViewState>;
    /// Installs a named host object callable by JavaScript in the page.
    fn addJavascriptInterface(&self, name: String, object: ComposeWebViewJavascriptInterface)
        -> ();
    /// Removes a previously installed JavaScript interface object.
    fn removeJavascriptInterface(&self, name: String) -> ();
}
/// Policy for HTTP subresources requested by an HTTPS page.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeWebViewMixedContentMode {
    /// Allows insecure subresources without restriction.
    #[serde(rename = "alwaysAllow")]
    AlwaysAllow,
    /// Blocks all insecure subresources.
    #[serde(rename = "neverAllow")]
    NeverAllow,
    /// Applies the platform's compatibility policy for legacy pages.
    #[serde(rename = "compatibilityMode")]
    CompatibilityMode,
}
/// Cache policy applied to WebView network requests.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeWebViewCacheMode {
    /// Uses the platform's normal protocol cache behavior.
    #[serde(rename = "default")]
    Default,
    /// Bypasses cached responses and does not store the result.
    #[serde(rename = "noCache")]
    NoCache,
    /// Uses cached content when available and otherwise requests the network.
    #[serde(rename = "cacheElseNetwork")]
    CacheElseNetwork,
    /// Serves only cached content and avoids network access.
    #[serde(rename = "cacheOnly")]
    CacheOnly,
}
/// Command that strokes a straight segment between two canvas points.
pub struct ComposeCanvasLineCommand {
    /// Selects straight-line command decoding.
    pub r#type: ComposeCanvasLineCommandType,
    /// Horizontal coordinate of the segment start.
    pub x1: ComposeCanvasNumber,
    /// Vertical coordinate of the segment start.
    pub y1: ComposeCanvasNumber,
    /// Horizontal coordinate of the segment end.
    pub x2: ComposeCanvasNumber,
    /// Vertical coordinate of the segment end.
    pub y2: ComposeCanvasNumber,
    /// Color used to stroke the segment.
    pub color: Option<ComposeColor>,
    /// Opacity multiplier applied to the stroke color.
    pub alpha: Option<f64>,
    /// Thickness of the stroked segment.
    pub strokeWidth: Option<ComposeCanvasNumber>,
    /// Default unit for scalar coordinates and stroke width.
    pub unit: Option<ComposeCanvasUnit>,
}
/// Command that fills or strokes an axis-aligned canvas rectangle.
pub struct ComposeCanvasRectCommand {
    /// Selects rectangle command decoding.
    pub r#type: ComposeCanvasRectCommandType,
    /// Horizontal coordinate of the rectangle origin.
    pub x: ComposeCanvasNumber,
    /// Vertical coordinate of the rectangle origin.
    pub y: ComposeCanvasNumber,
    /// Rectangle width.
    pub width: ComposeCanvasNumber,
    /// Rectangle height.
    pub height: ComposeCanvasNumber,
    /// Gradient paint used instead of a solid color.
    pub brush: Option<ComposeCanvasBrush>,
    /// Solid paint used when no brush is supplied.
    pub color: Option<ComposeColor>,
    /// Opacity multiplier applied to the selected paint.
    pub alpha: Option<f64>,
    /// Outline thickness when the rectangle is not filled.
    pub strokeWidth: Option<ComposeCanvasNumber>,
    /// Whether to fill the interior instead of drawing only the outline.
    pub filled: Option<bool>,
    /// Default unit for scalar geometry and stroke width.
    pub unit: Option<ComposeCanvasUnit>,
}
/// Command that fills or strokes a rectangle with rounded corners.
pub struct ComposeCanvasRoundRectCommand {
    /// Selects rounded-rectangle command decoding.
    pub r#type: ComposeCanvasRoundRectCommandType,
    /// Horizontal coordinate of the rectangle origin.
    pub x: ComposeCanvasNumber,
    /// Vertical coordinate of the rectangle origin.
    pub y: ComposeCanvasNumber,
    /// Rectangle width.
    pub width: ComposeCanvasNumber,
    /// Rectangle height.
    pub height: ComposeCanvasNumber,
    /// Radius applied to each corner.
    pub radius: Option<ComposeCanvasNumber>,
    /// Gradient paint used instead of a solid color.
    pub brush: Option<ComposeCanvasBrush>,
    /// Solid paint used when no brush is supplied.
    pub color: Option<ComposeColor>,
    /// Opacity multiplier applied to the selected paint.
    pub alpha: Option<f64>,
    /// Outline thickness when the shape is not filled.
    pub strokeWidth: Option<ComposeCanvasNumber>,
    /// Whether to fill the interior instead of drawing only the outline.
    pub filled: Option<bool>,
    /// Default unit for scalar geometry and stroke width.
    pub unit: Option<ComposeCanvasUnit>,
}
/// Command that fills or strokes a circle on the canvas.
pub struct ComposeCanvasCircleCommand {
    /// Selects circle command decoding.
    pub r#type: ComposeCanvasCircleCommandType,
    /// Horizontal coordinate of the circle center.
    pub cx: ComposeCanvasNumber,
    /// Vertical coordinate of the circle center.
    pub cy: ComposeCanvasNumber,
    /// Distance from the center to the circle edge.
    pub radius: ComposeCanvasNumber,
    /// Color used to fill or stroke the circle.
    pub color: Option<ComposeColor>,
    /// Opacity multiplier applied to the circle color.
    pub alpha: Option<f64>,
    /// Outline thickness when the circle is not filled.
    pub strokeWidth: Option<ComposeCanvasNumber>,
    /// Whether to fill the interior instead of drawing only the outline.
    pub filled: Option<bool>,
    /// Default unit for scalar geometry and stroke width.
    pub unit: Option<ComposeCanvasUnit>,
}
/// Command that lays out constrained text and draws it at a canvas position.
pub struct ComposeCanvasTextCommand {
    /// Selects measured-text command decoding.
    pub r#type: ComposeCanvasTextCommandType,
    /// Horizontal coordinate of the text layout origin.
    pub x: ComposeCanvasNumber,
    /// Vertical coordinate of the text layout origin.
    pub y: ComposeCanvasNumber,
    /// Text content to lay out and render.
    pub text: String,
    /// Color used to render glyphs.
    pub color: Option<ComposeColor>,
    /// Opacity multiplier applied to the glyph color.
    pub alpha: Option<f64>,
    /// Font size used by the text layout.
    pub fontSize: Option<ComposeCanvasNumber>,
    /// Minimum width reserved for the text layout.
    pub minWidth: Option<ComposeCanvasNumber>,
    /// Maximum width before lines wrap or overflow.
    pub maxWidth: Option<ComposeCanvasNumber>,
    /// Minimum height reserved for the text layout.
    pub minHeight: Option<ComposeCanvasNumber>,
    /// Maximum height available to the text layout.
    pub maxHeight: Option<ComposeCanvasNumber>,
    /// Maximum number of rendered lines.
    pub maxLines: Option<f64>,
    /// Clipping behavior when text exceeds its layout bounds.
    pub overflow: Option<ComposeTextOverflow>,
    /// Default unit for scalar positions, dimensions, and font size.
    pub unit: Option<ComposeCanvasUnit>,
}
/// Path operation that starts a new contour at a canvas point.
pub struct ComposeCanvasMoveToOp {
    /// Selects move-to operation decoding.
    pub r#type: ComposeCanvasMoveToOpType,
    /// Horizontal coordinate of the new current point.
    pub x: ComposeCanvasNumber,
    /// Vertical coordinate of the new current point.
    pub y: ComposeCanvasNumber,
}
/// Path operation that appends a straight segment to a point.
pub struct ComposeCanvasLineToOp {
    /// Selects line-to operation decoding.
    pub r#type: ComposeCanvasLineToOpType,
    /// Horizontal coordinate of the segment endpoint.
    pub x: ComposeCanvasNumber,
    /// Vertical coordinate of the segment endpoint.
    pub y: ComposeCanvasNumber,
}
/// Path operation that appends a cubic Bezier segment.
pub struct ComposeCanvasCubicToOp {
    /// Selects cubic-curve operation decoding.
    pub r#type: ComposeCanvasCubicToOpType,
    /// Horizontal coordinate of the first control point.
    pub x1: ComposeCanvasNumber,
    /// Vertical coordinate of the first control point.
    pub y1: ComposeCanvasNumber,
    /// Horizontal coordinate of the second control point.
    pub x2: ComposeCanvasNumber,
    /// Vertical coordinate of the second control point.
    pub y2: ComposeCanvasNumber,
    /// Horizontal coordinate of the curve endpoint.
    pub x3: ComposeCanvasNumber,
    /// Vertical coordinate of the curve endpoint.
    pub y3: ComposeCanvasNumber,
}
/// Path operation that appends a quadratic Bezier segment.
pub struct ComposeCanvasQuadToOp {
    /// Selects quadratic-curve operation decoding.
    pub r#type: ComposeCanvasQuadToOpType,
    /// Horizontal coordinate of the control point.
    pub x1: ComposeCanvasNumber,
    /// Vertical coordinate of the control point.
    pub y1: ComposeCanvasNumber,
    /// Horizontal coordinate of the curve endpoint.
    pub x2: ComposeCanvasNumber,
    /// Vertical coordinate of the curve endpoint.
    pub y2: ComposeCanvasNumber,
}
/// Path operation that closes the active contour.
pub struct ComposeCanvasCloseOp {
    /// Selects close-contour operation decoding.
    pub r#type: ComposeCanvasCloseOpType,
}
/// Operation used to construct the geometry of a drawable canvas path.
pub enum ComposeCanvasPathOp {
    /// Starts a new contour without drawing a segment.
    Variant1(ComposeCanvasMoveToOp),
    /// Adds a straight segment.
    Variant2(ComposeCanvasLineToOp),
    /// Adds a cubic Bezier segment.
    Variant3(ComposeCanvasCubicToOp),
    /// Adds a quadratic Bezier segment.
    Variant4(ComposeCanvasQuadToOp),
    /// Closes the active contour.
    Variant5(ComposeCanvasCloseOp),
}
/// Whether canvas path geometry is filled or outlined.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeCanvasDrawStyle {
    /// Paints the interior of the geometry.
    #[serde(rename = "fill")]
    Fill,
    /// Paints an outline centered on the geometry boundary.
    #[serde(rename = "stroke")]
    Stroke,
}
/// Command that assembles path operations and renders the resulting geometry.
pub struct ComposeCanvasDrawPathCommand {
    /// Selects draw-path command decoding.
    pub r#type: ComposeCanvasDrawPathCommandType,
    /// Ordered operations that construct one or more path contours.
    pub path: Vec<ComposeCanvasPathOp>,
    /// Color used to fill or stroke the path.
    pub color: Option<ComposeColor>,
    /// Opacity multiplier applied to the path color.
    pub alpha: Option<f64>,
    /// Outline thickness when using stroke style.
    pub strokeWidth: Option<ComposeCanvasNumber>,
    /// Chooses interior fill or boundary stroke rendering.
    pub style: Option<ComposeCanvasDrawStyle>,
    /// Default unit for scalar path coordinates and stroke width.
    pub unit: Option<ComposeCanvasUnit>,
}
/// Draw-scope command for a filled or stroked rounded rectangle.
pub struct ComposeCanvasDrawRoundRectCommand {
    /// Selects draw-rounded-rectangle command decoding.
    pub r#type: ComposeCanvasDrawRoundRectCommandType,
    /// Horizontal coordinate of the rectangle origin.
    pub x: ComposeCanvasNumber,
    /// Vertical coordinate of the rectangle origin.
    pub y: ComposeCanvasNumber,
    /// Rectangle width.
    pub width: ComposeCanvasNumber,
    /// Rectangle height.
    pub height: ComposeCanvasNumber,
    /// Radius applied to each corner.
    pub cornerRadius: Option<ComposeCanvasNumber>,
    /// Gradient paint used instead of a solid color.
    pub brush: Option<ComposeCanvasBrush>,
    /// Solid paint used when no brush is supplied.
    pub color: Option<ComposeColor>,
    /// Opacity multiplier applied to the selected paint.
    pub alpha: Option<f64>,
    /// Outline thickness when using stroke style.
    pub strokeWidth: Option<ComposeCanvasNumber>,
    /// Chooses interior fill or boundary stroke rendering.
    pub style: Option<ComposeCanvasDrawStyle>,
    /// Default unit for scalar geometry and stroke width.
    pub unit: Option<ComposeCanvasUnit>,
}
/// Draw-scope command that lays out and paints text at a canvas position.
pub struct ComposeCanvasDrawTextCommand {
    /// Selects draw-text command decoding.
    pub r#type: ComposeCanvasDrawTextCommandType,
    /// Text content to lay out and render.
    pub text: String,
    /// Horizontal coordinate of the text layout origin.
    pub x: ComposeCanvasNumber,
    /// Vertical coordinate of the text layout origin.
    pub y: ComposeCanvasNumber,
    /// Color used to render glyphs.
    pub color: Option<ComposeColor>,
    /// Opacity multiplier applied to the glyph color.
    pub alpha: Option<f64>,
    /// Font size used by the text layout.
    pub fontSize: Option<ComposeCanvasNumber>,
    /// Font weight name or numeric weight understood by the renderer.
    pub fontWeight: Option<String>,
    /// Minimum width reserved for the text layout.
    pub minWidth: Option<ComposeCanvasNumber>,
    /// Maximum width before lines wrap or overflow.
    pub maxWidth: Option<ComposeCanvasNumber>,
    /// Minimum height reserved for the text layout.
    pub minHeight: Option<ComposeCanvasNumber>,
    /// Maximum height available to the text layout.
    pub maxHeight: Option<ComposeCanvasNumber>,
    /// Maximum number of rendered lines.
    pub maxLines: Option<f64>,
    /// Clipping behavior when text exceeds its layout bounds.
    pub overflow: Option<ComposeTextOverflow>,
    /// Default unit for scalar positions, dimensions, and font size.
    pub unit: Option<ComposeCanvasUnit>,
}
/// Draw-scope command that paints a named Material icon.
pub struct ComposeCanvasDrawIconCommand {
    /// Selects draw-icon command decoding.
    pub r#type: ComposeCanvasDrawIconCommandType,
    /// Material icon name resolved by the host icon registry.
    pub icon: String,
    /// Horizontal coordinate of the icon bounds.
    pub x: ComposeCanvasNumber,
    /// Vertical coordinate of the icon bounds.
    pub y: ComposeCanvasNumber,
    /// Width and height of the square icon bounds.
    pub size: Option<ComposeCanvasNumber>,
    /// Tint applied to the icon glyph.
    pub color: Option<ComposeColor>,
    /// Opacity multiplier applied to the tint.
    pub alpha: Option<f64>,
    /// Default unit for scalar position and size values.
    pub unit: Option<ComposeCanvasUnit>,
}
/// Drawing operation rendered in order by a Compose canvas node.
pub enum ComposeCanvasCommand {
    /// Strokes a straight line segment.
    Variant1(ComposeCanvasLineCommand),
    /// Draws an axis-aligned rectangle.
    Variant2(ComposeCanvasRectCommand),
    /// Draws a rounded rectangle using the basic command form.
    Variant3(ComposeCanvasRoundRectCommand),
    /// Draws a circle.
    Variant4(ComposeCanvasCircleCommand),
    /// Draws constrained text using the basic command form.
    Variant5(ComposeCanvasTextCommand),
    /// Builds and renders arbitrary path geometry.
    Variant6(ComposeCanvasDrawPathCommand),
    /// Draws a rounded rectangle using draw-style semantics.
    Variant7(ComposeCanvasDrawRoundRectCommand),
    /// Draws styled text using draw-scope semantics.
    Variant8(ComposeCanvasDrawTextCommand),
    /// Draws a named Material icon.
    Variant9(ComposeCanvasDrawIconCommand),
}
/// Serialized operation name stored in a Compose modifier chain.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ComposeModifierName {
    #[serde(rename = "fillMaxSize")]
    FillMaxSize,
    #[serde(rename = "fillMaxWidth")]
    FillMaxWidth,
    #[serde(rename = "fillMaxHeight")]
    FillMaxHeight,
    #[serde(rename = "width")]
    Width,
    #[serde(rename = "height")]
    Height,
    #[serde(rename = "requiredWidth")]
    RequiredWidth,
    #[serde(rename = "requiredHeight")]
    RequiredHeight,
    #[serde(rename = "size")]
    Size,
    #[serde(rename = "requiredSize")]
    RequiredSize,
    #[serde(rename = "padding")]
    Padding,
    #[serde(rename = "offset")]
    Offset,
    #[serde(rename = "widthIn")]
    WidthIn,
    #[serde(rename = "heightIn")]
    HeightIn,
    #[serde(rename = "sizeIn")]
    SizeIn,
    #[serde(rename = "requiredWidthIn")]
    RequiredWidthIn,
    #[serde(rename = "requiredHeightIn")]
    RequiredHeightIn,
    #[serde(rename = "requiredSizeIn")]
    RequiredSizeIn,
    #[serde(rename = "defaultMinSize")]
    DefaultMinSize,
    #[serde(rename = "wrapContentWidth")]
    WrapContentWidth,
    #[serde(rename = "wrapContentHeight")]
    WrapContentHeight,
    #[serde(rename = "wrapContentSize")]
    WrapContentSize,
    #[serde(rename = "aspectRatio")]
    AspectRatio,
    #[serde(rename = "alpha")]
    Alpha,
    #[serde(rename = "rotate")]
    Rotate,
    #[serde(rename = "scale")]
    Scale,
    #[serde(rename = "zIndex")]
    ZIndex,
    #[serde(rename = "background")]
    Background,
    #[serde(rename = "border")]
    Border,
    #[serde(rename = "clip")]
    Clip,
    #[serde(rename = "clipToBounds")]
    ClipToBounds,
    #[serde(rename = "shadow")]
    Shadow,
    #[serde(rename = "clickable")]
    Clickable,
    #[serde(rename = "combinedClickable")]
    CombinedClickable,
    #[serde(rename = "tapGestures")]
    TapGestures,
    #[serde(rename = "dragGestures")]
    DragGestures,
    #[serde(rename = "transformGestures")]
    TransformGestures,
    #[serde(rename = "onSizeChanged")]
    OnSizeChanged,
    #[serde(rename = "onGloballyPositioned")]
    OnGloballyPositioned,
    #[serde(rename = "imePadding")]
    ImePadding,
    #[serde(rename = "statusBarsPadding")]
    StatusBarsPadding,
    #[serde(rename = "navigationBarsPadding")]
    NavigationBarsPadding,
    #[serde(rename = "systemBarsPadding")]
    SystemBarsPadding,
    #[serde(rename = "safeDrawingPadding")]
    SafeDrawingPadding,
    #[serde(rename = "weight")]
    Weight,
    #[serde(rename = "align")]
    Align,
    #[serde(rename = "matchParentSize")]
    MatchParentSize,
}
/// One serialized operation in an immutable Compose modifier chain.
pub struct ComposeModifierOp {
    /// Operation applied by the host during node layout or drawing.
    pub name: ComposeModifierName,
    /// Positional arguments encoded for the selected operation.
    pub args: Option<Vec<serde_json::Value>>,
}
/// Serializable sequence of modifier operations attached to a Compose node.
pub struct ComposeModifierValue {
    /// Operations applied in chain order by the host renderer.
    pub __modifierOps: Vec<ComposeModifierOp>,
}
/// Stores a dynamically dispatched Compose modifier implementation.
pub type ComposeModifierProxy = Arc<dyn ComposeModifierProxyApi>;
/// Chainable layout, drawing, input, and positioning operations for Compose nodes.
pub trait ComposeModifierProxyApi: Send + Sync {
    /// Occupies the requested fraction of both available dimensions.
    fn fillMaxSize(&self, fraction: Option<f64>) -> ComposeModifierProxy;
    /// Occupies the requested fraction of the available width.
    fn fillMaxWidth(&self, fraction: Option<f64>) -> ComposeModifierProxy;
    /// Occupies the requested fraction of the available height.
    fn fillMaxHeight(&self, fraction: Option<f64>) -> ComposeModifierProxy;
    /// Requests a width that remains subject to parent constraints.
    fn width(&self, value: f64) -> ComposeModifierProxy;
    /// Requests a height that remains subject to parent constraints.
    fn height(&self, value: f64) -> ComposeModifierProxy;
    /// Forces the measured width even when it conflicts with parent constraints.
    fn requiredWidth(&self, value: f64) -> ComposeModifierProxy;
    /// Forces the measured height even when it conflicts with parent constraints.
    fn requiredHeight(&self, value: f64) -> ComposeModifierProxy;
    /// Requests the same constrained extent for width and height.
    fn size_overload_1(&self, value: f64) -> ComposeModifierProxy;
    /// Requests independent constrained width and height values.
    fn size_overload_2(&self, width: f64, height: f64) -> ComposeModifierProxy;
    /// Forces the same extent for width and height.
    fn requiredSize_overload_1(&self, value: f64) -> ComposeModifierProxy;
    /// Forces independent width and height values.
    fn requiredSize_overload_2(&self, width: f64, height: f64) -> ComposeModifierProxy;
    /// Adds one uniform inset between the node boundary and its content.
    fn padding_overload_1(&self, value: f64) -> ComposeModifierProxy;
    /// Adds separate horizontal and vertical content insets.
    fn padding_overload_2(&self, horizontal: f64, vertical: f64) -> ComposeModifierProxy;
    /// Adds explicit logical-start, top, logical-end, and bottom insets.
    fn padding_overload_3(
        &self,
        start: f64,
        top: f64,
        end: f64,
        bottom: f64,
    ) -> ComposeModifierProxy;
    /// Adds insets from a structure that can mix uniform, axis, and edge values.
    fn padding_overload_4(&self, values: ComposeModifierPadding) -> ComposeModifierProxy;
    /// Translates the laid-out node by horizontal and optional vertical offsets.
    fn offset_overload_1(&self, x: f64, y: Option<f64>) -> ComposeModifierProxy;
    /// Translates the laid-out node using structured axis offsets.
    fn offset_overload_2(&self, values: ComposeModifierOffset) -> ComposeModifierProxy;
    /// Constrains measured width to optional minimum and maximum values.
    fn widthIn_overload_1(&self, min: Option<f64>, max: Option<f64>) -> ComposeModifierProxy;
    /// Constrains measured width using shorthand and explicit width bounds.
    fn widthIn_overload_2(&self, bounds: ComposeModifierWidthBounds) -> ComposeModifierProxy;
    /// Constrains measured height to optional minimum and maximum values.
    fn heightIn_overload_1(&self, min: Option<f64>, max: Option<f64>) -> ComposeModifierProxy;
    /// Constrains measured height using shorthand and explicit height bounds.
    fn heightIn_overload_2(&self, bounds: ComposeModifierHeightBounds) -> ComposeModifierProxy;
    /// Constrains width and height with four explicit bounds.
    fn sizeIn_overload_1(
        &self,
        minWidth: f64,
        minHeight: f64,
        maxWidth: f64,
        maxHeight: f64,
    ) -> ComposeModifierProxy;
    /// Constrains both dimensions using a structured bounds object.
    fn sizeIn_overload_2(&self, bounds: ComposeModifierSizeBounds) -> ComposeModifierProxy;
    /// Forces width into the supplied optional range despite parent constraints.
    fn requiredWidthIn_overload_1(
        &self,
        min: Option<f64>,
        max: Option<f64>,
    ) -> ComposeModifierProxy;
    /// Forces width into structured shorthand and explicit bounds.
    fn requiredWidthIn_overload_2(
        &self,
        bounds: ComposeModifierWidthBounds,
    ) -> ComposeModifierProxy;
    /// Forces height into the supplied optional range despite parent constraints.
    fn requiredHeightIn_overload_1(
        &self,
        min: Option<f64>,
        max: Option<f64>,
    ) -> ComposeModifierProxy;
    /// Forces height into structured shorthand and explicit bounds.
    fn requiredHeightIn_overload_2(
        &self,
        bounds: ComposeModifierHeightBounds,
    ) -> ComposeModifierProxy;
    /// Forces both dimensions into four explicit bounds.
    fn requiredSizeIn_overload_1(
        &self,
        minWidth: f64,
        minHeight: f64,
        maxWidth: f64,
        maxHeight: f64,
    ) -> ComposeModifierProxy;
    /// Forces both dimensions into a structured set of bounds.
    fn requiredSizeIn_overload_2(&self, bounds: ComposeModifierSizeBounds) -> ComposeModifierProxy;
    /// Supplies preferred minimum width and optional minimum height.
    fn defaultMinSize_overload_1(
        &self,
        minWidth: f64,
        minHeight: Option<f64>,
    ) -> ComposeModifierProxy;
    /// Supplies preferred minimum dimensions from structured values.
    fn defaultMinSize_overload_2(
        &self,
        values: ComposeModifierDefaultMinSize,
    ) -> ComposeModifierProxy;
    /// Wraps to measured width using the host's default horizontal alignment.
    fn wrapContentWidth_overload_1(&self) -> ComposeModifierProxy;
    /// Wraps to measured width with explicit alignment and constraint handling.
    fn wrapContentWidth_overload_2(
        &self,
        align: ComposeHorizontalAlignment,
        unbounded: Option<bool>,
    ) -> ComposeModifierProxy;
    /// Wraps to measured width using structured alignment options.
    fn wrapContentWidth_overload_3(
        &self,
        options: ComposeModifierWrapContentWidthOptions,
    ) -> ComposeModifierProxy;
    /// Wraps to measured height using the host's default vertical alignment.
    fn wrapContentHeight_overload_1(&self) -> ComposeModifierProxy;
    /// Wraps to measured height with explicit alignment and constraint handling.
    fn wrapContentHeight_overload_2(
        &self,
        align: ComposeVerticalAlignment,
        unbounded: Option<bool>,
    ) -> ComposeModifierProxy;
    /// Wraps to measured height using structured alignment options.
    fn wrapContentHeight_overload_3(
        &self,
        options: ComposeModifierWrapContentHeightOptions,
    ) -> ComposeModifierProxy;
    /// Wraps both dimensions using the host's default box alignment.
    fn wrapContentSize_overload_1(&self) -> ComposeModifierProxy;
    /// Wraps both dimensions with explicit box alignment and constraint handling.
    fn wrapContentSize_overload_2(
        &self,
        align: ComposeBoxAlignment,
        unbounded: Option<bool>,
    ) -> ComposeModifierProxy;
    /// Wraps both dimensions using structured box-alignment options.
    fn wrapContentSize_overload_3(
        &self,
        options: ComposeModifierWrapContentSizeOptions,
    ) -> ComposeModifierProxy;
    /// Derives one measured dimension from the other using a width-to-height ratio.
    fn aspectRatio(&self, ratio: f64) -> ComposeModifierProxy;
    /// Multiplies the opacity of the node and its rendered descendants.
    fn alpha(&self, value: f64) -> ComposeModifierProxy;
    /// Rotates rendered content by the supplied angle.
    fn rotate(&self, value: f64) -> ComposeModifierProxy;
    /// Uniformly scales rendered content around its transform origin.
    fn scale(&self, value: f64) -> ComposeModifierProxy;
    /// Sets sibling draw order, with larger values rendered above smaller ones.
    fn zIndex(&self, value: f64) -> ComposeModifierProxy;
    /// Paints a solid color behind the node using an optional shape outline.
    fn background_overload_1(
        &self,
        value: ComposeColor,
        shape: Option<ComposeShape>,
    ) -> ComposeModifierProxy;
    /// Paints a gradient brush behind the node using an optional shape outline.
    fn background_overload_2(
        &self,
        value: ComposeCanvasBrush,
        shape: Option<ComposeShape>,
    ) -> ComposeModifierProxy;
    /// Strokes a solid-color border around an optional shape outline.
    fn border_overload_1(
        &self,
        width: f64,
        value: ComposeColor,
        shape: Option<ComposeShape>,
    ) -> ComposeModifierProxy;
    /// Strokes a gradient border around an optional shape outline.
    fn border_overload_2(
        &self,
        width: f64,
        value: ComposeCanvasBrush,
        shape: Option<ComposeShape>,
    ) -> ComposeModifierProxy;
    /// Clips the node and descendant drawing to the supplied shape.
    fn clip(&self, shape: ComposeShape) -> ComposeModifierProxy;
    /// Clips descendant drawing to the node's rectangular layout bounds.
    fn clipToBounds(&self) -> ComposeModifierProxy;
    /// Casts a shadow from explicit elevation, shape, and clipping values.
    fn shadow_overload_1(
        &self,
        elevation: f64,
        shape: Option<ComposeShape>,
        clip: Option<bool>,
    ) -> ComposeModifierProxy;
    /// Casts a shadow using structured elevation and outline options.
    fn shadow_overload_2(&self, options: ComposeModifierShadowOptions) -> ComposeModifierProxy;
    /// Makes the node respond to a primary click callback.
    fn clickable(
        &self,
        onClick: Arc<dyn Fn() -> ComposeModifierProxyClickableOnClickOutput + Send + Sync>,
    ) -> ComposeModifierProxy;
    /// Installs primary, long-press, and double-click recognition together.
    fn combinedClickable(
        &self,
        options: ComposeModifierCombinedClickableOptions,
    ) -> ComposeModifierProxy;
    /// Installs callbacks for press, tap, double-tap, and long-press gestures.
    fn tapGestures(&self, options: ComposeModifierTapGesturesOptions) -> ComposeModifierProxy;
    /// Installs callbacks for drag start, updates, completion, and cancellation.
    fn dragGestures(&self, options: ComposeModifierDragGesturesOptions) -> ComposeModifierProxy;
    /// Installs combined pan, pinch-zoom, and rotation recognition.
    fn transformGestures(
        &self,
        options: ComposeModifierTransformGesturesOptions,
    ) -> ComposeModifierProxy;
    /// Observes changes to the node's measured width and height.
    fn onSizeChanged(
        &self,
        onSizeChanged: Arc<
            dyn Fn(ComposeSizeChangedEvent) -> ComposeModifierProxyOnSizeChangedOnSizeChangedOutput
                + Send
                + Sync,
        >,
    ) -> ComposeModifierProxy;
    /// Observes node bounds in root and host-window coordinates after layout.
    fn onGloballyPositioned(
        &self,
        onGloballyPositioned: Arc<
            dyn Fn(
                    ComposeGloballyPositionedEvent,
                )
                    -> ComposeModifierProxyOnGloballyPositionedOnGloballyPositionedOutput
                + Send
                + Sync,
        >,
    ) -> ComposeModifierProxy;
    /// Adds padding matching the visible on-screen keyboard inset.
    fn imePadding(&self) -> ComposeModifierProxy;
    /// Adds padding matching the system status-bar inset.
    fn statusBarsPadding(&self) -> ComposeModifierProxy;
    /// Adds padding matching the system navigation-bar inset.
    fn navigationBarsPadding(&self) -> ComposeModifierProxy;
    /// Adds padding matching combined status and navigation bar insets.
    fn systemBarsPadding(&self) -> ComposeModifierProxy;
    /// Insets content from all system areas that can obscure drawing.
    fn safeDrawingPadding(&self) -> ComposeModifierProxy;
    /// Allocates remaining main-axis space proportionally within a row or column.
    fn weight(&self, weight: f64, fill: Option<bool>) -> ComposeModifierProxy;
    /// Overrides this child's placement within the parent layout scope.
    fn align(&self, alignment: ComposeModifierAlign) -> ComposeModifierProxy;
    /// Matches the final size of the containing box without affecting its measurement.
    fn matchParentSize(&self) -> ComposeModifierProxy;
    /// Serializes the accumulated modifier operations for transport to the host.
    fn toJSON(&self) -> ComposeModifierValue;
}
/// Typography and color overrides applied to editable text.
pub struct ComposeTextFieldStyle {
    /// Font size used for the entered value.
    pub fontSize: Option<f64>,
    /// Font weight name or numeric weight used for the entered value.
    pub fontWeight: Option<String>,
    /// Font family used for the entered value.
    pub fontFamily: Option<String>,
    /// Foreground color used for the entered value.
    pub color: Option<ComposeColor>,
}
/// Layout, drawing, identity, and lifecycle properties shared by Compose nodes.
pub struct ComposeCommonProps {
    /// Stable identity used to preserve node state across render passes.
    pub key: Option<String>,
    /// Runs after the host has created and loaded the node.
    pub onLoad: Option<Arc<dyn Fn() -> ComposeCommonPropsOnLoadOutput + Send + Sync>>,
    /// Content presented as the host screen's top-bar title.
    pub topBarTitle: Option<ComposeChildren>,
    /// Ordered modifier operations applied to layout, drawing, and input.
    pub modifier: Option<ComposeModifierValue>,
    /// Sibling draw order, with larger values rendered above smaller ones.
    pub zIndex: Option<f64>,
    /// Share of remaining main-axis space inside a row or column.
    pub weight: Option<f64>,
    /// Whether weighted content expands to occupy its entire allocated share.
    pub weightFill: Option<bool>,
    /// Requested node width before parent constraints are applied.
    pub width: Option<f64>,
    /// Requested node height before parent constraints are applied.
    pub height: Option<f64>,
    /// Whether the node expands to the maximum available height.
    pub fillMaxHeight: Option<bool>,
    /// Uniform or axis-specific space between node bounds and content.
    pub padding: Option<ComposeCommonPropsPadding>,
    /// Content inset on the layout-direction start edge.
    pub paddingStart: Option<f64>,
    /// Content inset on the top edge.
    pub paddingTop: Option<f64>,
    /// Content inset on the layout-direction end edge.
    pub paddingEnd: Option<f64>,
    /// Content inset applied to both start and end edges.
    pub paddingHorizontal: Option<f64>,
    /// Content inset applied to both top and bottom edges.
    pub paddingVertical: Option<f64>,
    /// Content inset on the bottom edge.
    pub paddingBottom: Option<f64>,
    /// Gap inserted between adjacent child nodes.
    pub spacing: Option<f64>,
    /// Whether the node expands to the maximum available width.
    pub fillMaxWidth: Option<bool>,
    /// Whether the node expands to both maximum available dimensions.
    pub fillMaxSize: Option<bool>,
    /// Solid background paint behind node content.
    pub background: Option<ComposeColor>,
    /// Alternate explicit background color accepted by legacy component props.
    pub backgroundColor: Option<ComposeColor>,
    /// Material container color used by surface-like components.
    pub containerColor: Option<ComposeColor>,
    /// Opacity multiplier applied to background paint.
    pub backgroundAlpha: Option<f64>,
    /// Gradient brush used instead of a solid background color.
    pub backgroundBrush: Option<ComposeCanvasBrush>,
    /// Shape that bounds and optionally clips the background paint.
    pub backgroundShape: Option<ComposeShape>,
}
/// Properties for laying out child nodes vertically.
pub struct ColumnProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes placed from top to bottom.
    pub content: Option<ComposeChildren>,
    /// Cross-axis alignment of children within the column width.
    pub horizontalAlignment: Option<ComposeAlignment>,
    /// Main-axis distribution of children within the column height.
    pub verticalArrangement: Option<ComposeArrangement>,
}
/// Properties for laying out child nodes horizontally.
pub struct RowProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes placed from start to end.
    pub content: Option<ComposeChildren>,
    /// Main-axis distribution of children within the row width.
    pub horizontalArrangement: Option<ComposeArrangement>,
    /// Cross-axis alignment of children within the row height.
    pub verticalAlignment: Option<ComposeAlignment>,
    /// Optional click action for the row as one interactive target.
    pub onClick: Option<Arc<dyn Fn() -> RowPropsOnClickOutput + Send + Sync>>,
}
/// Properties for stacking child nodes in the same layout bounds.
pub struct BoxProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes drawn in stacking order.
    pub content: Option<ComposeChildren>,
    /// Default placement of children within the box bounds.
    pub contentAlignment: Option<ComposeAlignment>,
}
/// Empty layout element that reserves explicit width and height.
pub struct SpacerProps {
    /// Horizontal space reserved by the element.
    pub width: Option<f64>,
    /// Vertical space reserved by the element.
    pub height: Option<f64>,
}
/// Properties for rendering one styled block of plain text.
pub struct TextProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// String rendered by the text layout.
    pub text: String,
    /// Material typography role supplying default metrics.
    pub style: Option<ComposeTextStyle>,
    /// Foreground color of rendered glyphs.
    pub color: Option<ComposeColor>,
    /// Font weight override applied after the typography role.
    pub fontWeight: Option<String>,
    /// Font size override applied after the typography role.
    pub fontSize: Option<f64>,
    /// Font family override used to resolve glyphs.
    pub fontFamily: Option<String>,
    /// Maximum number of lines before overflow handling.
    pub maxLines: Option<f64>,
    /// Whether text may wrap at soft line-break opportunities.
    pub softWrap: Option<bool>,
    /// Rendering behavior for text beyond the available lines or bounds.
    pub overflow: Option<ComposeTextOverflow>,
    /// Share of remaining main-axis space when placed in a row or column.
    pub weight: Option<f64>,
}
/// Properties for parsing and rendering Markdown content.
pub struct MarkdownProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Markdown source rendered by the component.
    pub text: String,
    /// Default foreground color for Markdown text.
    pub color: Option<ComposeColor>,
    /// Base font size from which Markdown styles are derived.
    pub fontSize: Option<f64>,
    /// Whether links or embedded actions may open host dialogs.
    pub enableDialogs: Option<bool>,
    /// Tag used to associate incremental Markdown updates with one stream.
    pub streamTagName: Option<String>,
}
/// State, decoration, validation, and typography for an editable text field.
pub struct TextFieldProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Plain or composed label displayed with the field.
    pub label: Option<TextFieldPropsLabel>,
    /// Plain or composed hint shown while the value is empty.
    pub placeholder: Option<TextFieldPropsPlaceholder>,
    /// Decoration placed before the editable text.
    pub leadingIcon: Option<ComposeChildren>,
    /// Decoration placed after the editable text.
    pub trailingIcon: Option<ComposeChildren>,
    /// Content rendered immediately before the entered value.
    pub prefix: Option<ComposeChildren>,
    /// Content rendered immediately after the entered value.
    pub suffix: Option<ComposeChildren>,
    /// Helper or validation content rendered beneath the field.
    pub supportingText: Option<ComposeChildren>,
    /// Current controlled text value.
    pub value: String,
    /// Receives each user-proposed value for controlled-state updates.
    pub onValueChange: Arc<dyn Fn(String) -> () + Send + Sync>,
    /// Restricts input and layout to one visual line.
    pub singleLine: Option<bool>,
    /// Minimum visible line count reserved by the field.
    pub minLines: Option<f64>,
    /// Maximum visible line count before internal scrolling.
    pub maxLines: Option<f64>,
    /// Allows selection without accepting user edits.
    pub readOnly: Option<bool>,
    /// Applies error-state semantics and styling.
    pub isError: Option<bool>,
    /// Obscures entered characters as sensitive password input.
    pub isPassword: Option<bool>,
    /// Typography and foreground overrides for the entered value.
    pub style: Option<ComposeTextFieldStyle>,
}
/// Controlled state and colors for a binary sliding switch.
pub struct SwitchProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Current on or off state.
    pub checked: bool,
    /// Receives the state requested by user interaction.
    pub onCheckedChange: Arc<dyn Fn(bool) -> () + Send + Sync>,
    /// Whether the switch accepts pointer and accessibility actions.
    pub enabled: Option<bool>,
    /// Optional content rendered inside the movable thumb.
    pub thumbContent: Option<ComposeChildren>,
    /// Thumb color while the switch is on.
    pub checkedThumbColor: Option<ComposeColor>,
    /// Track color while the switch is on.
    pub checkedTrackColor: Option<ComposeColor>,
    /// Thumb color while the switch is off.
    pub uncheckedThumbColor: Option<ComposeColor>,
    /// Track color while the switch is off.
    pub uncheckedTrackColor: Option<ComposeColor>,
}
/// Controlled state and interaction for a binary checkbox.
pub struct CheckboxProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Current selected or unselected state.
    pub checked: bool,
    /// Receives the state requested by user interaction.
    pub onCheckedChange: Arc<dyn Fn(bool) -> () + Send + Sync>,
    /// Whether the checkbox accepts pointer and accessibility actions.
    pub enabled: Option<bool>,
}
/// Content, shape, state, and action for a standard Material button.
pub struct ButtonProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Arbitrary composed content rendered inside the button.
    pub content: Option<ComposeChildren>,
    /// Plain label used when custom content is unnecessary.
    pub text: Option<String>,
    /// Whether the button accepts pointer and accessibility actions.
    pub enabled: Option<bool>,
    /// Action invoked when the button is activated.
    pub onClick: Arc<dyn Fn() -> ButtonPropsOnClickOutput + Send + Sync>,
    /// Horizontal and vertical inset around the button content.
    pub contentPadding: Option<ComposePadding>,
    /// Outline used for button background, border, and clipping.
    pub shape: Option<ComposeShape>,
}
/// Icon or custom content rendered as a compact clickable control.
pub struct IconButtonProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Arbitrary content rendered inside the icon-button bounds.
    pub content: Option<ComposeChildren>,
    /// Material icon name used when custom content is absent.
    pub icon: Option<String>,
    /// Whether the control accepts pointer and accessibility actions.
    pub enabled: Option<bool>,
    /// Action invoked when the icon button is activated.
    pub onClick: Arc<dyn Fn() -> IconButtonPropsOnClickOutput + Send + Sync>,
    /// Outline used for interaction feedback and clipping.
    pub shape: Option<ComposeShape>,
}
/// Elevated or outlined Material container for grouped content.
pub struct CardProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the card container.
    pub content: Option<ComposeChildren>,
    /// Background color of the card surface.
    pub containerColor: Option<ComposeColor>,
    /// Opacity multiplier applied to the card background.
    pub containerAlpha: Option<f64>,
    /// Default foreground color inherited by card content.
    pub contentColor: Option<ComposeColor>,
    /// Opacity multiplier inherited by card content.
    pub contentAlpha: Option<f64>,
    /// Outline used for card background, border, and clipping.
    pub shape: Option<ComposeShape>,
    /// Optional stroke around the card outline.
    pub border: Option<ComposeBorder>,
    /// Shadow elevation separating the card from its background.
    pub elevation: Option<f64>,
}
/// Material surface that supplies container color, content color, and shape.
pub struct SurfaceProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Child nodes rendered inside the surface.
    pub content: Option<ComposeChildren>,
    /// Background color of the surface.
    pub containerColor: Option<ComposeColor>,
    /// Default foreground color inherited by surface content.
    pub contentColor: Option<ComposeColor>,
    /// Outline used for surface painting, clipping, and interaction feedback.
    pub shape: Option<ComposeShape>,
    /// Opacity multiplier applied to the surface.
    pub alpha: Option<f64>,
    /// Optional action that turns the surface into an interactive target.
    pub onClick: Option<Arc<dyn Fn() -> SurfacePropsOnClickOutput + Send + Sync>>,
}
/// Properties for rendering and optionally animating a Material icon.
pub struct IconProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Material icon name resolved by the host icon registry.
    pub name: Option<String>,
    /// Color applied to the icon glyph.
    pub tint: Option<ComposeColor>,
    /// Width and height of the square icon bounds.
    pub size: Option<f64>,
    /// Whether the icon continuously rotates as a loading affordance.
    pub spin: Option<bool>,
    /// Duration in milliseconds of one full rotation.
    pub spinDurationMs: Option<f64>,
}
/// Vertically scrolling list whose child content is hosted by a lazy column.
pub struct LazyColumnProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// List content rendered in vertical order.
    pub content: Option<ComposeChildren>,
    /// Vertical gap between adjacent list children.
    pub spacing: Option<f64>,
}
/// Properties for a horizontal determinate or indeterminate progress indicator.
pub struct LinearProgressIndicatorProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Determinate completion fraction; omission selects indeterminate animation.
    pub progress: Option<f64>,
}
/// Appearance of an indeterminate circular progress indicator.
pub struct CircularProgressIndicatorProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Thickness of the animated circular arc.
    pub strokeWidth: Option<f64>,
    /// Color used to paint the animated arc.
    pub color: Option<ComposeColor>,
}
/// Host slot where queued snackbar messages are presented.
pub struct SnackbarHostProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
}
/// Properties for embedding the host AI chat content without its workspace panel.
pub struct AiChatProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
}
/// Properties for a responsive trailing panel controlled by the Compose screen.
pub struct AdaptiveSidePanelProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Controls whether the trailing panel is visible.
    pub open: bool,
    /// Content rendered inside the trailing panel.
    pub side: ComposeChildren,
    /// Receives visibility changes initiated by the host surface.
    pub onOpenChanged: Arc<dyn Fn(bool) -> () + Send + Sync>,
    /// Width used by default on wide layouts.
    pub defaultWidth: Option<f64>,
    /// Smallest permitted width on wide layouts.
    pub minWidth: Option<f64>,
    /// Minimum width reserved for the primary content on wide layouts.
    pub minContentWidth: Option<f64>,
    /// Viewport width at which the panel switches to overlay mode.
    pub breakpoint: Option<f64>,
}
/// Drawing commands, viewport transform, and gesture callbacks for a canvas node.
pub struct CanvasProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Drawing operations rendered sequentially into the canvas.
    pub commands: Option<Vec<ComposeCanvasCommand>>,
    /// Scale, translation, and pivot applied to canvas content.
    pub transform: Option<ComposeCanvasTransform>,
    /// Receives incremental pan, zoom, and rotation gestures over the canvas.
    pub onTransform: Option<Arc<dyn Fn(ComposeCanvasTransformEvent) -> () + Send + Sync>>,
    /// Receives measured canvas dimensions after layout changes.
    pub onSizeChanged: Option<Arc<dyn Fn(ComposeCanvasSizeEvent) -> () + Send + Sync>>,
}
/// Content source, platform settings, controller, and event hooks for an embedded WebView.
pub struct WebViewProps {
    /// Shared node layout, drawing, identity, and lifecycle properties.
    pub base_compose_common_props: ComposeCommonProps,
    /// Remote or local URL loaded as the primary document.
    pub url: Option<String>,
    /// Inline HTML loaded instead of a URL.
    pub html: Option<String>,
    /// Origin and relative-link base for inline HTML content.
    pub baseUrl: Option<String>,
    /// Media type assigned to inline content.
    pub mimeType: Option<String>,
    /// Character encoding used to decode inline content.
    pub encoding: Option<String>,
    /// Additional HTTP headers attached to the initial URL request.
    pub headers: Option<BTreeMap<String, String>>,
    /// Whether page scripts may execute.
    pub javaScriptEnabled: Option<bool>,
    /// Whether DOM local and session storage APIs are available.
    pub domStorageEnabled: Option<bool>,
    /// Whether page database storage is enabled by the platform WebView.
    pub databaseEnabled: Option<bool>,
    /// Whether scripts may open windows without a user gesture.
    pub javaScriptCanOpenWindowsAutomatically: Option<bool>,
    /// Whether the WebView handles requests for additional browser windows.
    pub supportMultipleWindows: Option<bool>,
    /// Whether pages may read resources through file URLs.
    pub allowFileAccess: Option<bool>,
    /// Whether pages may access platform content-provider URLs.
    pub allowContentAccess: Option<bool>,
    /// Whether a file-origin page may read other file URLs.
    pub allowFileAccessFromFileURLs: Option<bool>,
    /// Whether a file-origin page may request resources from any origin.
    pub allowUniversalAccessFromFileURLs: Option<bool>,
    /// User-Agent string sent with WebView requests.
    pub userAgent: Option<String>,
    /// Whether WebView scrolling participates in the surrounding Compose scroll chain.
    pub nestedScrollInterop: Option<bool>,
    /// Whether the page supports zoom gestures and controls.
    pub supportZoom: Option<bool>,
    /// Whether platform-provided zoom controls are enabled.
    pub builtInZoomControls: Option<bool>,
    /// Whether platform zoom controls are visibly overlaid on the page.
    pub displayZoomControls: Option<bool>,
    /// Whether layout uses a wide viewport based on page metadata.
    pub useWideViewPort: Option<bool>,
    /// Whether an oversized page initially scales down to fit the viewport.
    pub loadWithOverviewMode: Option<bool>,
    /// Policy for HTTP subresources requested by HTTPS pages.
    pub mixedContentMode: Option<ComposeWebViewMixedContentMode>,
    /// Whether audio and video playback must begin from a user gesture.
    pub mediaPlaybackRequiresUserGesture: Option<bool>,
    /// Percentage scale applied to page text independently of page zoom.
    pub textZoom: Option<f64>,
    /// Network cache policy for page and resource requests.
    pub cacheMode: Option<ComposeWebViewCacheMode>,
    /// Whether platform safe-browsing checks protect navigation.
    pub safeBrowsingEnabled: Option<bool>,
    /// Whether embedded third-party origins may store and send cookies.
    pub acceptThirdPartyCookies: Option<bool>,
    /// Imperative handle associated with this WebView across renders.
    pub controller: Option<ComposeWebViewController>,
    /// Runs when the main page begins navigating.
    pub onPageStarted: Option<
        Arc<dyn Fn(ComposeWebViewPageEvent) -> WebViewPropsOnPageStartedOutput + Send + Sync>,
    >,
    /// Runs when the main page finishes loading.
    pub onPageFinished: Option<
        Arc<dyn Fn(ComposeWebViewPageEvent) -> WebViewPropsOnPageFinishedOutput + Send + Sync>,
    >,
    /// Runs for network or content-loading failures.
    pub onReceivedError: Option<
        Arc<dyn Fn(ComposeWebViewErrorEvent) -> WebViewPropsOnReceivedErrorOutput + Send + Sync>,
    >,
    /// Runs when a requested resource returns an HTTP error status.
    pub onReceivedHttpError: Option<
        Arc<
            dyn Fn(ComposeWebViewHttpErrorEvent) -> WebViewPropsOnReceivedHttpErrorOutput
                + Send
                + Sync,
        >,
    >,
    /// Runs when TLS certificate validation fails.
    pub onReceivedSslError: Option<
        Arc<
            dyn Fn(ComposeWebViewSslErrorEvent) -> WebViewPropsOnReceivedSslErrorOutput
                + Send
                + Sync,
        >,
    >,
    /// Runs when page content initiates a file download.
    pub onDownloadStart: Option<
        Arc<dyn Fn(ComposeWebViewDownloadEvent) -> WebViewPropsOnDownloadStartOutput + Send + Sync>,
    >,
    /// Receives messages emitted by the page's JavaScript console.
    pub onConsoleMessage: Option<
        Arc<dyn Fn(ComposeWebViewConsoleEvent) -> WebViewPropsOnConsoleMessageOutput + Send + Sync>,
    >,
    /// Runs when the active or requested URL changes.
    pub onUrlChanged: Option<
        Arc<dyn Fn(ComposeWebViewNavigationEvent) -> WebViewPropsOnUrlChangedOutput + Send + Sync>,
    >,
    /// Runs as page loading completion advances.
    pub onProgressChanged: Option<
        Arc<
            dyn Fn(ComposeWebViewProgressEvent) -> WebViewPropsOnProgressChangedOutput
                + Send
                + Sync,
        >,
    >,
    /// Receives complete navigation and loading state snapshots.
    pub onStateChanged:
        Option<Arc<dyn Fn(ComposeWebViewState) -> WebViewPropsOnStateChangedOutput + Send + Sync>>,
    /// Receives native WebView creation, disposal, commit, and renderer events.
    pub onLifecycleEvent: Option<
        Arc<
            dyn Fn(ComposeWebViewLifecycleEvent) -> WebViewPropsOnLifecycleEventOutput
                + Send
                + Sync,
        >,
    >,
    /// Selects whether a navigation is allowed, cancelled, rewritten, or opened externally.
    pub onShouldOverrideUrlLoading: Option<
        Arc<
            dyn Fn(
                    ComposeWebViewNavigationRequest,
                ) -> JsOptional<WebViewPropsOnShouldOverrideUrlLoadingOutput>
                + Send
                + Sync,
        >,
    >,
    /// Selects whether a resource request is allowed, blocked, rewritten, or answered locally.
    pub onInterceptRequest: Option<
        Arc<
            dyn Fn(
                    ComposeWebViewResourceRequest,
                ) -> JsOptional<WebViewPropsOnInterceptRequestOutput>
                + Send
                + Sync,
        >,
    >,
}
/// Serializable element in the Compose DSL render tree.
pub struct ComposeNode {
    /// Component factory name resolved by the host renderer.
    pub r#type: String,
    /// Component-specific properties encoded for the selected node type.
    pub props: Option<BTreeMap<String, serde_json::Value>>,
    /// Ordered descendant nodes rendered inside this element.
    pub children: Option<Vec<ComposeNode>>,
}
/// Child content accepted by a Compose node factory.
pub enum ComposeChildren {
    /// Renders one child node.
    Variant1(ComposeNode),
    /// Renders an ordered list of child nodes.
    Variant2(Vec<ComposeNode>),
    /// Explicitly supplies no child content.
    Null,
    /// Leaves child content unspecified.
    Undefined,
}
/// Factory that encodes component properties and children as a Compose node.
pub type ComposeNodeFactory<TProps = BTreeMap<String, serde_json::Value>> =
    Arc<dyn Fn(Option<TProps>, Option<ComposeChildren>) -> ComposeNode + Send + Sync>;
/// Core component factories available through `ComposeDslContext::UI`.
pub struct ComposeUiFactoryRegistry {
    /// Creates a vertical layout container.
    pub Column: ComposeNodeFactory<ColumnProps>,
    /// Creates a horizontal layout container.
    pub Row: ComposeNodeFactory<RowProps>,
    /// Creates a stacking layout container.
    pub Box: ComposeNodeFactory<BoxProps>,
    /// Creates an empty element that reserves layout space.
    pub Spacer: ComposeNodeFactory<SpacerProps>,
    /// Creates a plain text element.
    pub Text: ComposeNodeFactory<TextProps>,
    /// Creates a Markdown-rendering element.
    pub Markdown: ComposeNodeFactory<MarkdownProps>,
    /// Creates a controlled editable text field.
    pub TextField: ComposeNodeFactory<TextFieldProps>,
    /// Creates a controlled binary switch.
    pub Switch: ComposeNodeFactory<SwitchProps>,
    /// Creates a controlled checkbox.
    pub Checkbox: ComposeNodeFactory<CheckboxProps>,
    /// Creates a standard Material button.
    pub Button: ComposeNodeFactory<ButtonProps>,
    /// Creates a compact icon button.
    pub IconButton: ComposeNodeFactory<IconButtonProps>,
    /// Creates an elevated or outlined card container.
    pub Card: ComposeNodeFactory<CardProps>,
    /// Creates a Material surface container.
    pub Surface: ComposeNodeFactory<SurfaceProps>,
    /// Creates a Material icon element.
    pub Icon: ComposeNodeFactory<IconProps>,
    /// Creates a vertically scrolling lazy list.
    pub LazyColumn: ComposeNodeFactory<LazyColumnProps>,
    /// Creates a horizontal progress indicator.
    pub LinearProgressIndicator: ComposeNodeFactory<LinearProgressIndicatorProps>,
    /// Creates a circular progress indicator.
    pub CircularProgressIndicator: ComposeNodeFactory<CircularProgressIndicatorProps>,
    /// Creates the presentation slot for queued snackbars.
    pub SnackbarHost: ComposeNodeFactory<SnackbarHostProps>,
    /// Embeds the host AI chat content without the workspace panel.
    pub AiChat: ComposeNodeFactory<AiChatProps>,
    /// Creates a responsive trailing panel around the supplied screen content.
    pub AdaptiveSidePanel: ComposeNodeFactory<AdaptiveSidePanelProps>,
    /// Creates a command-driven drawing surface.
    pub Canvas: ComposeNodeFactory<CanvasProps>,
    /// Creates an embedded platform WebView.
    pub WebView: ComposeNodeFactory<WebViewProps>,
}
/// Named scalar substitutions used by runtime message templates.
pub struct ComposeTemplateValues {
    /// Maps placeholder names to text, numeric, boolean, null, or undefined values.
    pub additional_properties: BTreeMap<String, JsOptional<ComposeTemplateValuesAdditionalValue>>,
}
/// Runtime-module metadata available to a Compose DSL screen.
pub struct ComposeUiModuleSpec {
    /// Identifier of the registered UI module.
    pub id: Option<String>,
    /// Runtime implementation selected for the module.
    pub runtime: Option<String>,
    /// Module-defined metadata retained alongside the standard fields.
    pub additional_properties: BTreeMap<String, serde_json::Value>,
}
/// Object-form request for invoking a tool from a Compose screen.
pub struct ComposeToolCallConfig {
    /// Optional tool category or namespace used during resolution.
    pub r#type: Option<String>,
    /// Tool name requested by the screen.
    pub name: String,
    /// Named arguments passed to the tool implementation.
    pub params: Option<BTreeMap<String, serde_json::Value>>,
}
/// Package context used to resolve a callable runtime tool name.
pub struct ComposeResolveToolNameRequest {
    /// Package that owns or imports the requested tool.
    pub packageName: Option<String>,
    /// Optional subpackage containing the requested tool.
    pub subpackageId: Option<String>,
    /// Public tool name to resolve.
    pub toolName: String,
    /// Whether an imported package binding should win over a local definition.
    pub preferImported: Option<bool>,
}
/// Selection filters and permission behavior for the host file picker.
pub struct ComposeFilePickerOptions {
    /// Accepted media types shown by the picker.
    pub mimeTypes: Option<Vec<String>>,
    /// Whether the user may select more than one file.
    pub allowMultiple: Option<bool>,
    /// Whether the host should retain URI access beyond the current session.
    pub persistPermission: Option<bool>,
}
/// File metadata returned by the host picker.
pub struct ComposePickedFile {
    /// Platform URI granting access to the selected content.
    pub uri: String,
    /// Resolved filesystem path when the provider exposes one.
    pub path: Option<String>,
    /// Display name reported by the content provider.
    pub name: Option<String>,
    /// Media type reported for the selected file.
    pub mimeType: Option<String>,
    /// File size in bytes when reported by the provider.
    pub size: JsOptional<f64>,
}
/// Outcome of a host file-picker request.
pub struct ComposeFilePickerResult {
    /// Whether the picker closed without an accepted selection.
    pub cancelled: bool,
    /// Files selected by the user; empty when the request was cancelled.
    pub files: Vec<ComposePickedFile>,
}
/// Discoverable navigation target exposed to a Compose DSL screen.
pub struct ComposeRouteInfo {
    /// Stable route identifier accepted by `navigate`.
    pub routeId: String,
    /// Runtime responsible for rendering the destination.
    pub runtime: String,
    /// Human-readable destination title when available.
    pub title: JsOptional<String>,
    /// Package that owns the destination.
    pub ownerPackageName: JsOptional<String>,
    /// Tool-package UI module rendered by the destination.
    pub toolPkgUiModuleId: JsOptional<String>,
}
/// Theme, modifier builder, and component factories supplied to a screen renderer.
pub struct ComposeDslContext {
    /// Active Material theme values resolved by the host.
    pub MaterialTheme: ComposeMaterialTheme,
    /// Empty modifier chain from which node modifiers are built.
    pub Modifier: ComposeModifierProxy,
    /// Core, Material 3, and host-registered component factories.
    pub UI: ComposeDslContextUIIntersection,
}
/// Stateful rendering and host-service operations available to Compose screens.
pub trait ComposeDslContextMethods: Send + Sync {
    /// Retains a keyed value across renders and returns a setter that schedules updates.
    fn useState<T>(&self, key: String, initialValue: T) -> (T, Arc<dyn Fn(T) -> () + Send + Sync>);
    /// Retains a keyed mutable value and returns a callback for replacing it.
    fn useMutable<T>(
        &self,
        key: String,
        initialValue: T,
    ) -> (T, Arc<dyn Fn(T) -> () + Send + Sync>);
    /// Returns a keyed stable reference whose current value survives rerenders.
    fn useRef<T>(&self, key: String, initialValue: T) -> ComposeDslContextMethodsUseRefReturn<T>;
    /// Reuses a keyed computed value until its dependency list changes.
    fn useMemo<T>(
        &self,
        key: String,
        factory: Arc<dyn Fn() -> T + Send + Sync>,
        deps: Option<Vec<serde_json::Value>>,
    ) -> T;
    /// Measures text using host font metrics and the supplied layout constraints.
    fn measureText(&self, request: ComposeTextMeasureRequest) -> ComposeTextMeasureResult;
    /// Invokes a resolved tool by name and asynchronously decodes its result.
    fn callTool<T>(
        &self,
        toolName: String,
        params: Option<BTreeMap<String, serde_json::Value>>,
    ) -> JsFuture<T>;
    /// Reads one runtime environment value by key.
    fn getEnv(&self, key: String) -> Option<String>;
    /// Writes one runtime environment value for subsequent screen operations.
    fn setEnv(&self, key: String, value: String) -> ComposeDslContextMethodsSetEnvReturn;
    /// Requests navigation to a route with optional destination arguments.
    fn navigate(
        &self,
        route: String,
        args: Option<BTreeMap<String, serde_json::Value>>,
    ) -> ComposeDslContextMethodsNavigateReturn;
    /// Presents a short host toast message.
    fn showToast(&self, message: String) -> ComposeDslContextMethodsShowToastReturn;
    /// Forwards a screen error to host diagnostics and user-facing handling.
    fn reportError(&self, error: serde_json::Value) -> ComposeDslContextMethodsReportErrorReturn;
    /// Creates or retrieves the keyed imperative controller for a WebView node.
    fn createWebViewController(&self, key: String) -> ComposeWebViewController;
    /// Opens the host file picker and resolves selected file metadata.
    fn openFilePicker(
        &self,
        options: Option<ComposeFilePickerOptions>,
    ) -> JsFuture<ComposeFilePickerResult>;
    /// Returns the registered runtime-module specification decoded as the requested type.
    fn getModuleSpec<TSpec>(&self) -> TSpec;
    /// Returns the package name that owns the current Compose DSL module.
    fn getCurrentPackageName(&self) -> Option<String>;
    /// Returns the identifier of the active tool package.
    fn getCurrentToolPkgId(&self) -> Option<String>;
    /// Returns the identifier of the UI module currently being rendered.
    fn getCurrentUiModuleId(&self) -> Option<String>;
    /// Interpolates named scalar values into placeholders in a message template.
    fn formatTemplate(&self, template: String, values: ComposeTemplateValues) -> String;
    /// Writes a batch of runtime environment entries.
    fn setEnvs(&self, values: BTreeMap<String, String>) -> ComposeDslContextMethodsSetEnvsReturn;
    /// Lists navigation targets currently available to the screen.
    fn listRoutes(&self) -> Vec<ComposeRouteInfo>;
    /// Lists routes owned directly by the host rather than plugin packages.
    fn getHostRoutes(&self) -> Vec<ComposeRouteInfo>;
    /// Calls a tool by resolved name using package-tool compatible parameters.
    fn toolCall_overload_1<T>(
        &self,
        toolName: String,
        toolParams: Option<BTreeMap<String, serde_json::Value>>,
    ) -> JsFuture<T>;
    /// Calls a tool selected by explicit category and name.
    fn toolCall_overload_2<T>(
        &self,
        toolType: String,
        toolName: String,
        toolParams: Option<BTreeMap<String, serde_json::Value>>,
    ) -> JsFuture<T>;
    /// Calls a tool using an object-form request.
    fn toolCall_overload_3<T>(&self, config: ComposeToolCallConfig) -> JsFuture<T>;
    /// Reports whether the named package is already imported into this runtime.
    fn isPackageImported(
        &self,
        packageName: String,
    ) -> ComposeDslContextMethodsIsPackageImportedReturn;
    /// Imports a package and returns the resulting package identifier.
    fn importPackage(&self, packageName: String) -> ComposeDslContextMethodsImportPackageReturn;
    /// Removes an imported package and returns the removed identifier.
    fn removePackage(&self, packageName: String) -> ComposeDslContextMethodsRemovePackageReturn;
    /// Selects an imported package for subsequent resolution and returns its identifier.
    fn usePackage(&self, packageName: String) -> ComposeDslContextMethodsUsePackageReturn;
    /// Lists package names imported into the current runtime.
    fn listImportedPackages(&self) -> ComposeDslContextMethodsListImportedPackagesReturn;
    /// Resolves a package-scoped public tool name to its callable runtime name.
    fn resolveToolName(
        &self,
        request: ComposeResolveToolNameRequest,
    ) -> ComposeDslContextMethodsResolveToolNameReturn;
    /// Constructs a raw Compose node for a component name, props, and children.
    fn h<TProps>(
        &self,
        r#type: String,
        props: Option<TProps>,
        children: Option<ComposeChildren>,
    ) -> ComposeNode;
}
/// Screen renderer that receives a host context and produces a root node tree.
pub type ComposeDslScreen = Arc<dyn Fn(ComposeDslContext) -> ComposeDslScreenOutput + Send + Sync>;
/// Defines numeric unit accessors installed on the JavaScript `Number` interface.
pub struct ComposeNumberExtensions {
    /// Converts the number to a pixel unit value.
    pub px: ComposeUnitValue,
    /// Converts the number to a density-independent pixel unit value.
    pub dp: ComposeUnitValue,
    /// Converts the number to a fractional unit value.
    pub fraction: ComposeUnitValue,
}
