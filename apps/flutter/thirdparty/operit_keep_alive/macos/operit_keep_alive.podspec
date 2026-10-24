Pod::Spec.new do |s|
  s.name = 'operit_keep_alive'
  s.version = '0.1.0'
  s.summary = 'Process-owned Flutter background residency.'
  s.description = s.summary
  s.homepage = 'https://github.com/AAswordman/Operit'
  s.license = { :type => 'MIT', :file => '../LICENSE' }
  s.author = { 'Operit' => 'dev@operit.app' }
  s.source = { :path => '.' }
  s.source_files = 'Classes/**/*'
  s.dependency 'FlutterMacOS'
  s.platform = :osx, '10.15'
  s.swift_version = '5.0'
end
