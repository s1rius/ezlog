#import "EzlogFlutterPlugin.h"
#if __has_include(<ezlog_flutter/ezlog_flutter-Swift.h>)
#import <ezlog_flutter/ezlog_flutter-Swift.h>
#else
// Support project import fallback if the generated compatibility header
// is not copied when this plugin is created as a library.
// https://forums.swift.org/t/swift-static-libraries-dont-copy-generated-objective-c-header/19816
#import "ezlog_flutter-Swift.h"
#endif

@implementation EzlogFlutterPlugin
+ (void)registerWithRegistrar:(NSObject<FlutterPluginRegistrar>*)registrar {
  [SwiftEzlogFlutterPlugin registerWithRegistrar:registrar];
}
@end
