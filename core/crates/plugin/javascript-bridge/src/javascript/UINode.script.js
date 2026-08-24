/**
 * UINode - A powerful wrapper for Android UI elements with DOM-like operations
 * 
 * This class provides a convenient way to navigate, search, and interact with
 * Android UI elements. It wraps SimplifiedUINode objects and provides methods
 * similar to web DOM manipulation.
 * 
 * Key features:
 * - Search for elements by various attributes
 * - Chain search operations with results maintaining full functionality
 * - Extract text content from elements and their children
 * - Interact with elements (click, set text, etc.)
 * - Traverse element hierarchies (including parent references and node paths)
 * 
 * Example usage:
 * ```javascript
 * // Get current UI state
 * const ui = await UINode.getCurrentPage();
 * 
 * // Find and extract information
 * const buttonTexts = ui.findByClass('Button').allTexts();
 * 
 * // Search with multiple criteria
 * const loginBtn = ui.find({text: 'Login', clickable: true});
 * 
 * // Chain operations
 * await ui.find({className: 'EditText'}).setText('username');
 * 
 * // Display UI hierarchy in Kotlin format
 * console.log(ui.toTreeString());
 * 
 * // Get complete formatted page info (application, activity and UI elements)
 * console.log(ui.toFormattedString());
 * 
 * // Get node path
 * console.log(loginBtn.path);
 * 
 * // Navigate to parent
 * const parent = loginBtn.parent;
 * ```
 */
class UINode {
    /**
     * Create a new UINode instance
     * @param {Object} node - The SimplifiedUINode object to wrap
     * @param {UINode|undefined} parent - The parent UINode (undefined for root)
     */
    constructor(node, parent = undefined) {
        this._node = node || {};
        this._children = undefined;
        this._parent = parent;
    }

    // ===== Core Properties =====

    /**
     * Get the class name of the node
     * @return {string|undefined} Class name or undefined if not available
     */
    get className() {
        return this._node.className !== undefined ? this._node.className : undefined;
    }

    /**
     * Get the text content of the node
     * @return {string|undefined} Text content or undefined if not available
     */
    get text() {
        return this._node.text !== undefined ? this._node.text : undefined;
    }

    /**
     * Get the content description of the node
     * @return {string|undefined} Content description or undefined if not available
     */
    get contentDesc() {
        return this._node.contentDesc !== undefined ? this._node.contentDesc : undefined;
    }

    /**
     * Get the resource ID of the node
     * @return {string|undefined} Resource ID or undefined if not available
     */
    get resourceId() {
        return this._node.resourceId !== undefined ? this._node.resourceId : undefined;
    }

    /**
     * Get the bounds of the node
     * @return {string|undefined} Bounds in format "[x1,y1][x2,y2]" or undefined if not available
     */
    get bounds() {
        return this._node.bounds !== undefined ? this._node.bounds : undefined;
    }

    /**
     * Check if the node is clickable
     * @return {boolean} True if clickable, false otherwise
     */
    get isClickable() {
        return Boolean(this._node.isClickable);
    }

    /**
     * Get the underlying raw node
     * @return {Object} The wrapped SimplifiedUINode object
     */
    get rawNode() {
        return this._node;
    }

    /**
     * Get the parent node
     * @return {UINode|undefined} Parent node or undefined if this is the root
     */
    get parent() {
        return this._parent;
    }

    /**
     * Get the path from root to this node
     * @return {string} Path representation as a string
     */
    get path() {
        // Start with this node's identifier
        let identifier = this._getNodeIdentifier();

        // Build path by traversing up the parent chain
        let currentNode = this;
        let pathParts = [identifier];

        while (currentNode.parent) {
            currentNode = currentNode.parent;
            identifier = currentNode._getNodeIdentifier();
            pathParts.unshift(identifier);
        }

        return pathParts.join(" > ");
    }

