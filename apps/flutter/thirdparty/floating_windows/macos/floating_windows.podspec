Pod::Spec.new do |s|
  s.name = 'floating_windows'
  s.version = '0.1.0'
  s.summary = 'Transparent Flutter desktop widget surfaces.'
  s.description = 'Native macOS window implementation for the floating_windows contract.'
  s.homepage = 'https://github.com/AAswordman/Operit2'
  s.license = { :type => 'AGPL-3.0', :file => '../LICENSE' }
  s.author = 'Operit contributors'
  s.source = { :path => '.' }
  s.source_files = 'Classes/**/*'
  s.dependency 'FlutterMacOS'
  s.dependency 'desktop_multi_window'
  s.platform = :osx, '10.15'
  s.swift_version = '5.0'
end
