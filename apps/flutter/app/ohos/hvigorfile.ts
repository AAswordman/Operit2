import path from 'path'

declare const require: {
    (id: string): unknown
    main: { filename: string }
    resolve(id: string, options: { paths: string[] }): string
}

interface OhosHvigorPluginModule {
    appTasks: unknown
    OhosPluginId: { OHOS_APP_PLUGIN: string; OHOS_HAP_PLUGIN: string }
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

const { appTasks, OhosPluginId } = requireActiveHvigorPackage<OhosHvigorPluginModule>('@ohos/hvigor-ohos-plugin')
const { flutterHvigorPlugin } = requireOhosProjectPackage<FlutterHvigorPluginModule>('flutter-hvigor-plugin')

interface HvigorNode {
    getNodeName(): string
    getContext(pluginId: string): unknown
    afterNodeEvaluate(callback: (node: HvigorNode) => void): void
    subNodes(callback: (node: HvigorNode) => void): void
}

interface HvigorModule {
    hvigor: { getParameter(): { getExtParam(name: string): string | undefined } }
}

interface AppContext {
    getOverrides(): Record<string, string>
    setOverrides(overrides: Record<string, string>): void
}

interface NativeBuildProfile {
    buildOption: {
        externalNativeOptions: { abiFilters?: string[] }
        nativeLib: { filter: { excludes: string[] } }
    }
}

interface HapContext {
    getBuildProfileOpt(): NativeBuildProfile
    setBuildProfileOpt(profile: NativeBuildProfile): void
}

const { hvigor } = requireActiveHvigorPackage<HvigorModule>('@ohos/hvigor')
const targetAbis = new Map([
    ['ohos-arm64', 'arm64-v8a'],
    ['ohos-x64', 'x86_64'],
])

/** Applies the selected Flutter platform to native compilation and dependency resolution. */
function operitArchitecturePlugin() {
    return {
        pluginId: 'operit-ohos-architecture',
        /** Configures one build without rewriting project manifests. */
        apply(rootNode: HvigorNode): void {
            const platform = hvigor.getParameter().getExtParam('TARGET_PLATFORM')
            const abi = targetAbis.get(platform as string)
            if (abi === undefined) {
                throw new Error('Select exactly one TARGET_PLATFORM: ohos-arm64 or ohos-x64')
            }
            const nativePackage = `flutter_native_${abi.replace('-', '_')}`
            const nativeHar = `${abi.replace('-', '_')}_release.har`
            const embedding = 'file:' + path.join(__dirname, 'har', 'flutter_embedding_release.har')
            const native = 'file:' + path.join(__dirname, 'har', nativeHar)
            rootNode.afterNodeEvaluate(node => {
                const context = node.getContext(OhosPluginId.OHOS_APP_PLUGIN) as AppContext
                const overrides = context.getOverrides()
                overrides['@ohos/flutter_ohos'] = embedding
                overrides[nativePackage] = native
                context.setOverrides(overrides)
            })
            rootNode.subNodes(subNode => {
                if (subNode.getNodeName() !== 'entry') {
                    return
                }
                subNode.afterNodeEvaluate(node => {
                    const context = node.getContext(OhosPluginId.OHOS_HAP_PLUGIN) as HapContext
                    const profile = context.getBuildProfileOpt()
                    profile.buildOption.externalNativeOptions.abiFilters = [abi]
                    profile.buildOption.nativeLib.filter.excludes.push(
                        ...['arm64-v8a', 'armeabi-v7a', 'x86_64']
                            .filter(candidate => candidate !== abi)
                            .map(candidate => `**/${candidate}/**`)
                    )
                    context.setBuildProfileOpt(profile)
                })
            })
        }
    }
}

export default {
    system: appTasks,  /* Built-in plugin of Hvigor. It cannot be modified. */
    plugins: [flutterHvigorPlugin(path.dirname(__dirname)), operitArchitecturePlugin()]
}
