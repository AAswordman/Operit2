/**
 * AndroidUtils.js - Utilities for interacting with Android system through shell commands
 * 
 * This library provides a set of convenient wrappers for common Android operations
 * using shell commands under the hood. Classes are provided for:
 * - Content provider operations
 * - Package management
 * - System settings and properties
 * - Device control (screen, volume, etc.)
 * 
 * All functions ultimately use the Shell commands to execute their operations.
 * This library requires Shizuku service to be running with proper permissions.
 * 
 * Example usage:
 * ```javascript
 * // Get system property
 * const systemManager = new SystemManager();
 * const sdkVersion = await systemManager.getProperty("ro.build.version.sdk");
 * 
 * // Install an app
 * const packageManager = new PackageManager();
 * await packageManager.install("/sdcard/myapp.apk");
 * ```
 */

function executeShell(command, timeout = 15000) {
    return Tools.System.terminal.hiddenExec(command, {
        executorKey: 'android-utils',
        type: 'shell',
        timeoutMs: timeout
    }).then(function(result) {
        if (result && result.output !== undefined) {
            return result.output;
        }
        if (result && result.content !== undefined) {
            return result.content;
        }
        return '';
    });
}

function parseKeyValueOutput(output, separator = ': ') {
    const result = {};
    if (!output) return result;

    const lines = output.split('\n');
    for (const line of lines) {
        if (!line.trim()) continue;

        const separatorIndex = line.indexOf(separator);
        if (separatorIndex !== -1) {
            const key = line.substring(0, separatorIndex).trim();
            const value = line.substring(separatorIndex + separator.length).trim();
            result[key] = value;
        }
    }

    return result;
}

