#!/usr/bin/env ruby
# Builds the production CocoaPods WebView and runs its geometry regression tests
# in a small UIKit host, without compiling the application's Rust/runtime assets.
# Usage: ruby tools/tests/run_ios_webview_geometry_tests.rb [simulator-udid]
require 'xcodeproj'
require 'json'
require 'tmpdir'
require 'fileutils'

repo = File.expand_path('../..', __dir__)
output = Dir.mktmpdir('operit-webview-geometry-')
puts "WebView test artifacts: #{output}"

def run_logged(log, *command)
  puts command.join(' ')
  return if system(*command, out: log, err: [:child, :out])
  warn File.readlines(log).last(50).join
  abort "Command failed; full log: #{log}"
end

flutter_command = ENV.fetch('PATH').split(File::PATH_SEPARATOR)
  .map { |path| File.join(path, 'flutter') }.find { |path| File.executable?(path) }
abort 'flutter must be on PATH, or FLUTTER_ROOT must be set' unless flutter_command || ENV['FLUTTER_ROOT']
flutter_root = ENV['FLUTTER_ROOT'] || File.expand_path('..', File.dirname(File.realpath(flutter_command)))
flutter = File.join(flutter_root, 'bin/cache/artifacts/engine/ios/Flutter.xcframework/ios-arm64_x86_64-simulator/Flutter.framework')
abort "Simulator Flutter.framework not found: #{flutter}" unless File.directory?(flutter)

udid = ARGV.first
unless udid
  devices = JSON.parse(IO.popen(['xcrun', 'simctl', 'list', 'devices', 'available', '--json'], &:read))
  candidates = devices.fetch('devices').select { |runtime, _| runtime.include?('.iOS-') }.values.flatten
  udid = (candidates.find { |device| device['state'] == 'Booted' } || candidates.first)&.fetch('udid')
end
abort 'An available iOS simulator is required' unless udid

ios = File.join(repo, 'apps/flutter/app/ios')
run_logged(File.join(output, 'pod-install.log'), 'pod', 'install', "--project-directory=#{ios}")
pod_build = File.join(output, 'pod-build')
run_logged(File.join(output, 'pod-build.log'), 'xcodebuild', '-project', File.join(ios, 'Pods/Pods.xcodeproj'),
           '-target', 'webview_flutter_wkwebview', '-configuration', 'Debug', '-sdk', 'iphonesimulator',
           '-arch', 'arm64', "CONFIGURATION_BUILD_DIR=#{pod_build}", 'CODE_SIGNING_ALLOWED=NO', 'build')

File.write(File.join(output, 'Host.m'), <<~OBJC)
  #import <UIKit/UIKit.h>
  @interface GeometryTestScene : UIResponder <UIWindowSceneDelegate>
  @property(strong, nonatomic) UIWindow *window;
  @end
  @implementation GeometryTestScene
  - (void)scene:(UIScene *)scene willConnectToSession:(UISceneSession *)session
        options:(UISceneConnectionOptions *)options {
    self.window = [[UIWindow alloc] initWithWindowScene:(UIWindowScene *)scene];
    self.window.rootViewController = [[UIViewController alloc] init];
    [self.window makeKeyAndVisible];
  }
  @end
  @interface GeometryTestApp : UIResponder <UIApplicationDelegate>
  @end
  @implementation GeometryTestApp
  - (UISceneConfiguration *)application:(UIApplication *)app
        configurationForConnectingSceneSession:(UISceneSession *)session
        options:(UISceneConnectionOptions *)options {
    UISceneConfiguration *config = [[UISceneConfiguration alloc] initWithName:@"Tests" sessionRole:session.role];
    config.delegateClass = GeometryTestScene.class;
    return config;
  }
  @end
  int main(int argc, char **argv) {
    @autoreleasepool { return UIApplicationMain(argc, argv, nil, NSStringFromClass(GeometryTestApp.class)); }
  }
OBJC
Xcodeproj::Plist.write_to_path({
  'CFBundleExecutable' => '$(EXECUTABLE_NAME)', 'CFBundleIdentifier' => '$(PRODUCT_BUNDLE_IDENTIFIER)',
  'CFBundleName' => '$(PRODUCT_NAME)', 'CFBundlePackageType' => 'APPL',
  'CFBundleVersion' => '1', 'CFBundleShortVersionString' => '1.0', 'UILaunchScreen' => {},
  'UIApplicationSceneManifest' => {
    'UIApplicationSupportsMultipleScenes' => false,
    'UISceneConfigurations' => { 'UIWindowSceneSessionRoleApplication' => [
      { 'UISceneConfigurationName' => 'Tests', 'UISceneDelegateClassName' => 'GeometryTestScene' }
    ] }
  }
}, File.join(output, 'Host-Info.plist'))