    /**
     * Get a string identifier for this node to use in the path
     * @private
     * @return {string} A string identifying this node
     */
    _getNodeIdentifier() {
        // Try to use resourceId first as it's most specific
        if (this.resourceId) {
            // Extract just the ID name without the package
            const idParts = this.resourceId.split('/');
            return `#${idParts[idParts.length - 1]}`;
        }

        // Use text content if available
        if (this.text) {
            // Truncate long text
            const displayText = this.text.length > 20
                ? this.text.substring(0, 17) + "..."
                : this.text;
            return `"${displayText}"`;
        }

        // Use content description
        if (this.contentDesc) {
            const displayDesc = this.contentDesc.length > 20
                ? this.contentDesc.substring(0, 17) + "..."
                : this.contentDesc;
            return `[desc="${displayDesc}"]`;
        }

        // Fall back to class name with index
        if (this.className) {
            // Get last part of class name (e.g., "android.widget.Button" -> "Button")
            const classNameParts = this.className.split('.');
            const shortClassName = classNameParts[classNameParts.length - 1];

            // If we have a parent, try to determine our index among siblings of same class
            if (this.parent) {
                const siblingsOfSameClass = this.parent.children.filter(
                    child => child.className === this.className
                );
                if (siblingsOfSameClass.length > 1) {
                    const index = siblingsOfSameClass.indexOf(this);
                    if (index !== -1) {
                        return `${shortClassName}[${index}]`;
                    }
                }
            }

            return shortClassName;
        }

        // Last resort
        return "Node";
    }

    /**
     * Get the center point coordinates based on bounds
     * @return {Object|undefined} Object with x and y coordinates or undefined if bounds not available
     */
    get centerPoint() {
        if (!this.bounds) return undefined;

        try {
            // Parse bounds format "[x1,y1][x2,y2]"
            const boundsMatch = this.bounds.match(/\[(\d+),(\d+)\]\[(\d+),(\d+)\]/);
            if (boundsMatch) {
                const x1 = parseInt(boundsMatch[1], 10);
                const y1 = parseInt(boundsMatch[2], 10);
                const x2 = parseInt(boundsMatch[3], 10);
                const y2 = parseInt(boundsMatch[4], 10);

                return {
                    x: Math.floor((x1 + x2) / 2),
                    y: Math.floor((y1 + y2) / 2)
                };
            }
        } catch (e) {
            console.error("Error parsing bounds:", e);
        }
        return undefined;
    }

    /**
     * Get all children nodes
     * @return {UINode[]} Array of UINode objects representing children
     */
    get children() {
        if (this._children === undefined) {
            this._children = [];

            if (Array.isArray(this._node.children)) {
                this._children = this._node.children.map(child => new UINode(child, this));
            }
        }

        return this._children;
    }

    /**
     * Get the number of children
     * @return {number} Number of children
     */
    get childCount() {
        return this.children.length;
    }

    // ===== Text Extraction =====

    /**
     * Get all text content from this node and its descendants
     * @param {boolean} [trim=true] - Whether to trim whitespace from text
     * @param {boolean} [skipEmpty=true] - Whether to skip empty text values
     * @return {string[]} Array of text values
     */
    allTexts(trim = true, skipEmpty = true) {
        const result = [];

        // Add this node's text if it exists and meets criteria
        if (this.text) {
            const nodeText = trim ? this.text.trim() : this.text;
            if (!skipEmpty || nodeText) {
                result.push(nodeText);
            }
        }

        // Add text from all descendants through recursive search
        this._collectTextsRecursive(this, result, trim, skipEmpty);

        return result;
    }

    /**
     * Helper method to collect texts recursively
     * @private
     */
    _collectTextsRecursive(node, result, trim, skipEmpty) {
        for (const child of node.children) {
            if (child.text) {
                const childText = trim ? child.text.trim() : child.text;
                if (!skipEmpty || childText) {
                    result.push(childText);
                }
            }

            this._collectTextsRecursive(child, result, trim, skipEmpty);
        }
    }

    /**
     * Get all text content as a single string
     * @param {string} [separator=" "] - String to join text values with
     * @return {string} Combined text content
     */
    textContent(separator = " ") {
        return this.allTexts().join(separator);
    }

