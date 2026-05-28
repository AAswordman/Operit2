/**
 * Android utilities type definitions for Assistance Package Tools
 */

/**
 * Base class for ADB command execution
 */
export class AdbExecutor {
    /**
     * Execute a raw ADB command
     * @param {string} command - ADB command to execute
     * @param {number} timeout - Optional timeout in milliseconds (default: 15000)
     * @returns {Promise<string>} - Command output
     */
    executeAdb(command: string, timeout?: number): Promise<string>;

    /**
     * Execute an ADB shell command
     * @param {string} command - Shell command to execute
     * @param {number} timeout - Optional timeout in milliseconds
     * @returns {Promise<string>} - Command output
     */
    executeShell(command: string, timeout?: number): Promise<string>;

    /**
     * Parse key-value output (usually from getprop, settings, etc.)
     * @param {string} output - Command output to parse
     * @param {string} separator - Separator between key and value (default: ': ')
     * @returns {Object} - Key-value object
     */
    parseKeyValueOutput(output: string, separator?: string): Record<string, string>;

    /**
     * Escape a string for shell command usage
     * @param {string} str - String to escape
     * @returns {string} - Escaped string
     */
    escapeShellArg(str: string | number | boolean): string;
}

/**
 * Intent flags enum for Android intents
 */
export const enum IntentFlag {
    // Activity launch flags
    ACTIVITY_NEW_TASK = 0x10000000,
    ACTIVITY_CLEAR_TOP = 0x04000000,
    ACTIVITY_SINGLE_TOP = 0x20000000,
    ACTIVITY_CLEAR_TASK = 0x00008000,
    ACTIVITY_NO_HISTORY = 0x40000000,
    ACTIVITY_BROUGHT_TO_FRONT = 0x00400000,
    ACTIVITY_RESET_TASK_IF_NEEDED = 0x00200000,
    ACTIVITY_LAUNCHED_FROM_HISTORY = 0x00100000,
    ACTIVITY_NO_ANIMATION = 0x00010000,
    ACTIVITY_REORDER_TO_FRONT = 0x00020000,
    ACTIVITY_NO_USER_ACTION = 0x00040000,
    ACTIVITY_RETAIN_IN_RECENTS = 0x00002000,
    ACTIVITY_TASK_ON_HOME = 0x00004000,
    ACTIVITY_MULTIPLE_TASK = 0x08000000,

    // URI permission flags
    GRANT_READ_URI_PERMISSION = 0x00000001,
    GRANT_WRITE_URI_PERMISSION = 0x00000002,
    GRANT_PERSISTABLE_URI_PERMISSION = 0x00000040,
    GRANT_PREFIX_URI_PERMISSION = 0x00000008,

    // Receiver flags
    RECEIVER_REGISTERED_ONLY = 0x40000000,
    RECEIVER_REPLACE_PENDING = 0x20000000,
    RECEIVER_FOREGROUND = 0x10000000,
    RECEIVER_NO_ABORT = 0x08000000,

    // Package visibility flags
    FLAG_EXCLUDE_STOPPED_PACKAGES = 0x00000010,
    FLAG_INCLUDE_STOPPED_PACKAGES = 0x00000020,

    // Other flags
    FLAG_DEBUG_LOG_RESOLUTION = 0x00000008,
    FLAG_FROM_BACKGROUND = 0x00000004,
    FLAG_ACTIVITY_FORWARD_RESULT = 0x02000000
}

/**
 * Intent actions enum for Android intents
 */
