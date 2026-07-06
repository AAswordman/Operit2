/**
 * Android utilities type definitions for Assistance Package Tools
 */

/**
 * Class for package management operations
 */
export class PackageManager {
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
export class ContentProvider {
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
export class SystemManager {
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
export class DeviceController {
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
     * Create a new ContentProvider
     * @param {string} uri - Content URI
     * @returns {ContentProvider} - New ContentProvider object
     */
    createContentProvider(uri: string): ContentProvider;
} 