    /**
     * Check if this node or any descendant contains the specified text
     * @param {string} text - Text to search for
     * @param {boolean} [caseSensitive=true] - Whether the search is case-sensitive
     * @return {boolean} True if text is found
     */
    hasText(text, caseSensitive = true) {
        if (!text) return false;

        // Helper function for text comparison
        const matches = (nodeText) => {
            if (!nodeText) return false;

            if (caseSensitive) {
                return nodeText.includes(text);
            } else {
                return nodeText.toLowerCase().includes(text.toLowerCase());
            }
        };

        // Check current node
        if (matches(this.text)) {
            return true;
        }

        // Check descendants
        return this.find(node => matches(node.text)) !== undefined;
    }

    // ===== Search Methods =====

    /**
     * Find the first descendant node matching the criteria
     * @param {Object|Function} criteria - Search criteria or predicate function
     * @param {boolean} [deep=true] - Whether to search recursively
     * @return {UINode|undefined} Matching node or undefined if not found
     */
    find(criteria, deep = true) {
        // Handle function predicate
        if (typeof criteria === 'function') {
            return this._findByPredicate(criteria, deep);
        }

        // Handle object criteria
        if (typeof criteria === 'object') {
            const predicate = this._createPredicateFromCriteria(criteria);
            return this._findByPredicate(predicate, deep);
        }

        // Invalid criteria
        console.error("Invalid search criteria:", criteria);
        return undefined;
    }

    /**
     * Find all descendant nodes matching the criteria
     * @param {Object|Function} criteria - Search criteria or predicate function
     * @param {boolean} [deep=true] - Whether to search recursively
     * @return {UINode[]} Array of matching nodes
     */
    findAll(criteria, deep = true) {
        // Handle function predicate
        if (typeof criteria === 'function') {
            return this._findAllByPredicate(criteria, deep);
        }

        // Handle object criteria
        if (typeof criteria === 'object') {
            const predicate = this._createPredicateFromCriteria(criteria);
            return this._findAllByPredicate(predicate, deep);
        }

        // Invalid criteria
        console.error("Invalid search criteria:", criteria);
        return [];
    }

    /**
     * Implementation of find with predicate function
     * @private
     */
    _findByPredicate(predicate, deep) {
        // Check direct children
        for (const child of this.children) {
            try {
                if (predicate(child)) {
                    return child;
                }
            } catch (error) {
                // Skip this child if predicate throws an error
                console.warn("Predicate error for child node:", error.message);
                continue;
            }

            // Recursively check descendants if enabled
            if (deep) {
                const found = child._findByPredicate(predicate, true);
                if (found) {
                    return found;
                }
            }
        }

        return undefined;
    }

    /**
     * Implementation of findAll with predicate function
     * @private
     */
    _findAllByPredicate(predicate, deep) {
        const results = [];

        // Helper function for recursive search
        const collect = (node, deep) => {
            for (const child of node.children) {
                try {
                    if (predicate(child)) {
                        results.push(child);
                    }
                } catch (error) {
                    // Skip this child if predicate throws an error
                    console.warn("Predicate error for child node:", error.message);
                    continue;
                }

                if (deep) {
                    collect(child, true);
                }
            }
        };

        collect(this, deep);
        return results;
    }