function escapeShellArg(str) {
    if (typeof str !== 'string') {
        str = String(str);
    }
    return "'" + str.replace(/'/g, "'\\''") + "'";
}
/**
 * Class for package management operations
 */
class PackageManager {
    /**
     * Create a new PackageManager
     */
    constructor() {
    }

    /**
     * Install an APK
     * @param {string} apkPath - Path to APK file
     * @param {boolean} replaceExisting - Replace existing app if present
     * @returns {Promise<string>} - Command output
     */
    async install(apkPath, replaceExisting = true) {
        const args = replaceExisting ? '-r' : '';
        return executeShell(`pm install ${args} ${apkPath}`, 60000); // Longer timeout for installs
    }

    /**
     * Uninstall an app
     * @param {string} packageName - Package name to uninstall
     * @param {boolean} keepData - Keep app data and cache
     * @returns {Promise<string>} - Command output
     */
    async uninstall(packageName, keepData = false) {
        const args = keepData ? '-k' : '';
        return executeShell(`pm uninstall ${args} ${packageName}`);
    }

    /**
     * Get information about a package
     * @param {string} packageName - Package name
     * @returns {Promise<Object>} - Package info object
     */
    async getInfo(packageName) {
        const output = await executeShell(`dumpsys package ${packageName}`);

        // Parse the output to extract useful information
        const result = {
            packageName: packageName,
            versionCode: undefined,
            versionName: undefined,
            firstInstallTime: undefined,
            lastUpdateTime: undefined,
            permissions: [],
            activities: [],
            services: []
        };

        // Extract version info
        const versionCodeMatch = output.match(/versionCode=(\d+)/);
        if (versionCodeMatch) {
            result.versionCode = parseInt(versionCodeMatch[1]);
        }

        const versionNameMatch = output.match(/versionName=([^\s]+)/);
        if (versionNameMatch) {
            result.versionName = versionNameMatch[1];
        }

        // Extract install times
        const firstInstallMatch = output.match(/firstInstallTime=([^\s]+)/);
        if (firstInstallMatch) {
            result.firstInstallTime = firstInstallMatch[1];
        }

        const lastUpdateMatch = output.match(/lastUpdateTime=([^\s]+)/);
        if (lastUpdateMatch) {
            result.lastUpdateTime = lastUpdateMatch[1];
        }

        // Extract activities - 改进的活动提取正则表达式
        const activityLines = output.split('\n');
        for (const line of activityLines) {
            // 寻找活动定义行，形如：12a2137 com.tencent.mobileqq/.activity.JumpActivity filter 9e4da28
            const activityMatch = line.match(/\s+([0-9a-f]+)\s+([\w.]+\/)?([\w.]+)\s+filter/);
            if (activityMatch) {
                const packagePart = activityMatch[2] ? activityMatch[2].replace('/', '') : packageName;
                const activityPart = activityMatch[3];
                const fullActivityName = activityPart.includes(packagePart) ? activityPart : `${packagePart}.${activityPart}`;

                if (!result.activities.includes(fullActivityName)) {
                    result.activities.push(fullActivityName);
                }
            }

            // 另一种格式：com.tencent.mobileqq.activity.SplashActivity
            const directActivityMatch = line.match(/\s+([\w.]+)\/([\w.]+)/);
            if (directActivityMatch && !activityMatch) {
                const activityName = directActivityMatch[2];
                if (!result.activities.includes(activityName) && activityName.toLowerCase().includes("activity")) {
                    result.activities.push(activityName);
                }
            }
        }

        // 尝试从 Activity Resolver Table 部分提取活动
        const activityTableMatch = output.match(/Activity Resolver Table:[\s\S]+?Non-Data Actions:/);
        if (activityTableMatch) {
            const activityTable = activityTableMatch[0];
            const activityMatches = activityTable.matchAll(/[\s\w.]+\/\.([\w.]+)/g);
            if (activityMatches) {
                for (const match of activityMatches) {
                    const activityName = match[1];
                    const fullActivityName = `${packageName}.${activityName}`;
                    if (activityName && !result.activities.includes(fullActivityName)) {
                        result.activities.push(fullActivityName);
                    }
                }
            }
        }

        // 查找主活动（通常与 LAUNCHER 类别关联）
        const launcherActivityMatch = output.match(/Category: "android\.intent\.category\.LAUNCHER"[\s\S]+?([a-zA-Z0-9_.]+)\/([a-zA-Z0-9_.]+)/);
        if (launcherActivityMatch && launcherActivityMatch[2]) {
            const launcherActivity = launcherActivityMatch[2].startsWith('.') ?
                `${packageName}${launcherActivityMatch[2]}` : launcherActivityMatch[2];

            // 将主活动放在列表的第一位
            if (!result.activities.includes(launcherActivity)) {
                result.activities.unshift(launcherActivity);
            } else {
                const index = result.activities.indexOf(launcherActivity);
                result.activities.splice(index, 1);
                result.activities.unshift(launcherActivity);
            }
        }

        // Extract services
        const serviceMatches = output.matchAll(/Service\s+{[^}]*?([a-zA-Z0-9_.]+)\//g);
        if (serviceMatches) {
            for (const match of serviceMatches) {
                if (match[1] && !result.services.includes(match[1])) {
                    result.services.push(match[1]);
                }
            }
        }

        return result;
    }

    /**
     * Get a list of installed packages
     * @param {boolean} includeSystem - Include system packages
     * @returns {Promise<Array<string>>} - List of package names
     */
    async getList(includeSystem = false) {
        const flag = includeSystem ? '-a' : '-3';
        const output = await executeShell(`pm list packages ${flag}`);

        const packages = [];
        const lines = output.split('\n');
        for (const line of lines) {
            if (line.startsWith('package:')) {
                packages.push(line.substring(8).trim());
            }
        }

        return packages;
    }

    /**
     * Clear app data
     * @param {string} packageName - Package name
     * @returns {Promise<string>} - Command output
     */
    async clearData(packageName) {
        return executeShell(`pm clear ${packageName}`);
    }

    /**
     * Check if a package is installed
     * @param {string} packageName - Package name to check
     * @returns {Promise<boolean>} - True if installed
     */
    async isInstalled(packageName) {
        try {
            const output = await executeShell(`pm list packages | grep ${packageName}`);
            return output.includes(`package:${packageName}`);
        } catch (error) {
            return false;
        }
    }
}

/**
 * Class for content provider operations
 */
class ContentProvider {
    /**
     * Create a new ContentProvider
     * @param {string} uri - Content URI
     */
    constructor(uri) {
        this.uri = uri;
    }

    /**
     * Set the URI for this content provider
     * @param {string} uri - Content URI
     * @returns {ContentProvider} - This content provider for chaining
     */
    setUri(uri) {
        this.uri = uri;
        return this;
    }

    /**
     * Query this content provider
     * @param {Array<string>} projection - Columns to return
     * @param {string} selection - WHERE clause
     * @param {Array<string>} selectionArgs - WHERE clause arguments
     * @param {string} sortOrder - ORDER BY clause
     * @returns {Promise<Array<Object>>} - Query results
     */
    async query(projection = undefined, selection = undefined, selectionArgs = undefined, sortOrder = undefined) {
        if (!this.uri) {
            throw new Error("URI not set. Call setUri() first.");
        }

        let command = ['content', 'query', '--uri', escapeShellArg(this.uri)];

        if (projection && projection.length) {
            command.push('--projection', escapeShellArg(projection.join(' ')));
        }

        if (selection) {
            command.push('--where', escapeShellArg(selection));
        }

        if (selectionArgs && selectionArgs.length) {
            command.push('--arg', escapeShellArg(selectionArgs.join(' ')));
        }

        if (sortOrder) {
            command.push('--sort', escapeShellArg(sortOrder));
        }

        const output = await executeShell(command.join(' '));

        // Parse output into objects
        const results = [];
        const lines = output.split('\n');

        if (lines.length >= 2) {
            // First line contains column names
            const columns = lines[0].split(' ').filter(col => col.trim().length > 0);

            // Remaining lines contain values
            for (let i = 1; i < lines.length; i++) {
                const line = lines[i].trim();
                if (!line) continue;

                const values = line.split(' ');
                const row = {};

                for (let j = 0; j < columns.length && j < values.length; j++) {
                    row[columns[j]] = values[j];
                }

                results.push(row);
            }
        }

        return results;
    }

    /**
     * Insert data into this content provider
     * @param {Object} values - Values to insert
     * @returns {Promise<string>} - Command output
     */
    async insert(values) {
        if (!this.uri) {
            throw new Error("URI not set. Call setUri() first.");
        }

        let command = ['content', 'insert', '--uri', escapeShellArg(this.uri)];

        // Add values
        for (const [key, value] of Object.entries(values)) {
            command.push('--bind', escapeShellArg(`${key}:s:${value}`));
        }

        return executeShell(command.join(' '));
    }

    /**
     * Update data in this content provider
     * @param {Object} values - Values to update
     * @param {string} selection - WHERE clause
     * @param {Array<string>} selectionArgs - WHERE clause arguments
     * @returns {Promise<string>} - Command output
     */
    async update(values, selection = undefined, selectionArgs = undefined) {
        if (!this.uri) {
            throw new Error("URI not set. Call setUri() first.");
        }

        let command = ['content', 'update', '--uri', escapeShellArg(this.uri)];

        // Add values
        for (const [key, value] of Object.entries(values)) {
            command.push('--bind', escapeShellArg(`${key}:s:${value}`));
        }

        if (selection) {
            command.push('--where', escapeShellArg(selection));
        }

        if (selectionArgs && selectionArgs.length) {
            for (const arg of selectionArgs) {
                command.push('--arg', escapeShellArg(arg));
            }
        }

        return executeShell(command.join(' '));
    }

    /**
     * Delete data from this content provider
     * @param {string} selection - WHERE clause
     * @param {Array<string>} selectionArgs - WHERE clause arguments
     * @returns {Promise<string>} - Command output
     */
    async delete(selection = undefined, selectionArgs = undefined) {
        if (!this.uri) {
            throw new Error("URI not set. Call setUri() first.");
        }

        let command = ['content', 'delete', '--uri', escapeShellArg(this.uri)];

        if (selection) {
            command.push('--where', escapeShellArg(selection));
        }

        if (selectionArgs && selectionArgs.length) {
            for (const arg of selectionArgs) {
                command.push('--arg', escapeShellArg(arg));
            }
        }

        return executeShell(command.join(' '));
    }
}

/**
 * Class for system properties and settings
 */
class SystemManager {
    /**
     * Create a new SystemManager
     */
    constructor() {
    }

    /**
     * Get a system property
     * @param {string} prop - Property name
     * @returns {Promise<string>} - Property value
     */
    async getProperty(prop) {
        const output = await executeShell(`getprop ${prop}`);
        return output.trim();
    }

    /**
     * Set a system property
     * @param {string} prop - Property name
     * @param {string} value - Property value
     * @returns {Promise<string>} - Command output
     */
    async setProperty(prop, value) {
        return executeShell(`setprop ${prop} ${escapeShellArg(value)}`);
    }

    /**
     * Get all system properties
     * @returns {Promise<Object>} - Properties as key-value pairs
     */
    async getAllProperties() {
        const output = await executeShell('getprop');

        // Parse [prop]: [value] format
        const properties = {};
        const propRegex = /\[([^\]]+)\]:\s*\[([^\]]*)\]/g;
        let match;

        while ((match = propRegex.exec(output)) !== undefined) {
            properties[match[1]] = match[2];
        }

        return properties;
    }

    /**
     * Get a system setting
     * @param {string} namespace - Settings namespace (system, secure, global)
     * @param {string} key - Setting key
     * @returns {Promise<string>} - Setting value
     */
    async getSetting(namespace, key) {
        // Validate namespace
        if (!['system', 'secure', 'global'].includes(namespace)) {
            throw new Error('Invalid namespace. Must be system, secure, or global.');
        }

        const output = await executeShell(`settings get ${namespace} ${key}`);
        return output.trim();
    }

    /**
     * Set a system setting
     * @param {string} namespace - Settings namespace (system, secure, global)
     * @param {string} key - Setting key
     * @param {string} value - Setting value
     * @returns {Promise<string>} - Command output
     */
    async setSetting(namespace, key, value) {
        // Validate namespace
        if (!['system', 'secure', 'global'].includes(namespace)) {
            throw new Error('Invalid namespace. Must be system, secure, or global.');
        }

        return executeShell(`settings put ${namespace} ${key} ${escapeShellArg(value)}`);
    }

    /**
     * List all settings in a namespace
     * @param {string} namespace - Settings namespace (system, secure, global)
     * @returns {Promise<Object>} - Settings as key-value pairs
     */
    async listSettings(namespace) {
        // Validate namespace
        if (!['system', 'secure', 'global'].includes(namespace)) {
            throw new Error('Invalid namespace. Must be system, secure, or global.');
        }

        const output = await executeShell(`settings list ${namespace}`);

        // Parse name=value format
        const settings = {};
        const lines = output.split('\n');
        for (const line of lines) {
            const parts = line.split('=');
            if (parts.length >= 2) {
                const key = parts[0].trim();
                const value = parts.slice(1).join('=').trim();
                settings[key] = value;
            }
        }

        return settings;
    }

    /**
     * Get device screen properties
     * @returns {Promise<Object>} - Screen properties
     */
    async getScreenInfo() {
        const output = await executeShell('wm size; wm density');

        const info = {
            width: undefined,
            height: undefined,
            density: undefined,
            densityDpi: undefined
        };

        // Parse physical size
        const sizeMatch = output.match(/Physical size: (\d+)x(\d+)/);
        if (sizeMatch) {
            info.width = parseInt(sizeMatch[1]);
            info.height = parseInt(sizeMatch[2]);
        }

        // Parse density
        const densityMatch = output.match(/Physical density: (\d+)/);
        if (densityMatch) {
            info.densityDpi = parseInt(densityMatch[1]);
            info.density = info.densityDpi / 160; // Convert DPI to density scale factor
        }

        return info;
    }
}

