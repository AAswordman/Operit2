import path from 'path'

declare const require: {
    (id: string): unknown
    main: { filename: string }
    resolve(id: string, options: { paths: string[] }): string
}

interface OhosHvigorPluginModule {
    appTasks: unknown
}

interface FlutterHvigorPluginModule {
    flutterHvigorPlugin(flutterProjectPath: string): unknown
}

/** Loads a package from the current hvigorw installation. */
function requireActiveHvigorPackage<T>(packageName: string): T {
    const modulePath = require.resolve(packageName, {
        paths: [path.dirname(require.main.filename)]
    })
    return require(modulePath) as T
}

/** Loads a package declared by this OHOS project. */
function requireOhosProjectPackage<T>(packageName: string): T {
    const modulePath = require.resolve(packageName, {
        paths: [__dirname]
    })
    return require(modulePath) as T
}

const { appTasks } = requireActiveHvigorPackage<OhosHvigorPluginModule>('@ohos/hvigor-ohos-plugin')
const { flutterHvigorPlugin } = requireOhosProjectPackage<FlutterHvigorPluginModule>('flutter-hvigor-plugin')

export default {
    system: appTasks,  /* Built-in plugin of Hvigor. It cannot be modified. */
    plugins:[flutterHvigorPlugin(path.dirname(__dirname))]         /* Custom plugin to extend the functionality of Hvigor. */
}
