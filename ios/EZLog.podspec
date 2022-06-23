#
# Be sure to run `pod lib lint EZLog.podspec' to ensure this is a
# valid spec before submitting.
#
# Any lines starting with a # are optional, but their use is encouraged
# To learn more about a Podspec see https://guides.cocoapods.org/syntax/podspec.html
#

Pod::Spec.new do |s|
  s.name             = 'EZLog'
  s.version          = '0.1.0'
  s.summary          = 'A short description of EZLog.'
  s.swift_version = '5.6'

# This description is used to generate tags and improve search results.
#   * Think: What does it do? Why did you write it? What is the focus?
#   * Try to keep it short, snappy and to the point.
#   * Write the description between the DESC delimiters below.
#   * Finally, don't worry about the indent, CocoaPods strips it!

  s.description      = <<-DESC
TODO: Add long description of the pod here.
                       DESC

  s.homepage         = 'https://github.com/s1rius/ezlog'
  # s.screenshots     = 'www.example.com/screenshots_1', 'www.example.com/screenshots_2'
  s.license          = { :type => 'MIT', :file => 'LICENSE' }
  s.author           = { 's1rius' => 's1rius.noone@gmail.com' }
  s.source           = { :git => 'https://github.com/s1rius/ezlog.git', :tag => s.version.to_s }

  s.ios.deployment_target = '15.0'

  s.source_files = 'ios/ezlog/Source/**/*'
  s.ios.vendored_frameworks = 'ios/framework/ezlog.xcframework'
  
  # s.resource_bundles = {
  #   'EZLog' => ['EZLog/Assets/*.png']
  # }

  # s.public_header_files = 'Pod/Classes/**/*.h'
  # s.frameworks = 'UIKit', 'MapKit'
  # s.dependency 'AFNetworking', '~> 2.3'
end