    /**
     * Create a predicate function from criteria object
     * @private
     */
    _createPredicateFromCriteria(criteria) {
        // Extract options
        const { exact = true, caseSensitive = true, ...actualCriteria } = criteria;

        return (node) => {
            try {
                // Check each specified criterion
                for (const [key, value] of Object.entries(actualCriteria)) {
                    // Special handling for clickable property
                    if (key === 'clickable') {
                        if (node.isClickable !== value) {
                            return false;
                        }
                        continue;
                    }

                    // Handle text properties (text, resourceId, className, contentDesc)
                    const nodeValue = node[key];

                    // If the node doesn't have the property, it's not a match
                    if (nodeValue === undefined || nodeValue === null) {
                        return false;
                    }

                    // For text properties, compare based on exact and case sensitivity options
                    if (typeof nodeValue === 'string' && typeof value === 'string') {
                        if (exact) {
                            // Exact matching
                            if (caseSensitive) {
                                if (nodeValue !== value) return false;
                            } else {
                                if (nodeValue.toLowerCase() !== value.toLowerCase()) return false;
                            }
                        } else {
                            // Substring matching
                            if (caseSensitive) {
                                if (!nodeValue.includes(value)) return false;
                            } else {
                                if (!nodeValue.toLowerCase().includes(value.toLowerCase())) return false;
                            }
                        }
                    } else {
                        // For non-string properties, use strict equality
                        if (nodeValue !== value) return false;
                    }
                }

                return true;
            } catch (error) {
                // If any error occurs during property access or comparison, treat as non-match
                console.warn("Error in predicate evaluation:", error.message);
                return false;
            }
        };
    }

    // ===== Convenience Search Methods =====

    /**
     * Find a node by text content
     * @param {string} text - Text to search for
     * @param {Object} [options] - Search options
     * @param {boolean} [options.exact=true] - Whether to match exactly
     * @param {boolean} [options.caseSensitive=true] - Whether search is case-sensitive
     * @return {UINode|undefined} Matching node or undefined if not found
     */
    findByText(text, options = {}) {
        return this.find({ text, ...options });
    }

    /**
     * Find nodes by text content
     * @param {string} text - Text to search for
     * @param {Object} [options] - Search options
     * @return {UINode[]} Array of matching nodes
     */
    findAllByText(text, options = {}) {
        return this.findAll({ text, ...options });
    }

    /**
     * Find a node by resource ID
     * @param {string} id - Resource ID to search for
     * @param {Object} [options] - Search options
     * @return {UINode|undefined} Matching node or undefined if not found
     */
    findById(id, options = {}) {
        // Default to non-exact matching for resource IDs to support partial ID matching
        const searchOptions = { exact: false, ...options };
        return this.find({ resourceId: id, ...searchOptions });
    }

    /**
     * Find nodes by resource ID
     * @param {string} id - Resource ID to search for
     * @param {Object} [options] - Search options
     * @return {UINode[]} Array of matching nodes
     */
    findAllById(id, options = {}) {
        // Default to non-exact matching for resource IDs to support partial ID matching
        const searchOptions = { exact: false, ...options };
        return this.findAll({ resourceId: id, ...searchOptions });
    }

    /**
     * Find a node by class name
     * @param {string} className - Class name to search for
     * @param {Object} [options] - Search options
     * @return {UINode|undefined} Matching node or undefined if not found
     */
    findByClass(className, options = {}) {
        return this.find({ className, ...options });
    }

    /**
     * Find nodes by class name
     * @param {string} className - Class name to search for
     * @param {Object} [options] - Search options
     * @return {UINode[]} Array of matching nodes
     */
    findAllByClass(className, options = {}) {
        return this.findAll({ className, ...options });
    }

    /**
     * Find a node by content description
     * @param {string} description - Content description to search for
     * @param {Object} [options] - Search options
     * @return {UINode|undefined} Matching node or undefined if not found
     */
    findByContentDesc(description, options = {}) {
        return this.find({ contentDesc: description, ...options });
    }

    /**
     * Find nodes by content description
     * @param {string} description - Content description to search for
     * @param {Object} [options] - Search options
     * @return {UINode[]} Array of matching nodes
     */
    findAllByContentDesc(description, options = {}) {
        return this.findAll({ contentDesc: description, ...options });
    }

    /**
     * Find all clickable nodes
     * @return {UINode[]} Array of clickable nodes
     */
    findClickable() {
        return this.findAll({ clickable: true });
    }

    /**
     * Find closest ancestor that matches the criteria
     * @param {Object|Function} criteria - Search criteria or predicate function
     * @return {UINode|undefined} The matching ancestor or undefined if none found
     */
    closest(criteria) {
        let currentNode = this.parent;

        // Convert object criteria to a predicate function if needed
        const predicate = typeof criteria === 'function'
            ? criteria
            : this._createPredicateFromCriteria(criteria);

        // Traverse up the ancestry chain
        while (currentNode) {
            if (predicate(currentNode)) {
                return currentNode;
            }
            currentNode = currentNode.parent;
        }

        return undefined;
    }

