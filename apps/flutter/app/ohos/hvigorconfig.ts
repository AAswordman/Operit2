import fs from 'fs'
import path from 'path'

declare const require: {
  (id: string): unknown
  main: { filename: string }
  resolve(id: string, options: { paths: string[] }): string
}

interface HvigorConfig {
  includeNode(name: string, srcPath: string, extraOptions?: Record<string, unknown>): void
}

interface ActiveHvigorModule {
  hvigor: {
    getHvigorConfig(): HvigorConfig
  }
}

interface FlutterPluginDependency {
  name: string
  path: string
  native_build?: boolean
}

interface FlutterPluginsDependencies {
  plugins: {
    ohos: FlutterPluginDependency[]
  }
}

const STAGED_OHOS_PLUGINS_DIR = '.flutter_ohos_plugins'

/** Returns a POSIX-style path for Hvigor module descriptors. */
function toHvigorPath(value: string): string {
  return value.split(path.sep).join(path.posix.sep)
}

/** Copies one native OHOS plugin into this project for Hvigor module inclusion. */
function stageNativeOhosPlugin(nativeProjectPath: string, plugin: FlutterPluginDependency): string {
  const sourcePath = path.join(plugin.path, 'ohos')
  const stagedPath = path.join(nativeProjectPath, STAGED_OHOS_PLUGINS_DIR, plugin.name)
  fs.rmSync(stagedPath, { recursive: true, force: true })
  fs.mkdirSync(path.dirname(stagedPath), { recursive: true })
  fs.cpSync(sourcePath, stagedPath, { recursive: true })
  patchStagedPluginHvigorfile(stagedPath)
  patchStagedPluginPackage(stagedPath)
  return `./${toHvigorPath(path.relative(nativeProjectPath, stagedPath))}`
}

/** Rewrites a staged plugin hvigorfile to resolve OHOS plugin tasks from the active hvigorw installation. */
function patchStagedPluginHvigorfile(stagedPath: string): void {
  const hvigorfilePath = path.join(stagedPath, 'hvigorfile.ts')
  const content = fs.readFileSync(hvigorfilePath, 'utf-8')
  const importTemplate = "import { harTasks } from '@ohos/hvigor-ohos-plugin';"
  const exportTemplate = "export { harTasks } from '@ohos/hvigor-ohos-plugin';"
  const replacement = `import path from 'path'

declare const require: {
    (id: string): unknown
    main: { filename: string }
    resolve(id: string, options: { paths: string[] }): string
}

interface OhosHvigorPluginModule {
    harTasks: unknown
}

/** Loads a package from the current hvigorw installation. */
function requireActiveHvigorPackage<T>(packageName: string): T {
    const modulePath = require.resolve(packageName, {
        paths: [path.dirname(require.main.filename)]
    })
    return require(modulePath) as T
}

const { harTasks } = requireActiveHvigorPackage<OhosHvigorPluginModule>('@ohos/hvigor-ohos-plugin')`
  if (content.includes(importTemplate)) {
    fs.writeFileSync(hvigorfilePath, content.replace(importTemplate, replacement))
    return
  }
  if (content.includes(exportTemplate)) {
    fs.writeFileSync(hvigorfilePath, content.replace(exportTemplate, `${replacement}\n\nexport { harTasks };`))
    return
  }
  throw new Error(`Unsupported staged OHOS plugin hvigorfile: ${hvigorfilePath}`)
}

/** Rewrites a staged plugin package to consume the app-level Flutter HAR override. */
function patchStagedPluginPackage(stagedPath: string): void {
  const packagePath = path.join(stagedPath, 'oh-package.json5')
  const content = fs.readFileSync(packagePath, 'utf-8')
  const updated = content.replace('"@ohos/flutter_ohos": "file:libs/flutter.har"', '"@ohos/flutter_ohos": ""')
  fs.writeFileSync(packagePath, updated)
}

/** Reads Flutter's generated plugin dependency graph. */
function readFlutterPluginsDependencies(flutterProjectPath: string): FlutterPluginsDependencies {
  const dependenciesPath = path.join(flutterProjectPath, '.flutter-plugins-dependencies')
  const fileContent = fs.readFileSync(dependenciesPath, 'utf-8')
  return JSON.parse(fileContent) as FlutterPluginsDependencies
}

/** Adds native OHOS Flutter plugins to Hvigor's project graph. */
function injectNativeOhosModules(nativeProjectPath: string, flutterProjectPath: string): void {
  const activeHvigor = resolveActiveHvigor()
  const hvigorConfig = activeHvigor.getHvigorConfig()
  const dependencies = readFlutterPluginsDependencies(flutterProjectPath)
  const nativePlugins = dependencies.plugins.ohos.filter(plugin => plugin.native_build !== false)

  nativePlugins.forEach(plugin => {
    hvigorConfig.includeNode(plugin.name, stageNativeOhosPlugin(nativeProjectPath, plugin))
  })
}

/** Resolves the Hvigor module owned by the current hvigorw process. */
function resolveActiveHvigor(): ActiveHvigorModule['hvigor'] {
  const hvigorModulePath = require.resolve('@ohos/hvigor', {
    paths: [path.dirname(require.main.filename)]
  })
  const hvigorModule = require(hvigorModulePath) as ActiveHvigorModule
  return hvigorModule.hvigor
}

injectNativeOhosModules(__dirname, path.dirname(__dirname))