project = Xcodeproj::Project.new(File.join(output, 'WebViewGeometry.xcodeproj'))
app = project.new_target(:application, 'WebViewTestHost', :ios, '16.4')
app.source_build_phase.add_file_reference(project.main_group.new_file(File.join(output, 'Host.m')))
tests = project.new_target(:unit_test_bundle, 'WebViewGeometryTests', :ios, '16.4')
tests.add_dependency(app)
source = File.join(repo, 'apps/flutter/thirdparty/webview_flutter_wkwebview/darwin')
%w[WebViewGeometryTests.m WebViewProxyAPITests.swift TestProxyApiRegistrar.swift TestBinaryMessenger.swift].each do |file|
  tests.source_build_phase.add_file_reference(project.main_group.new_file(File.join(source, 'Tests', file)))
end
[app, tests].each do |target|
  target.build_configurations.each do |config|
    config.build_settings.merge!({
      'GENERATE_INFOPLIST_FILE' => 'YES', 'CODE_SIGNING_ALLOWED' => 'NO',
      'PRODUCT_BUNDLE_IDENTIFIER' => "app.operit.#{target.name}",
      'CLANG_ENABLE_MODULES' => 'YES', 'CLANG_ENABLE_OBJC_ARC' => 'YES',
      'TARGETED_DEVICE_FAMILY' => '1,2'
    })
  end
end
app.build_configurations.each do |config|
  config.build_settings['GENERATE_INFOPLIST_FILE'] = 'NO'
  config.build_settings['INFOPLIST_FILE'] = File.join(output, 'Host-Info.plist')
end
framework = project.main_group.new_file(flutter)
tests.frameworks_build_phase.add_file_reference(framework)
tests.frameworks_build_phase.add_file_reference(project.main_group.new_file(File.join(pod_build, 'libwebview_flutter_wkwebview.a')))
embed = app.new_copy_files_build_phase('Embed Flutter')
embed.dst_subfolder_spec = '10'
embed.add_file_reference(framework)
tests.build_configurations.each do |config|
  config.build_settings.merge!({
    'TEST_HOST' => '$(BUILT_PRODUCTS_DIR)/WebViewTestHost.app/WebViewTestHost',
    'BUNDLE_LOADER' => '$(TEST_HOST)', 'SWIFT_VERSION' => '5.0',
    'SWIFT_INCLUDE_PATHS' => pod_build, 'LIBRARY_SEARCH_PATHS' => ['$(inherited)', pod_build],
    'FRAMEWORK_SEARCH_PATHS' => File.dirname(flutter),
    'HEADER_SEARCH_PATHS' => [File.join(source, 'webview_flutter_wkwebview/Sources/OperitWebViewSafety/include'), pod_build,
                              File.join(ios, 'Pods/Headers/Public/webview_flutter_wkwebview')],
    'OTHER_SWIFT_FLAGS' => ['$(inherited)', '-Xcc', "-fmodule-map-file=#{pod_build}/webview_flutter_wkwebview.modulemap"],
    'LD_RUNPATH_SEARCH_PATHS' => ['$(inherited)', '@executable_path/Frameworks']
  })
end
project.save
scheme = Xcodeproj::XCScheme.new
scheme.add_build_target(app)
scheme.add_build_target(tests)
scheme.add_test_target(tests)
scheme.set_launch_target(app)
scheme.save_as(project.path, 'WebViewGeometry')

selectors = %w[
  WebViewGeometryTests
  WebViewProxyAPITests/testProductionWebViewRejectsInvalidGeometry
  WebViewProxyAPITests/testPigeonDefaultConstructor
  WebViewProxyAPITests/testWebViewContentInsetBehaviorShouldBeNever
  WebViewProxyAPITests/testContentInsetsSumAlwaysZeroAfterSetFrame
  WebViewProxyAPITests/testContentInsetsIsOppositeOfScrollViewAdjustedInset
].map { |name| "-only-testing:WebViewGeometryTests/#{name}" }
run_logged(File.join(output, 'tests.log'), 'xcodebuild', 'test', '-project', project.path.to_s,
           '-scheme', 'WebViewGeometry', '-destination', "platform=iOS Simulator,id=#{udid}",
           '-derivedDataPath', File.join(output, 'build'), '-resultBundlePath', File.join(output, 'results.xcresult'),
           *selectors, 'CODE_SIGNING_ALLOWED=NO')
puts File.readlines(File.join(output, 'tests.log')).grep(/Executed |TEST SUCCEEDED/).last(4).join
