#
# Be sure to run `pod lib lint EZLog.podspec' to ensure this is a
# valid spec before submitting.
#
# Any lines starting with a # are optional, but their use is encouraged
# To learn more about a Podspec see https://guides.cocoapods.org/syntax/podspec.html
#

Pod::Spec.new do |s|
  s.name             = 'EZLog'
  s.version          = '0.2.0'
  s.summary          = 'A high efficiency cross-platform Logging Library.'
  s.swift_version = '5.6'

  s.description      = <<-DESC
  A high efficiency Cross-platform Logging Library.
                       DESC

  s.homepage         = 'https://github.com/s1rius/ezlog'
  s.license          = { :type => 'MIT', :file => 'LICENSE-MIT' }
  s.author           = { 's1rius' => 's1rius.noone@gmail.com' }
  s.source           = { :git => 'https://github.com/s1rius/ezlog.git', :tag => '0.2.0' }

  s.ios.deployment_target = '13.0'
  s.prepare_command = <<-CMD
  if [ -d "./ios/framework/" ] && [ "$(find ./ios/framework/ -mindepth 1 -print -quit)" ]; then
    echo "framework folder exists and is not empty， exit"
    exit 0
  fi
  echo "download framework from internet"
  mkdir ./ios/build
  curl -L -o ./ios/build/ezlog.xcframework.zip "https://github.com/s1rius/ezlog/releases/download/0.2.0/ezlog_0.2.0_xcframework.zip"
  if [ -s "./ios/build/ezlog.xcframework.zip" ]; then
    echo "unzip xcframework"
    unzip -o ./ios/build/ezlog.xcframework.zip -d ./ios/framework
  else
    echo "xcframework build from source"
    pushd ios
    sh b_ios.sh
  fi
  CMD
  s.source_files = 'ios/EZLog/Sources/**/*'
  s.ios.vendored_frameworks = 'ios/framework/ezlog.xcframework'
end