/**
 * Class for device control operations
 */
class DeviceController {
    /**
     * Create a new DeviceController
     */
    constructor() {
        this.systemManager = new SystemManager();
    }

    /**
     * Take a screenshot
     * @param {string} outputPath - Path to save screenshot
     * @returns {Promise<string>} - Command output
     */
    async takeScreenshot(outputPath) {
        return executeShell(`screencap -p ${outputPath}`);
    }

    /**
     * Record screen
     * @param {string} outputPath - Path to save recording
     * @param {number} timeLimit - Time limit in seconds (max 180)
     * @param {number} bitRate - Bit rate in Mbps
     * @param {string} size - Size in WIDTHxHEIGHT format
     * @returns {Promise<string>} - Command output
     */
    async recordScreen(outputPath, timeLimit = 180, bitRate = 4, size = undefined) {
        // Build command
        let command = `screenrecord --time-limit ${Math.min(timeLimit, 180)} --bit-rate ${bitRate}000000`;

        if (size) {
            command += ` --size ${size}`;
        }

        command += ` ${outputPath}`;

        return executeShell(command);
    }

    /**
     * Set screen brightness
     * @param {number} brightness - Brightness value (0-255)
     * @returns {Promise<string>} - Command output
     */
    async setBrightness(brightness) {
        // Ensure brightness is in valid range
        const value = Math.max(0, Math.min(255, Math.floor(brightness)));
        return this.systemManager.setSetting('system', 'screen_brightness', value.toString());
    }