export const enum IntentAction {
    // Standard actions
    ACTION_MAIN = "android.intent.action.MAIN",
    ACTION_VIEW = "android.intent.action.VIEW",
    ACTION_SEND = "android.intent.action.SEND",
    ACTION_SENDTO = "android.intent.action.SENDTO",
    ACTION_DIAL = "android.intent.action.DIAL",
    ACTION_CALL = "android.intent.action.CALL",
    ACTION_SEARCH = "android.intent.action.SEARCH",
    ACTION_WEB_SEARCH = "android.intent.action.WEB_SEARCH",
    ACTION_PICK = "android.intent.action.PICK",
    ACTION_GET_CONTENT = "android.intent.action.GET_CONTENT",
    ACTION_EDIT = "android.intent.action.EDIT",
    ACTION_INSERT = "android.intent.action.INSERT",
    ACTION_DELETE = "android.intent.action.DELETE",
    ACTION_RUN = "android.intent.action.RUN",
    ACTION_SYNC = "android.intent.action.SYNC",
    ACTION_PICK_ACTIVITY = "android.intent.action.PICK_ACTIVITY",
    ACTION_ATTACH_DATA = "android.intent.action.ATTACH_DATA",

    // System actions
    ACTION_POWER_USAGE_SUMMARY = "android.intent.action.POWER_USAGE_SUMMARY",
    ACTION_DATE_SETTINGS = "android.intent.action.DATE_SETTINGS",
    ACTION_LOCALE_SETTINGS = "android.intent.action.LOCALE_SETTINGS",
    ACTION_VOICE_CONTROL_SETTINGS = "android.intent.action.VOICE_CONTROL_SETTINGS",
    ACTION_WIRELESS_SETTINGS = "android.intent.action.WIRELESS_SETTINGS",
    ACTION_SETTINGS = "android.intent.action.SETTINGS",
    ACTION_DEVICE_INFO_SETTINGS = "android.intent.action.DEVICE_INFO_SETTINGS",
    ACTION_MANAGE_APPLICATIONS_SETTINGS = "android.intent.action.MANAGE_APPLICATIONS_SETTINGS",
    ACTION_MANAGE_ALL_APPLICATIONS_SETTINGS = "android.intent.action.MANAGE_ALL_APPLICATIONS_SETTINGS",

    // Media actions
    ACTION_MEDIA_PLAY = "android.intent.action.MEDIA_PLAY",
    ACTION_MEDIA_PAUSE = "android.intent.action.MEDIA_PAUSE",
    ACTION_MEDIA_STOP = "android.intent.action.MEDIA_STOP",
    ACTION_MEDIA_NEXT = "android.intent.action.MEDIA_NEXT",
    ACTION_MEDIA_PREVIOUS = "android.intent.action.MEDIA_PREVIOUS",
    ACTION_MUSIC_PLAYER = "android.intent.action.MUSIC_PLAYER",
    ACTION_CAMERA_BUTTON = "android.intent.action.CAMERA_BUTTON",
    ACTION_IMAGE_CAPTURE = "android.media.action.IMAGE_CAPTURE",
    ACTION_VIDEO_CAPTURE = "android.media.action.VIDEO_CAPTURE",

    // Communication actions
    ACTION_ANSWER = "android.intent.action.ANSWER",
    ACTION_CALL_BUTTON = "android.intent.action.CALL_BUTTON",
    ACTION_CALL_EMERGENCY = "android.intent.action.CALL_EMERGENCY",
    ACTION_QUICK_CLOCK = "android.intent.action.QUICK_CLOCK",
    ACTION_SEND_MULTIPLE = "android.intent.action.SEND_MULTIPLE",

    // Broadcast actions
    ACTION_BOOT_COMPLETED = "android.intent.action.BOOT_COMPLETED",
    ACTION_PACKAGE_ADDED = "android.intent.action.PACKAGE_ADDED",
    ACTION_PACKAGE_REMOVED = "android.intent.action.PACKAGE_REMOVED",
    ACTION_PACKAGE_REPLACED = "android.intent.action.PACKAGE_REPLACED",
    ACTION_BATTERY_LOW = "android.intent.action.BATTERY_LOW",
    ACTION_BATTERY_OKAY = "android.intent.action.BATTERY_OKAY",
    ACTION_POWER_CONNECTED = "android.intent.action.ACTION_POWER_CONNECTED",
    ACTION_POWER_DISCONNECTED = "android.intent.action.ACTION_POWER_DISCONNECTED",
    ACTION_SHUTDOWN = "android.intent.action.ACTION_SHUTDOWN",
    ACTION_AIRPLANE_MODE_CHANGED = "android.intent.action.AIRPLANE_MODE",
    ACTION_SCREEN_ON = "android.intent.action.SCREEN_ON",
    ACTION_SCREEN_OFF = "android.intent.action.SCREEN_OFF",
    ACTION_USER_PRESENT = "android.intent.action.USER_PRESENT"
}

/**
 * Intent categories enum for Android intents
 */
