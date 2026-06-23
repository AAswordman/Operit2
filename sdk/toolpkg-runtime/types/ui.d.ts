/**
 * UI operation type definitions for Assistance Package Tools
 */

import {
    UIPageResultData, UIActionResultData, CombinedOperationResultData, SimplifiedUINode
} from './results';

/**
 * UI operations namespace
 */
export namespace UI {
    /**
     * Get current page information
     */
    function getPageInfo(): Promise<UIPageResultData>;

    /**
     * Tap at coordinates
     * @param x - X coordinate
     * @param y - Y coordinate
     */
    function tap(x: number, y: number): Promise<UIActionResultData>;

    /**
     * Long press at coordinates
     * @param x - X coordinate
     * @param y - Y coordinate
     */
    function longPress(x: number, y: number): Promise<UIActionResultData>;



    /**
     * Click on an element
     * Multiple call patterns supported:
     * - clickElement(resourceId: string): Click by resource ID
     * - clickElement(bounds: string): Click by bounds "[x1,y1][x2,y2]"
     * - clickElement(params: object): Click using parameters object
     * - clickElement(type: "resourceId"|"className"|"bounds", value: string): Click by type
     * - clickElement(resourceId: string, index: number): Click by resource ID and index
     * @param param1 - Resource ID, bounds, or parameter object
     * @param param2 - Optional index or value
     * @param param3 - Optional index when using type+value
     */
    function clickElement(param1: string | { [key: string]: any }, param2?: string | number, param3?: number): Promise<UIActionResultData>;

    /**
     * Click on an element (detailed overloads matching implementation)
     * @param params - Parameters object with resourceId, className, text, contentDesc, bounds, etc.
     */
    function clickElement(params: {
        resourceId?: string,
        className?: string,
        text?: string,
        contentDesc?: string,
        bounds?: string,
        index?: number,
        partialMatch?: boolean,
        isClickable?: boolean
    }): Promise<UIActionResultData>;

    /**
     * Click on an element by resource ID
     * @param resourceId - Element resource ID to click
     */
    function clickElement(resourceId: string): Promise<UIActionResultData>;

    /**
     * Click on an element by bounds
     * @param bounds - Element bounds in format "[x1,y1][x2,y2]"
     */
    function clickElement(bounds: string): Promise<UIActionResultData>;

    /**
     * Click on an element by resource ID with index
     * @param resourceId - Element resource ID to click
     * @param index - Index of the element when multiple match (0-based)
     */
    function clickElement(resourceId: string, index: number): Promise<UIActionResultData>;

    /**
     * Click on an element by type and value
     * @param type - Type of identifier ("resourceId", "className", or "bounds")
     * @param value - Value for the specified type
     */
    function clickElement(type: "resourceId" | "className" | "bounds", value: string): Promise<UIActionResultData>;

    /**
     * Click on an element by type, value and index
     * @param type - Type of identifier ("resourceId" or "className")
     * @param value - Value for the specified type
     * @param index - Index of the element when multiple match (0-based)
     */
    function clickElement(type: "resourceId" | "className", value: string, index: number): Promise<UIActionResultData>;



    /**
     * Set text in input field
     * @param text - Text to input
     */
    function setText(text: string, resourceId?: string): Promise<UIActionResultData>;

    /**
     * Press a key
     * @param keyCode - Key code to press
     */
    function pressKey(keyCode: string): Promise<UIActionResultData>;

    /**
     * Swipe from one position to another
     * @param startX - Start X coordinate
     * @param startY - Start Y coordinate
     * @param endX - End X coordinate
     * @param endY - End Y coordinate
     */
    function swipe(startX: number, startY: number, endX: number, endY: number, duration?: number): Promise<UIActionResultData>;

    /**
     * Run the built-in UI automation subagent.
     * @param intent - High-level task description for the subagent (e.g. "打开微信并发送一条消息").
     * @param maxSteps - Optional maximum number of steps (default 20).
     * @param agentId - Optional agent id to reuse the same virtual screen session.
     * @param targetApp - Optional target app name/package name used for virtual display prewarm.
     */
    function runSubAgent(intent: string, maxSteps?: number, agentId?: string, targetApp?: string): Promise<AutomationExecutionResultData>;
}

/**
 * UINode - A powerful wrapper for Android UI elements with DOM-like operations
 * 
 * This class provides a convenient way to navigate, search, and interact with
 * Android UI elements. It wraps SimplifiedUINode objects and provides methods
 * similar to web DOM manipulation.
 */
export class UINode {
    /**
     * Create a new UINode instance
     * @param node - The SimplifiedUINode object to wrap
     * @param parent - The parent UINode (undefined for root)
     */
    constructor(node: SimplifiedUINode, parent?: UINode | undefined);

    // Core Properties

    /**
     * The class name of the node
     */
    readonly className: string | undefined;

    /**
     * The text content of the node
     */
    readonly text: string | undefined;

    /**
     * The content description of the node
     */
    readonly contentDesc: string | undefined;

    /**
     * The resource ID of the node
     */
    readonly resourceId: string | undefined;

    /**
     * The bounds of the node in format "[x1,y1][x2,y2]"
     */
    readonly bounds: string | undefined;

    /**
     * Whether the node is clickable
     */
    readonly isClickable: boolean;

    /**
     * The underlying wrapped SimplifiedUINode object
     */
    readonly rawNode: SimplifiedUINode;

    /**
     * The parent node of this element, or undefined if it's the root
     */
    readonly parent: UINode | undefined;

    /**
     * The path from root to this node as a string
     */
    readonly path: string;

    /**
     * The center point coordinates based on bounds
     */
    readonly centerPoint: { x: number, y: number } | undefined;

    /**
     * All children nodes
     */
    readonly children: UINode[];