    // ===== Actions =====

    /**
     * Click on this node
     * @return {Promise<Object>} Result of the click operation
     */
    async click() {
        if (!this.isClickable) {
            console.warn("Attempting to click on a non-clickable element:", this.toString());
        }

        // Try to click using coordinates if bounds are available
        const point = this.centerPoint;
        if (point) {
            return Tools.UI.clickElement({ bounds: this.bounds });
        }

        // Fall back to other identifiers
        if (this.resourceId) {
            return Tools.UI.clickElement({ resourceId: this.resourceId });
        }

        if (this.text) {
            return Tools.UI.clickElement({ text: this.text });
        }

        if (this.contentDesc) {
            return Tools.UI.clickElement({ contentDesc: this.contentDesc });
        }

        throw new Error("Cannot click element: no suitable identifier found");
    }

    /**
     * Set text in this node (typically an input field)
     * @param {string} text - Text to enter
     * @return {Promise<Object>} Result of the operation
     */
    async setText(text) {
        // First click to focus
        await this.click();

        // Then set the text
        return Tools.UI.setText(text);
    }

    /**
     * Wait for a specified time, then return an updated UI state
     * @param {number} [ms=1000] - Milliseconds to wait
     * @return {Promise<UINode>} New UINode with updated state
     */
    async wait(ms = 1000) {
        await new Promise(resolve => setTimeout(resolve, ms));
        return UINode.getCurrentPage();
    }

    /**
     * Click this node and wait for the UI to update
     * @param {number} [ms=1000] - Milliseconds to wait after clicking
     * @return {Promise<UINode>} New UINode with updated state
     */
    async clickAndWait(ms = 1000) {
        await this.click();
        return this.wait(ms);
    }

    /**
     * Long press on this node
     * @return {Promise<Object>} Result of the long press operation
     */
    async longPress() {
        // Try to long press using coordinates if bounds are available
        const point = this.centerPoint;
        if (point) {
            return Tools.UI.longPress(point.x, point.y);
        }

        throw new Error("Cannot long press element: no bounds available to determine coordinates");
    }

    /**
     * Long press this node and wait for the UI to update
     * @param {number} [ms=1000] - Milliseconds to wait after long pressing
     * @return {Promise<UINode>} New UINode with updated state
     */
    async longPressAndWait(ms = 1000) {
        await this.longPress();
        return this.wait(ms);
    }

    // ===== Utility Methods =====

    /**
     * Check if this node should be included in the tree representation
     * Based on the Kotlin shouldKeepNode() implementation
     * @private
     * @return {boolean} True if node should be kept
     */
    _shouldKeepNode() {
        // Key element types to always include
        const keyElements = [
            'Button', 'TextView', 'EditText',
            'ScrollView', 'Switch', 'ImageView'
        ];

        // Keep conditions: key element types or has content or clickable
        const isKeyElement = this.className && keyElements.includes(this.className);
        const hasContent = Boolean(this.text) || Boolean(this.contentDesc);

        // Check if any children should be kept
        const hasKeepableChildren = this.children.some(child => child._shouldKeepNode());

        return isKeyElement || hasContent || this.isClickable || hasKeepableChildren;
    }