    /**
     * Control device volume
     * @param {string} stream - Stream type (music, call, ring, alarm, notification)
     * @param {number} volume - Volume level
     * @returns {Promise<string>} - Command output
     */
    async setVolume(stream, volume) {
        const validStreams = {
            music: 3,
            call: 0,
            ring: 2,
            alarm: 4,
            notification: 5
        };

        if (!validStreams[stream]) {
            throw new Error('Invalid stream type. Must be music, call, ring, alarm, or notification.');
        }

        return executeShell(`media volume --stream ${validStreams[stream]} --set ${volume}`);
    }

    /**
     * Toggle WiFi
     * @param {boolean} enable - Enable or disable WiFi
     * @returns {Promise<string>} - Command output
     */
    async setWiFi(enable) {
        return executeShell(`svc wifi ${enable ? 'enable' : 'disable'}`);
    }

    /**
     * Toggle Bluetooth
     * @param {boolean} enable - Enable or disable Bluetooth
     * @returns {Promise<string>} - Command output
     */
    async setBluetooth(enable) {
        // Use service to enable/disable bluetooth
        return executeShell(`service call bluetooth_manager 6 i32 ${enable ? '1' : '0'}`);
    }

    /**
     * Lock the device
     * @returns {Promise<string>} - Command output
     */
    async lock() {
        return executeShell('input keyevent 26'); // KEYCODE_POWER
    }