    /**
     * The number of children
     */
    readonly childCount: number;

    // Text Extraction

    /**
     * Get all text content from this node and its descendants
     * @param trim - Whether to trim whitespace from text
     * @param skipEmpty - Whether to skip empty text values
     */
    allTexts(trim?: boolean, skipEmpty?: boolean): string[];

    /**
     * Get all text content as a single string
     * @param separator - String to join text values with
     */
    textContent(separator?: string): string;

    /**
     * Check if this node or any descendant contains the specified text
     * @param text - Text to search for
     * @param caseSensitive - Whether the search is case-sensitive
     */
    hasText(text: string, caseSensitive?: boolean): boolean;

    // Search Methods

    /**
     * Find the first descendant node matching the criteria
     * @param criteria - Search criteria or predicate function
     * @param deep - Whether to search recursively
     */
    find(criteria: object | ((node: UINode) => boolean), deep?: boolean): UINode | undefined;

    /**
     * Find all descendant nodes matching the criteria
     * @param criteria - Search criteria or predicate function
     * @param deep - Whether to search recursively
     */
    findAll(criteria: object | ((node: UINode) => boolean), deep?: boolean): UINode[];

    // Convenience Search Methods

    /**
     * Find a node by text content
     * @param text - Text to search for
     * @param options - Search options
     */
    findByText(text: string, options?: { exact?: boolean, caseSensitive?: boolean }): UINode | undefined;

    /**
     * Find nodes by text content
     * @param text - Text to search for
     * @param options - Search options
     */
    findAllByText(text: string, options?: { exact?: boolean, caseSensitive?: boolean }): UINode[];

    /**
     * Find a node by resource ID
     * @param id - Resource ID to search for
     * @param options - Search options
     */
    findById(id: string, options?: { exact?: boolean, caseSensitive?: boolean }): UINode | undefined;

    /**
     * Find nodes by resource ID
     * @param id - Resource ID to search for
     * @param options - Search options
     */
    findAllById(id: string, options?: { exact?: boolean, caseSensitive?: boolean }): UINode[];

    /**
     * Find a node by class name
     * @param className - Class name to search for
     * @param options - Search options
     */
    findByClass(className: string, options?: { exact?: boolean, caseSensitive?: boolean }): UINode | undefined;

    /**
     * Find nodes by class name
     * @param className - Class name to search for
     * @param options - Search options
     */
    findAllByClass(className: string, options?: { exact?: boolean, caseSensitive?: boolean }): UINode[];

    /**
     * Find a node by content description
     * @param description - Content description to search for
     * @param options - Search options
     */
    findByContentDesc(description: string, options?: { exact?: boolean, caseSensitive?: boolean }): UINode | undefined;

    /**
     * Find nodes by content description
     * @param description - Content description to search for
     * @param options - Search options
     */
    findAllByContentDesc(description: string, options?: { exact?: boolean, caseSensitive?: boolean }): UINode[];

    /**
     * Find all clickable nodes
     */
    findClickable(): UINode[];

    /**
     * Find closest ancestor that matches the criteria
     * @param criteria - Search criteria or predicate function
     */
    closest(criteria: object | ((node: UINode) => boolean)): UINode | undefined;

    // Actions

    /**
     * Click on this node
     */
    click(): Promise<UIActionResultData>;

    /**
     * Long press on this node
     */
    longPress(): Promise<UIActionResultData>;

    /**
     * Set text in this node (typically an input field)
     * @param text - Text to enter
     */
    setText(text: string): Promise<UIActionResultData>;

    /**
     * Wait for a specified time, then return an updated UI state
     * @param ms - Milliseconds to wait
     */
    wait(ms?: number): Promise<UINode>;

    /**
     * Click this node and wait for the UI to update
     * @param ms - Milliseconds to wait after clicking
     */
    clickAndWait(ms?: number): Promise<UINode>;

    /**
     * Long press this node and wait for the UI to update
     * @param ms - Milliseconds to wait after long pressing
     */
    longPressAndWait(ms?: number): Promise<UINode>;

    // Utility Methods

    /**
     * Convert to string representation
     */
    toString(): string;

    /**
     * Get a tree representation of this node and its descendants
     * @param indent - Indentation string for formatting
     */
    toTree(indent?: string): string;

    /**
     * Get a tree representation of this node and its descendants in Kotlin format
     * with filtering for relevant nodes only
     * @param indent - Indentation string for formatting
     */
    toTreeString(indent?: string): string;

    /**
     * Get a formatted string representation of the page info including
     * application package name, activity name, and UI hierarchy in tree format
     * (Only available on nodes created via fromPageInfo())
     */
    toFormattedString?(): string;

    /**
     * Check if this node and another are the same
     * @param other - Node to compare with
     */
    equals(other: UINode): boolean;

    // Static Methods

    /**
     * Create a UINode from a page info result
     * @param pageInfo - Page info from UI.getPageInfo()
     */
    static fromPageInfo(pageInfo: UIPageResultData): UINode;

    /**
     * Get the current page UI
     */
    static getCurrentPage(): Promise<UINode>;

    /**
     * Perform a search, wait, and return updated UI state
     * @param query - Search parameters
     * @param delayMs - Milliseconds to wait
     */
    static findAndWait(query: object, delayMs?: number): Promise<UINode>;

    /**
     * Click an element, wait, and return updated UI state
     * @param query - Element to click (search parameters)
     * @param delayMs - Milliseconds to wait
     */
    static clickAndWait(query: object, delayMs?: number): Promise<UINode>;

    /**
     * Long press an element, wait, and return updated UI state
     * @param query - Element to long press (search parameters)
     * @param delayMs - Milliseconds to wait
     */
    static longPressAndWait(query: object, delayMs?: number): Promise<UINode>;
}