#
# To learn more about a Podspec see http://guides.cocoapods.org/syntax/podspec.html.
# Run `pod lib lint ezlog_flutter.podspec` to validate before publishing.
#
Pod::Spec.new do |s|
  s.name             = 'ezlog_flutter'
  s.version          = '0.0.1'
  s.summary          = 'EZLog Flutter plugin project. A cross-platform file logging library.'
  s.description      = <<-DESC
  EZLog Flutter plugin project. A cross-platform file logging library.
                       DESC
  s.homepage         = 'http://example.com'
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'Your Company' => 'email@example.com' }
  s.source           = { :path => '.' }
  s.source_files = 'Classes/**/*'
  s.dependency 'Flutter'
  s.dependency 'EZLog', '~> 0.1'
  s.platform = :ios, '13.0'

  # Flutter.framework does not contain a i386 slice.
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES', 'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386' }
  s.swift_version = '5.0'
end