    /**
     * Get a tree representation of this node and its descendants in Kotlin format
     * @param {string} [indent=""] - Indentation string for formatting
     * @return {string} Tree representation matching the Kotlin implementation
     */
    toTreeString(indent = "") {
        if (!this._shouldKeepNode()) return "";

        let result = "";

        // Node identifier with clickable indicator
        result += indent;
        result += this.isClickable ? "鈻?" : "鈼?";

        // Class name
        if (this.className) {
            result += `[${this.className}] `;
        }

        // Text content (maximum 30 characters)
        if (this.text) {
            const displayText = this.text.length > 30
                ? this.text.substring(0, 27) + "..."
                : this.text;
            result += `T: "${displayText}" `;
        }

        // Content description
        if (this.contentDesc) {
            result += `D: "${this.contentDesc}" `;
        }

        // Resource ID
        if (this.resourceId) {
            result += `ID: ${this.resourceId} `;
        }

        // Bounds
        if (this.bounds) {
            result += `猱?${this.bounds}`;
        }

        result += "\n";

        // Process children recursively
        for (const child of this.children) {
            result += child.toTreeString(`${indent}  `);
        }

        return result;
    }

    /**
     * Convert to string representation
     * @return {string} String representation
     */
    toString() {
        const parts = [];

        if (this.className) parts.push(`class="${this.className}"`);
        if (this.resourceId) parts.push(`id="${this.resourceId}"`);
        if (this.text) parts.push(`text="${this.text}"`);
        if (this.contentDesc) parts.push(`desc="${this.contentDesc}"`);
        if (this.isClickable) parts.push("clickable");

        return `<UINode ${parts.join(" ")}>`;
    }

    /**
     * Get a tree representation of this node and its descendants
     * @param {string} [indent=""] - Indentation string for formatting
     * @return {string} Tree representation
     */
    toTree(indent = "") {
        let result = `${indent}${this.toString()}\n`;

        for (const child of this.children) {
            result += child.toTree(`${indent}  `);
        }

        return result;
    }

    /**
     * Check if this node and another are the same (by comparing resource IDs)
     * @param {UINode} other - Node to compare with
     * @return {boolean} True if nodes are the same
     */
    equals(other) {
        if (!(other instanceof UINode)) return false;

        // If both nodes have resource IDs, compare them
        if (this.resourceId && other.resourceId) {
            return this.resourceId === other.resourceId;
        }

        // Otherwise compare bounds if available
        if (this.bounds && other.bounds) {
            return this.bounds === other.bounds;
        }

        // Last resort: compare text and class
        if (this.text && other.text && this.className && other.className) {
            return this.text === other.text && this.className === other.className;
        }

        // Can't determine equality
        return false;
    }

    // ===== Static Methods =====

    /**
     * Create a UINode from a page info result
     * @param {Object} pageInfo - Page info from UI.getPageInfo()
     * @return {UINode} UINode wrapping the page elements
     */
    static fromPageInfo(pageInfo) {
        const node = new UINode(pageInfo.uiElements);

        // Add formatted string method to mimic Kotlin's UIPageResultData toString()
        node.toFormattedString = function () {
            return `Current Application: ${pageInfo.packageName}
Current Activity: ${pageInfo.activityName}

UI Elements:
${this.toTreeString()}`;
        };

        return node;
    }

    /**
     * Get the current page UI
     * @return {Promise<UINode>} UINode representing the current page
     */
    static async getCurrentPage() {
        const pageInfo = await Tools.UI.getPageInfo();
        return UINode.fromPageInfo(pageInfo);
    }

    /**
     * Perform a search, wait, and return updated UI state
     * @param {Object} query - Search parameters
     * @param {number} [delayMs=1000] - Milliseconds to wait
     * @return {Promise<UINode>} UINode representing updated page
     */
    static async findAndWait(query, delayMs = 1000) {
        const result = await Tools.UI.combinedOperation(
            JSON.stringify({ action: "findElement", params: query }),
            delayMs
        );
        return UINode.fromPageInfo(result.pageInfo);
    }

    /**
     * Click an element, wait, and return updated UI state
     * @param {Object} query - Element to click (search parameters)
     * @param {number} [delayMs=1000] - Milliseconds to wait
     * @return {Promise<UINode>} UINode representing updated page
     */
    static async clickAndWait(query, delayMs = 1000) {
        const result = await Tools.UI.combinedOperation(
            JSON.stringify({ action: "clickElement", params: query }),
            delayMs
        );
        return UINode.fromPageInfo(result.pageInfo);
    }
}

__operitExpose('UINode', UINode);