    /**
     * Unlock the device (only works if no secure lock is set)
     * @returns {Promise<string>} - Command output
     */
    async unlock() {
        // Wake device if sleeping
        await executeShell('input keyevent 224'); // KEYCODE_WAKEUP

        // Dismiss keyguard
        return executeShell('input keyevent 82'); // KEYCODE_MENU
    }

    /**
     * Reboot the device
     * @param {string} mode - Reboot mode (undefined, recovery, bootloader)
     * @returns {Promise<string>} - Command output
     */
    async reboot(mode = undefined) {
        let command = 'reboot';
        if (mode) {
            command += ` ${mode}`;
        }
        return executeShell(command);
    }
}

/**
 * Main Android class that provides access to all Android functionality
 */
class Android {
    /**
     * Create a new Android interface
     */
    constructor() {
        this.packageManager = new PackageManager();
        this.systemManager = new SystemManager();
        this.deviceController = new DeviceController();
    }

    /**
     * Create a new ContentProvider
     * @param {string} uri - Content URI
     * @returns {ContentProvider} - New ContentProvider object
     */
    createContentProvider(uri) {
        return new ContentProvider(uri);
    }
}
/*
// Export the classes
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        Android,
        PackageManager,
        ContentProvider,
        SystemManager,
        DeviceController
    };
} 
    */

__operitExpose('Android', Android);
__operitExpose('PackageManager', PackageManager);
__operitExpose('ContentProvider', ContentProvider);
__operitExpose('SystemManager', SystemManager);
__operitExpose('DeviceController', DeviceController);