export const enum IntentCategory {
    // Standard categories
    CATEGORY_DEFAULT = "android.intent.category.DEFAULT",
    CATEGORY_BROWSABLE = "android.intent.category.BROWSABLE",
    CATEGORY_LAUNCHER = "android.intent.category.LAUNCHER",
    CATEGORY_HOME = "android.intent.category.HOME",
    CATEGORY_PREFERENCE = "android.intent.category.PREFERENCE",
    CATEGORY_APP_BROWSER = "android.intent.category.APP_BROWSER",
    CATEGORY_APP_CALCULATOR = "android.intent.category.APP_CALCULATOR",
    CATEGORY_APP_CALENDAR = "android.intent.category.APP_CALENDAR",
    CATEGORY_APP_CONTACTS = "android.intent.category.APP_CONTACTS",
    CATEGORY_APP_EMAIL = "android.intent.category.APP_EMAIL",
    CATEGORY_APP_GALLERY = "android.intent.category.APP_GALLERY",
    CATEGORY_APP_MAPS = "android.intent.category.APP_MAPS",
    CATEGORY_APP_MARKET = "android.intent.category.APP_MARKET",
    CATEGORY_APP_MESSAGING = "android.intent.category.APP_MESSAGING",
    CATEGORY_APP_MUSIC = "android.intent.category.APP_MUSIC",
    CATEGORY_INFO = "android.intent.category.INFO",
    CATEGORY_ALTERNATIVE = "android.intent.category.ALTERNATIVE",
    CATEGORY_SELECTED_ALTERNATIVE = "android.intent.category.SELECTED_ALTERNATIVE",
    CATEGORY_TAB = "android.intent.category.TAB",
    CATEGORY_OPENABLE = "android.intent.category.OPENABLE",
    CATEGORY_VOICE = "android.intent.category.VOICE",
    CATEGORY_CAR_DOCK = "android.intent.category.CAR_DOCK",
    CATEGORY_DESK_DOCK = "android.intent.category.DESK_DOCK",
    CATEGORY_LE_DESK_DOCK = "android.intent.category.LE_DESK_DOCK",
    CATEGORY_HE_DESK_DOCK = "android.intent.category.HE_DESK_DOCK",
    CATEGORY_CAR_MODE = "android.intent.category.CAR_MODE"
}

/**
 * Class representing an Android intent
 */
export class Intent {
    /**
     * Create a new Intent
     * @param {string} action - Optional action to set
     */
    constructor(action?: string | IntentAction | undefined);

    /**
     * The action for this intent
     */
    action: string | undefined;

    /**
     * The package name for this intent
     */
    packageName: string | undefined;

    /**
     * The component for this intent
     */
    component: string | undefined;

    /**
     * The extras for this intent
     */
    extras: Record<string, any>;

    /**
     * The flags for this intent
     */
    flags: string[];

    /**
     * The categories for this intent
     */
    categories: string[];

    /**
     * The ADB executor for this intent
     */
    executor: AdbExecutor;

    /**
     * The data URI for this intent
     */
    uri: string | undefined;

    /**
     * The MIME type for this intent
     */
    type: string | undefined;

    /**
     * Set the component for this intent
     * @param {string} packageName - Package name
     * @param {string} component - Component name
     * @returns {Intent} - This intent for chaining
     */
    setComponent(packageName: string, component: string): Intent;

    /**
     * Set just the package name without specifying component
     * @param {string} packageName - Package name
     * @returns {Intent} - This intent for chaining
     */
    setPackage(packageName: string): Intent;

    /**
     * Set the action for this intent
     * @param {string} action - Intent action
     * @returns {Intent} - This intent for chaining
     */
    setAction(action: string | IntentAction): Intent;

    /**
     * Set the data URI for this intent
     * @param {string} uri - Data URI
     * @returns {Intent} - This intent for chaining
     */
    setData(uri: string): Intent;

    /**
     * Set the MIME type for this intent
     * @param {string} type - MIME type
     * @returns {Intent} - This intent for chaining
     */
    setType(type: string): Intent;

    /**
     * Add a category to this intent
     * @param {string} category - Intent category
     * @returns {Intent} - This intent for chaining
     */
    addCategory(category: string | IntentCategory): Intent;

    /**
     * Remove a category from this intent
     * @param {string} category - Intent category to remove
     * @returns {Intent} - This intent for chaining
     */
    removeCategory(category: string | IntentCategory): Intent;

    /**
     * Check if intent has a specific category
     * @param {string} category - Intent category to check
     * @returns {boolean} - True if the intent has the category
     */
    hasCategory(category: string | IntentCategory): boolean;

    /**
     * Get all categories
     * @returns {Array<string>} - Array of categories
     */
    getCategories(): string[];

    /**
     * Clear all categories
     * @returns {Intent} - This intent for chaining
     */
    clearCategories(): Intent;

    /**
     * Add a flag to this intent
     * @param {IntentFlag} flag - Intent flag from IntentFlag enum
     * @returns {Intent} - This intent for chaining
     */
    addFlag(flag: IntentFlag | string): Intent;

    /**
     * Put an extra value in this intent
     * @param {string} key - Extra key
     * @param {any} value - Extra value
     * @returns {Intent} - This intent for chaining
     */
    putExtra(key: string, value: any): Intent;

