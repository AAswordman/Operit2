//! UI inspection, element targeting, and interaction contracts exposed to plugins.
use super::results::*;
use super::{JsDate, JsFuture, JsObject, JsUndefined};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Holds an object-style set of element-selection criteria for a click operation.
pub struct UIHostClickElementParam1Variant2 {
    /// Stores legacy element criteria not represented by a dedicated typed field.
    #[serde(flatten)]
    pub additional_properties: BTreeMap<String, serde_json::Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts either a textual element locator or an object of selection criteria.
pub enum UIHostClickElementParam1 {
    Variant1(String),
    Variant2(UIHostClickElementParam1Variant2),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts the identifier value or match index used by a click overload.
pub enum UIHostClickElementParam2 {
    Variant1(String),
    Variant2(f64),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Describes the attributes used to locate one UI element for interaction.
pub struct UIHostClickElementParams {
    /// Matches the platform resource identifier assigned to the element.
    #[serde(rename = "resourceId")]
    pub resource_id: Option<String>,
    /// Matches the platform widget or accessibility class name.
    #[serde(rename = "className")]
    pub class_name: Option<String>,
    /// Matches the visible text associated with the element.
    pub text: Option<String>,
    /// Matches the accessibility content description.
    #[serde(rename = "contentDesc")]
    pub content_desc: Option<String>,
    /// Matches serialized screen bounds in `[x1,y1][x2,y2]` form.
    pub bounds: Option<String>,
    /// Selects one element when multiple nodes satisfy the criteria.
    pub index: Option<f64>,
    /// Allows textual criteria to match substrings instead of complete values.
    #[serde(rename = "partialMatch")]
    pub partial_match: Option<bool>,
    /// Restricts matches to nodes with the requested clickability state.
    #[serde(rename = "isClickable")]
    pub is_clickable: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects the element attribute interpreted by typed click overloads.
pub enum UIHostClickElementType {
    #[serde(rename = "resourceId")]
    ResourceId,
    #[serde(rename = "className")]
    ClassName,
    #[serde(rename = "bounds")]
    Bounds,
}
/// Inspects the current Android UI and performs element or coordinate interactions.
pub trait UIHost: Send + Sync {
    ///
    ///Get current page information
    ///
    fn getPageInfo(&self) -> JsFuture<UIPageResultData>;
    ///
    ///Tap at coordinates
    ///@param x - X coordinate
    ///@param y - Y coordinate
    ///
    fn tap(&self, x: f64, y: f64) -> JsFuture<UIActionResultData>;
    ///
    ///Long press at coordinates
    ///@param x - X coordinate
    ///@param y - Y coordinate
    ///
    fn longPress(&self, x: f64, y: f64) -> JsFuture<UIActionResultData>;
    ///
    ///Click on an element
    ///Multiple call patterns supported:
    ///- clickElement(resourceId: string): Click by resource ID
    ///- clickElement(bounds: string): Click by bounds "[x1,y1][x2,y2]"
    ///- clickElement(params: object): Click using parameters object
    ///- clickElement(type: "resourceId"|"className"|"bounds", value: string): Click by type
    ///- clickElement(resourceId: string, index: number): Click by resource ID and index
    ///@param param1 - Resource ID, bounds, or parameter object
    ///@param param2 - Optional index or value
    ///@param param3 - Optional index when using type+value
    ///
    fn clickElement_overload_1(
        &self,
        param1: UIHostClickElementParam1,
        param2: Option<UIHostClickElementParam2>,
        param3: Option<f64>,
    ) -> JsFuture<UIActionResultData>;
    ///
    ///Click on an element selected by structured attributes.
    ///@param params - Parameters object with resourceId, className, text, contentDesc, bounds, etc.
    ///
    fn clickElement_overload_2(
        &self,
        params: UIHostClickElementParams,
    ) -> JsFuture<UIActionResultData>;
    ///
    ///Click on an element by resource ID
    ///@param resourceId - Element resource ID to click
    ///
    fn clickElement_overload_3(&self, resourceId: String) -> JsFuture<UIActionResultData>;
    ///
    ///Click on an element by bounds
    ///@param bounds - Element bounds in format "[x1,y1][x2,y2]"
    ///
    fn clickElement_overload_4(&self, bounds: String) -> JsFuture<UIActionResultData>;
    ///
    ///Click on an element by resource ID with index
    ///@param resourceId - Element resource ID to click
    ///@param index - Index of the element when multiple match (0-based)
    ///
    fn clickElement_overload_5(
        &self,
        resourceId: String,
        index: f64,
    ) -> JsFuture<UIActionResultData>;
    ///
    ///Click on an element by type and value
    ///@param type - Type of identifier ("resourceId", "className", or "bounds")
    ///@param value - Value for the specified type
    ///
    fn clickElement_overload_6(
        &self,
        r#type: UIHostClickElementType,
        value: String,
    ) -> JsFuture<UIActionResultData>;
    ///
    ///Click on an element by type, value and index
    ///@param type - Type of identifier ("resourceId" or "className")
    ///@param value - Value for the specified type
    ///@param index - Index of the element when multiple match (0-based)
    ///
    fn clickElement_overload_7(
        &self,
        r#type: UIHostClickElementType,
        value: String,
        index: f64,
    ) -> JsFuture<UIActionResultData>;
    ///
    ///Set text in input field
    ///@param text - Text to input
    ///
    fn setText(&self, text: String, resourceId: Option<String>) -> JsFuture<UIActionResultData>;
    ///
    ///Press a key
    ///@param keyCode - Key code to press
    ///
    fn pressKey(&self, keyCode: String) -> JsFuture<UIActionResultData>;
    ///
    ///Swipe from one position to another
    ///@param startX - Start X coordinate
    ///@param startY - Start Y coordinate
    ///@param endX - End X coordinate
    ///@param endY - End Y coordinate
    ///
    fn swipe(
        &self,
        startX: f64,
        startY: f64,
        endX: f64,
        endY: f64,
        duration: Option<f64>,
    ) -> JsFuture<UIActionResultData>;
    ///
    ///Run the built-in UI automation subagent.
    ///@param intent - High-level task description for the subagent, such as opening an app and sending a message.
    ///@param maxSteps - Optional maximum number of steps (default 20).
    ///@param agentId - Optional agent id to reuse the same virtual screen session.
    ///@param targetApp - Optional target app name/package name used for virtual display prewarm.
    ///
    fn runSubAgent(
        &self,
        intent: String,
        maxSteps: Option<f64>,
        agentId: Option<String>,
        targetApp: Option<String>,
    ) -> JsFuture<AutomationExecutionResultData>;
}
/// Contains the center coordinates derived from a UI node's bounds.
pub struct UINodeCenterPoint {
    /// Contains the horizontal center coordinate.
    pub x: f64,
    /// Contains the vertical center coordinate.
    pub y: f64,
}
/// Selects UI nodes by object-based property filters or a predicate.
pub enum UINodeCriteria {
    /// Contains an object-based search description.
    Object(JsObject),
    /// Contains a predicate applied to candidate nodes.
    Predicate(Arc<dyn Fn(Arc<UINode>) -> bool + Send + Sync>),
}
/// Configures exact and case-sensitive UI node searches.
pub struct UINodeSearchOptions {
    /// Requires an exact property match.
    pub exact: Option<bool>,
    /// Enables case-sensitive string comparison.
    pub caseSensitive: Option<bool>,
}
/// Stores one Android UI element together with its hierarchy and geometry.
pub struct UINode {
    ///
    ///The class name of the node
    ///
    pub className: JsUndefined<String>,
    ///
    ///The text content of the node
    ///
    pub text: JsUndefined<String>,
    ///
    ///The content description of the node
    ///
    pub contentDesc: JsUndefined<String>,
    ///
    ///The resource ID of the node
    ///
    pub resourceId: JsUndefined<String>,
    ///
    ///The bounds of the node in format "[x1,y1][x2,y2]"
    ///
    pub bounds: JsUndefined<String>,
    ///
    ///Whether the node is clickable
    ///
    pub isClickable: bool,
    ///
    ///The underlying wrapped SimplifiedUINode object
    ///
    pub rawNode: SimplifiedUINode,
    ///
    ///The parent node of this element, or undefined if it's the root
    ///
    pub parent: JsUndefined<Arc<UINode>>,
    ///
    ///The path from root to this node as a string
    ///
    pub path: String,
    ///
    ///The center point coordinates based on bounds
    ///
    pub centerPoint: JsUndefined<UINodeCenterPoint>,
    ///
    ///All children nodes
    ///
    pub children: Vec<Arc<UINode>>,
    ///
    ///The number of children
    ///
    pub childCount: f64,
}
/// Traverses, searches, formats, and interacts with Android UI node trees.
pub trait UINodeMethods: Send + Sync {
    /// Constructs a node wrapper from a simplified node and optional parent.
    fn new(node: SimplifiedUINode, parent: Option<Arc<UINode>>) -> Arc<UINode>;
    ///
    ///Get all text content from this node and its descendants
    ///@param trim - Whether to trim whitespace from text
    ///@param skipEmpty - Whether to skip empty text values
    ///
    fn allTexts(&self, trim: Option<bool>, skipEmpty: Option<bool>) -> Vec<String>;
    ///
    ///Get all text content as a single string
    ///@param separator - String to join text values with
    ///
    fn textContent(&self, separator: Option<String>) -> String;
    ///
    ///Check if this node or any descendant contains the specified text
    ///@param text - Text to search for
    ///@param caseSensitive - Whether the search is case-sensitive
    ///
    fn hasText(&self, text: String, caseSensitive: Option<bool>) -> bool;
    ///
    ///Find the first descendant node matching the criteria
    ///@param criteria - Search criteria or predicate function
    ///@param deep - Whether to search recursively
    ///
    fn find(&self, criteria: UINodeCriteria, deep: Option<bool>) -> JsUndefined<Arc<UINode>>;
    ///
    ///Find all descendant nodes matching the criteria
    ///@param criteria - Search criteria or predicate function
    ///@param deep - Whether to search recursively
    ///
    fn findAll(&self, criteria: UINodeCriteria, deep: Option<bool>) -> Vec<Arc<UINode>>;
    ///
    ///Find a node by text content
    ///@param text - Text to search for
    ///@param options - Search options
    ///
    fn findByText(
        &self,
        text: String,
        options: Option<UINodeSearchOptions>,
    ) -> JsUndefined<Arc<UINode>>;
    ///
    ///Find nodes by text content
    ///@param text - Text to search for
    ///@param options - Search options
    ///
    fn findAllByText(&self, text: String, options: Option<UINodeSearchOptions>)
        -> Vec<Arc<UINode>>;
    ///
    ///Find a node by resource ID
    ///@param id - Resource ID to search for
    ///@param options - Search options
    ///
    fn findById(
        &self,
        id: String,
        options: Option<UINodeSearchOptions>,
    ) -> JsUndefined<Arc<UINode>>;
    ///
    ///Find nodes by resource ID
    ///@param id - Resource ID to search for
    ///@param options - Search options
    ///
    fn findAllById(&self, id: String, options: Option<UINodeSearchOptions>) -> Vec<Arc<UINode>>;
    ///
    ///Find a node by class name
    ///@param className - Class name to search for
    ///@param options - Search options
    ///
    fn findByClass(
        &self,
        className: String,
        options: Option<UINodeSearchOptions>,
    ) -> JsUndefined<Arc<UINode>>;
    ///
    ///Find nodes by class name
    ///@param className - Class name to search for
    ///@param options - Search options
    ///
    fn findAllByClass(
        &self,
        className: String,
        options: Option<UINodeSearchOptions>,
    ) -> Vec<Arc<UINode>>;
    ///
    ///Find a node by content description
    ///@param description - Content description to search for
    ///@param options - Search options
    ///
    fn findByContentDesc(
        &self,
        description: String,
        options: Option<UINodeSearchOptions>,
    ) -> JsUndefined<Arc<UINode>>;
    ///
    ///Find nodes by content description
    ///@param description - Content description to search for
    ///@param options - Search options
    ///
    fn findAllByContentDesc(
        &self,
        description: String,
        options: Option<UINodeSearchOptions>,
    ) -> Vec<Arc<UINode>>;
    ///
    ///Find all clickable nodes
    ///
    fn findClickable(&self) -> Vec<Arc<UINode>>;
    ///
    ///Find closest ancestor that matches the criteria
    ///@param criteria - Search criteria or predicate function
    ///
    fn closest(&self, criteria: UINodeCriteria) -> JsUndefined<Arc<UINode>>;
    ///
    ///Click on this node
    ///
    fn click(&self) -> JsFuture<UIActionResultData>;
    ///
    ///Long press on this node
    ///
    fn longPress(&self) -> JsFuture<UIActionResultData>;
    ///
    ///Set text in this node (typically an input field)
    ///@param text - Text to enter
    ///
    fn setText(&self, text: String) -> JsFuture<UIActionResultData>;
    ///
    ///Wait for a specified time, then return an updated UI state
    ///@param ms - Milliseconds to wait
    ///
    fn wait(&self, ms: Option<f64>) -> JsFuture<Arc<UINode>>;
    ///
    ///Click this node and wait for the UI to update
    ///@param ms - Milliseconds to wait after clicking
    ///
    fn clickAndWait(&self, ms: Option<f64>) -> JsFuture<Arc<UINode>>;
    ///
    ///Long press this node and wait for the UI to update
    ///@param ms - Milliseconds to wait after long pressing
    ///
    fn longPressAndWait(&self, ms: Option<f64>) -> JsFuture<Arc<UINode>>;
    #[allow(non_snake_case)]
    ///
    ///Convert to string representation
    ///
    fn toString(&self) -> String;
    ///
    ///Get a tree representation of this node and its descendants
    ///@param indent - Indentation string for formatting
    ///
    fn toTree(&self, indent: Option<String>) -> String;
    ///
    ///Get a tree representation of this node and its descendants in Kotlin format
    ///with filtering for relevant nodes only
    ///@param indent - Indentation string for formatting
    ///
    fn toTreeString(&self, indent: Option<String>) -> String;
    ///
    ///Get a formatted string representation of the page info including
    ///application package name, activity name, and UI hierarchy in tree format
    ///(Only available on nodes created via fromPageInfo())
    ///
    fn toFormattedString(&self) -> String;
    ///
    ///Check if this node and another are the same
    ///@param other - Node to compare with
    ///
    fn equals(&self, other: Arc<UINode>) -> bool;
    ///
    ///Create a UINode from a page info result
    ///@param pageInfo - Page info from UI.getPageInfo()
    ///
    fn fromPageInfo(pageInfo: UIPageResultData) -> Arc<UINode>;
    ///
    ///Get the current page UI
    ///
    fn getCurrentPage() -> JsFuture<Arc<UINode>>;
    ///
    ///Perform a search, wait, and return updated UI state
    ///@param query - Search parameters
    ///@param delayMs - Milliseconds to wait
    ///
    fn findAndWait(query: JsObject, delayMs: Option<f64>) -> JsFuture<Arc<UINode>>;
    ///
    ///Click an element, wait, and return updated UI state
    ///@param query - Element to click (search parameters)
    ///@param delayMs - Milliseconds to wait
    ///
    fn clickAndWait_overload_2(query: JsObject, delayMs: Option<f64>) -> JsFuture<Arc<UINode>>;
    ///
    ///Long press an element, wait, and return updated UI state
    ///@param query - Element to long press (search parameters)
    ///@param delayMs - Milliseconds to wait
    ///
    fn longPressAndWait_overload_2(query: JsObject, delayMs: Option<f64>) -> JsFuture<Arc<UINode>>;
}
