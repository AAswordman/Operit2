#!/usr/bin/env ruby
# Runs the production OCR implementation with ImageIO/Vision in a UIKit host.
# Usage: ruby tools/tests/run_ios_ocr_tests.rb [simulator-udid]
require 'xcodeproj'
require 'json'
require 'tmpdir'

repo = File.expand_path('../..', __dir__)
output = Dir.mktmpdir('operit-ios-ocr-')
puts "OCR test artifacts: #{output}"
udid = ARGV.first
unless udid
  devices = JSON.parse(IO.popen(['xcrun', 'simctl', 'list', 'devices', 'available', '--json'], &:read))
  candidates = devices.fetch('devices').select { |runtime, _| runtime.include?('.iOS-') }.values.flatten
  udid = (candidates.find { |device| device['state'] == 'Booted' } || candidates.first)&.fetch('udid')
end
abort 'An available iOS simulator is required' unless udid
File.write(File.join(output, 'Host.swift'), <<~SWIFT)
  import UIKit
  class OCRTestScene: UIResponder, UIWindowSceneDelegate {
    var window: UIWindow?
    func scene(_ scene: UIScene, willConnectTo session: UISceneSession, options: UIScene.ConnectionOptions) {
      guard let scene = scene as? UIWindowScene else { return }
      window = UIWindow(windowScene: scene)
      window?.rootViewController = UIViewController()
      window?.makeKeyAndVisible()
    }
  }
  @main class OCRTestApp: UIResponder, UIApplicationDelegate {
    func application(_ application: UIApplication, configurationForConnecting session: UISceneSession,
      options: UIScene.ConnectionOptions) -> UISceneConfiguration {
      let configuration = UISceneConfiguration(name: "Tests", sessionRole: session.role)
      configuration.delegateClass = OCRTestScene.self
      return configuration
    }
  }
SWIFT
Xcodeproj::Plist.write_to_path({
  'CFBundleExecutable' => '$(EXECUTABLE_NAME)', 'CFBundleIdentifier' => '$(PRODUCT_BUNDLE_IDENTIFIER)',
  'CFBundleName' => '$(PRODUCT_NAME)', 'CFBundlePackageType' => 'APPL',
  'CFBundleVersion' => '1', 'CFBundleShortVersionString' => '1.0', 'UILaunchScreen' => {},
  'UIApplicationSceneManifest' => { 'UIApplicationSupportsMultipleScenes' => false,
    'UISceneConfigurations' => { 'UIWindowSceneSessionRoleApplication' => [
      { 'UISceneConfigurationName' => 'Tests', 'UISceneDelegateClassName' => 'Runner.OCRTestScene' }
    ] }
  }
}, File.join(output, 'Host-Info.plist'))
project = Xcodeproj::Project.new(File.join(output, 'OCRTests.xcodeproj'))
app = project.new_target(:application, 'OCRTestHost', :ios, '16.4')
app.source_build_phase.add_file_reference(project.main_group.new_file(File.join(output, 'Host.swift')))
app.source_build_phase.add_file_reference(project.main_group.new_file(File.join(repo, 'apps/flutter/app/ios/Runner/AppleTextRecognizer.swift')))
tests = project.new_target(:unit_test_bundle, 'OCRTests', :ios, '16.4')
tests.add_dependency(app)
tests.source_build_phase.add_file_reference(project.main_group.new_file(File.join(repo, 'apps/flutter/app/ios/RunnerTests/AppleTextRecognizerTests.swift')))
[app, tests].each do |target|
  target.build_configurations.each do |config|
    config.build_settings.merge!({
      'GENERATE_INFOPLIST_FILE' => 'YES', 'CODE_SIGNING_ALLOWED' => 'NO',
      'PRODUCT_BUNDLE_IDENTIFIER' => "app.operit.#{target.name}",
      'CLANG_ENABLE_MODULES' => 'YES', 'SWIFT_VERSION' => '5.0',
      'TARGETED_DEVICE_FAMILY' => '1,2', 'ENABLE_TESTABILITY' => 'YES'
    })
  end
end
app.build_configurations.each do |config|
  config.build_settings['PRODUCT_MODULE_NAME'] = 'Runner'
  config.build_settings['GENERATE_INFOPLIST_FILE'] = 'NO'
  config.build_settings['INFOPLIST_FILE'] = File.join(output, 'Host-Info.plist')
end
tests.build_configurations.each do |config|
  config.build_settings['TEST_HOST'] = '$(BUILT_PRODUCTS_DIR)/OCRTestHost.app/OCRTestHost'
  config.build_settings['BUNDLE_LOADER'] = '$(TEST_HOST)'
end
project.save
scheme = Xcodeproj::XCScheme.new
scheme.add_build_target(app)
scheme.add_build_target(tests)
scheme.add_test_target(tests)
scheme.set_launch_target(app)
scheme.save_as(project.path, 'OCRTests')
command = ['xcodebuild', 'test', '-project', project.path.to_s, '-scheme', 'OCRTests',
           '-destination', "platform=iOS Simulator,id=#{udid}", '-derivedDataPath', File.join(output, 'build'),
           '-resultBundlePath', File.join(output, 'results.xcresult'), 'CODE_SIGNING_ALLOWED=NO']
puts command.join(' ')
log = File.join(output, 'tests.log')
unless system(*command, out: log, err: [:child, :out])
  warn File.readlines(log).last(60).join
  abort "OCR tests failed; full log: #{log}"
end
puts File.readlines(log).grep(/Executed |TEST SUCCEEDED/).last(4).join