    /**
     * Start this intent as an activity
     * @returns {Promise<Object>} - Intent result object
     */
    start(): Promise<any>;

    /**
     * Send this intent as a broadcast
     * @returns {Promise<Object>} - Intent result object
     */
    sendBroadcast(): Promise<any>;

    /**
     * Start this intent as a service
     * @returns {Promise<Object>} - Intent result object
     */
    startService(): Promise<any>;
}

/**
 * Class for package management operations
 */
export class PackageManager extends AdbExecutor {
    /**
     * Create a new PackageManager
     */
    constructor();

    /**
     * Install an APK
     * @param {string} apkPath - Path to APK file
     * @param {boolean} replaceExisting - Replace existing app if present
     * @returns {Promise<string>} - Command output
     */
    install(apkPath: string, replaceExisting?: boolean): Promise<string>;

    /**
     * Uninstall an app
     * @param {string} packageName - Package name to uninstall
     * @param {boolean} keepData - Keep app data and cache
     * @returns {Promise<string>} - Command output
     */
    uninstall(packageName: string, keepData?: boolean): Promise<string>;

    /**
     * Get information about a package
     * @param {string} packageName - Package name
     * @returns {Promise<Object>} - Package info object
     */
    getInfo(packageName: string): Promise<{
        packageName: string;
        versionCode: number | undefined;
        versionName: string | undefined;
        firstInstallTime: string | undefined;
        lastUpdateTime: string | undefined;
        permissions: string[];
        activities: string[];
        services: string[];
    }>;

    /**
     * Get a list of installed packages
     * @param {boolean} includeSystem - Include system packages
     * @returns {Promise<Array<string>>} - List of package names
     */
    getList(includeSystem?: boolean): Promise<string[]>;

    /**
     * Clear app data
     * @param {string} packageName - Package name
     * @returns {Promise<string>} - Command output
     */
    clearData(packageName: string): Promise<string>;

    /**
     * Check if a package is installed
     * @param {string} packageName - Package name to check
     * @returns {Promise<boolean>} - True if installed
     */
    isInstalled(packageName: string): Promise<boolean>;
}

/**
 * Class for content provider operations
 */
export class ContentProvider extends AdbExecutor {
    /**
     * Create a new ContentProvider
     * @param {string} uri - Content URI
     */
    constructor(uri: string);

    /**
     * The URI for this content provider
     */
    uri: string;

    /**
     * Set the URI for this content provider
     * @param {string} uri - Content URI
     * @returns {ContentProvider} - This content provider for chaining
     */
    setUri(uri: string): ContentProvider;

    /**
     * Query this content provider
     * @param {Array<string>} projection - Columns to return
     * @param {string} selection - WHERE clause
     * @param {Array<string>} selectionArgs - WHERE clause arguments
     * @param {string} sortOrder - ORDER BY clause
     * @returns {Promise<Array<Object>>} - Query results
     */
    query(projection?: string[] | undefined, selection?: string | undefined, selectionArgs?: string[] | undefined, sortOrder?: string | undefined): Promise<Record<string, string>[]>;

    /**
     * Insert data into this content provider
     * @param {Object} values - Values to insert
     * @returns {Promise<string>} - Command output
     */
    insert(values: Record<string, string>): Promise<string>;

    /**
     * Update data in this content provider
     * @param {Object} values - Values to update
     * @param {string} selection - WHERE clause
     * @param {Array<string>} selectionArgs - WHERE clause arguments
     * @returns {Promise<string>} - Command output
     */
    update(values: Record<string, string>, selection?: string | undefined, selectionArgs?: string[] | undefined): Promise<string>;

    /**
     * Delete data from this content provider
     * @param {string} selection - WHERE clause
     * @param {Array<string>} selectionArgs - WHERE clause arguments
     * @returns {Promise<string>} - Command output
     */
    delete(selection?: string | undefined, selectionArgs?: string[] | undefined): Promise<string>;
}

/**
 * Class for system properties and settings
 */
export class SystemManager extends AdbExecutor {
    /**
     * Create a new SystemManager
     */
    constructor();

    /**
     * Get a system property
     * @param {string} prop - Property name
     * @returns {Promise<string>} - Property value
     */
    getProperty(prop: string): Promise<string>;

    /**
     * Set a system property
     * @param {string} prop - Property name
     * @param {string} value - Property value
     * @returns {Promise<string>} - Command output
     */
    setProperty(prop: string, value: string): Promise<string>;

