Pod::Spec.new do |s|
  s.name             = 'operit_folder_access'
  s.version          = '0.1.0'
  s.summary          = 'Unified folder selection and persistent macOS folder access.'
  s.description      = s.summary
  s.homepage         = 'https://github.com/AAswordman/Operit'
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'Operit contributors' => '' }
  s.source           = { :path => '.' }
  s.source_files     = 'Classes/**/*.swift'
  s.dependency 'FlutterMacOS'
  s.platform         = :osx, '10.15'
  s.swift_version    = '5.0'
  s.frameworks       = 'AppKit', 'Security'
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES' }
end
