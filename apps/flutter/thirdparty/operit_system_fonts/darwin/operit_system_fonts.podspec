Pod::Spec.new do |s|
  s.name = 'operit_system_fonts'
  s.version = '0.1.0'
  s.summary = 'Client-local installed font enumeration and access.'
  s.description = s.summary
  s.homepage = 'https://operit.app'
  s.license = { :type => 'AGPL-3.0-only' }
  s.author = { 'Operit developers' => '' }
  s.source = { :path => '.' }
  s.source_files = 'Classes/**/*.swift'
  s.ios.dependency 'Flutter'
  s.osx.dependency 'FlutterMacOS'
  s.ios.deployment_target = '13.0'
  s.osx.deployment_target = '10.15'
  s.swift_version = '5.0'
  s.frameworks = 'CoreText'
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES' }
end