    /**
     * Get all system properties
     * @returns {Promise<Object>} - Properties as key-value pairs
     */
    getAllProperties(): Promise<Record<string, string>>;

    /**
     * Get a system setting
     * @param {string} namespace - Settings namespace (system, secure, global)
     * @param {string} key - Setting key
     * @returns {Promise<string>} - Setting value
     */
    getSetting(namespace: 'system' | 'secure' | 'global', key: string): Promise<string>;

    /**
     * Set a system setting
     * @param {string} namespace - Settings namespace (system, secure, global)
     * @param {string} key - Setting key
     * @param {string} value - Setting value
     * @returns {Promise<string>} - Command output
     */
    setSetting(namespace: 'system' | 'secure' | 'global', key: string, value: string): Promise<string>;

    /**
     * List all settings in a namespace
     * @param {string} namespace - Settings namespace (system, secure, global)
     * @returns {Promise<Object>} - Settings as key-value pairs
     */
    listSettings(namespace: 'system' | 'secure' | 'global'): Promise<Record<string, string>>;

    /**
     * Get device screen properties
     * @returns {Promise<Object>} - Screen properties
     */
    getScreenInfo(): Promise<{
        width: number | undefined;
        height: number | undefined;
        density: number | undefined;
        densityDpi: number | undefined;
    }>;
}

/**
 * Class for device control operations
 */
export class DeviceController extends AdbExecutor {
    /**
     * Create a new DeviceController
     */
    constructor();

    /**
     * The system manager for this device controller
     */
    systemManager: SystemManager;

    /**
     * Take a screenshot
     * @param {string} outputPath - Path to save screenshot
     * @returns {Promise<string>} - Command output
     */
    takeScreenshot(outputPath: string): Promise<string>;

    /**
     * Record screen
     * @param {string} outputPath - Path to save recording
     * @param {number} timeLimit - Time limit in seconds (max 180)
     * @param {number} bitRate - Bit rate in Mbps
     * @param {string} size - Size in WIDTHxHEIGHT format
     * @returns {Promise<string>} - Command output
     */
    recordScreen(outputPath: string, timeLimit?: number, bitRate?: number, size?: string | undefined): Promise<string>;

    /**
     * Set screen brightness
     * @param {number} brightness - Brightness value (0-255)
     * @returns {Promise<string>} - Command output
     */
    setBrightness(brightness: number): Promise<string>;

    /**
     * Control device volume
     * @param {string} stream - Stream type (music, call, ring, alarm, notification)
     * @param {number} volume - Volume level
     * @returns {Promise<string>} - Command output
     */
    setVolume(stream: 'music' | 'call' | 'ring' | 'alarm' | 'notification', volume: number): Promise<string>;

    /**
     * Toggle airplane mode
     * @param {boolean} enable - Enable or disable airplane mode
     * @returns {Promise<string>} - Command output
     */
    setAirplaneMode(enable: boolean): Promise<string>;

    /**
     * Toggle WiFi
     * @param {boolean} enable - Enable or disable WiFi
     * @returns {Promise<string>} - Command output
     */
    setWiFi(enable: boolean): Promise<string>;

    /**
     * Toggle Bluetooth
     * @param {boolean} enable - Enable or disable Bluetooth
     * @returns {Promise<string>} - Command output
     */
    setBluetooth(enable: boolean): Promise<string>;

    /**
     * Lock the device
     * @returns {Promise<string>} - Command output
     */
    lock(): Promise<string>;

    /**
     * Unlock the device (only works if no secure lock is set)
     * @returns {Promise<string>} - Command output
     */
    unlock(): Promise<string>;

    /**
     * Reboot the device
     * @param {string} mode - Reboot mode (undefined, recovery, bootloader)
     * @returns {Promise<string>} - Command output
     */
    reboot(mode?: string | undefined): Promise<string>;
}

/**
 * Main Android class that provides access to all Android functionality
 */
export class Android {
    /**
     * Create a new Android interface
     */
    constructor();

    /**
     * The package manager for this Android interface
     */
    packageManager: PackageManager;

    /**
     * The system manager for this Android interface
     */
    systemManager: SystemManager;

    /**
     * The device controller for this Android interface
     */
    deviceController: DeviceController;

    /**
     * Create a new Intent
     * @param {string} action - Optional action to set
     * @returns {Intent} - New Intent object
     */
    createIntent(action?: string | undefined): Intent;

    /**
     * Create a new ContentProvider
     * @param {string} uri - Content URI
     * @returns {ContentProvider} - New ContentProvider object
     */
    createContentProvider(uri: string): ContentProvider;
} 