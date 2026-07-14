
// Script for compiling build behavior. It is built in the build plug-in and cannot be modified currently.
import path from 'path'

declare const require: {
    (id: string): unknown
    main: { filename: string }
    resolve(id: string, options: { paths: string[] }): string
}

interface OhosHvigorPluginModule {
    hapTasks: unknown
}

/** Loads a package from the current hvigorw installation. */
function requireActiveHvigorPackage<T>(packageName: string): T {
    const modulePath = require.resolve(packageName, {
        paths: [path.dirname(require.main.filename)]
    })
    return require(modulePath) as T
}

const { hapTasks } = requireActiveHvigorPackage<OhosHvigorPluginModule>('@ohos/hvigor-ohos-plugin')

export default {
    system: hapTasks,  /* Built-in plugin of Hvigor. It cannot be modified. */
    plugins: []        /* Custom plugin to extend the functionality of Hvigor. */
}
