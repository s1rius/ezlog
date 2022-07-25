#
# Be sure to run `pod lib lint EZLog.podspec' to ensure this is a
# valid spec before submitting.
#
# Any lines starting with a # are optional, but their use is encouraged
# To learn more about a Podspec see https://guides.cocoapods.org/syntax/podspec.html
#

Pod::Spec.new do |s|
  s.name             = 'EZLog'
  s.version          = '0.1.4'
  s.summary          = 'A high efficiency Cross-platform Logging Library.'
  s.swift_version = '5.6'

  s.description      = <<-DESC
  A high efficiency Cross-platform Logging Library.
                       DESC

  s.homepage         = 'https://github.com/s1rius/ezlog'
  s.license          = { :type => 'MIT', :file => 'LICENSE' }
  s.author           = { 's1rius' => 's1rius.noone@gmail.com' }
  s.source           = { :git => 'https://github.com/s1rius/ezlog.git', :tag => s.version.to_s }

  s.ios.deployment_target = '13.0'

  s.source_files = 'ios/EZLog/Source/**/*'
  s.ios.vendored_frameworks = 'ios/framework/ezlog.xcframework'
end
