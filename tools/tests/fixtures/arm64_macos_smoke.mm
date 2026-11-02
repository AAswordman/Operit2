#import <Foundation/Foundation.h>
#import <objc/runtime.h>
#import "OperitFlutterBridge.h"
#include <stdexcept>

// Isolate Foundation's Application Support lookup without changing production
// root preparation or touching the developer's installed application data.
static NSURL *TestSupport;
@interface NSFileManager (ORTSmoke)
- (NSArray<NSURL *> *)ortSmokeURLsForDirectory:(NSSearchPathDirectory)directory
                                   inDomains:(NSSearchPathDomainMask)domains;
@end
@implementation NSFileManager (ORTSmoke)
- (NSArray<NSURL *> *)ortSmokeURLsForDirectory:(NSSearchPathDirectory)directory
                                   inDomains:(NSSearchPathDomainMask)domains {
  if (directory == NSApplicationSupportDirectory) return @[TestSupport];
  return [self ortSmokeURLsForDirectory:directory inDomains:domains];
}
@end

static void Check(bool condition, NSString *message) {
  if (!condition) throw std::runtime_error(message.UTF8String ?: "macOS ARM64 smoke failed");
}
static NSDictionary *Decode(char *raw) {
  Check(raw != nullptr, @"Bridge returned no response");
  NSData *data = [NSData dataWithBytes:raw length:strlen(raw)];
  operit_macos_arm64_terminal_free(raw);
  id envelope = [NSJSONSerialization JSONObjectWithData:data options:0 error:nullptr];
  Check([envelope isKindOfClass:NSDictionary.class], @"Invalid bridge response");
  return envelope;
}
static NSString *Encode(NSDictionary *request) {
  NSData *data = [NSJSONSerialization dataWithJSONObject:request options:0 error:nullptr];
  return [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
}
static NSDictionary *Call(NSString *command, NSDictionary *request) {
  NSDictionary *envelope = Decode(operit_macos_arm64_terminal_call(command.UTF8String, Encode(request).UTF8String));
  Check(envelope[@"error"] == nil, envelope.description);
  return envelope[@"result"];
}
static NSString *Output(NSDictionary *result) {
  NSMutableData *data = [NSMutableData data];
  for (NSNumber *value in result[@"outputBytes"]) {
    uint8_t byte = value.unsignedCharValue;
    [data appendBytes:&byte length:1];
  }
  return [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
}
static void Execute(NSString *session, NSString *command, NSString *expected) {
  NSDictionary *result = Call(@"terminalExecute", @{ @"sessionId": session, @"command": command, @"timeoutMs": @30000 });
  Check([result[@"exitCode"] intValue] == 0 && ![result[@"timedOut"] boolValue], result.description);
  Check(expected.length == 0 || [Output(result) containsString:expected],
        [NSString stringWithFormat:@"Command %@: %@", command, result]);
}
struct Capture { NSMutableData *data = [NSMutableData data]; bool cancel = false; };
static bool Stream(void *context, const uint8_t *bytes, size_t count) {
  auto &capture = *static_cast<Capture *>(context);
  if (count) [capture.data appendBytes:bytes length:count];
  NSString *text = [[NSString alloc] initWithData:capture.data encoding:NSUTF8StringEncoding];
  return !(capture.cancel && [text containsString:@"STREAM_A"]);
}
int main(int argc, const char **argv) {
  @autoreleasepool {
    try {
      Check(argc == 3, @"Expected isolated support directory and restart flag");
      TestSupport = [NSURL fileURLWithPath:@(argv[1]) isDirectory:YES];
      method_exchangeImplementations(
          class_getInstanceMethod(NSFileManager.class, @selector(URLsForDirectory:inDomains:)),
          class_getInstanceMethod(NSFileManager.class, @selector(ortSmokeURLsForDirectory:inDomains:)));
      Check([NSBundle.mainBundle URLForResource:@"arm64-rootfs" withExtension:@"tar.gz"] != nil, @"Missing bundled root archive");
      NSDictionary *launch = Call(@"terminalStart", @{ @"sessionName": @"macos-smoke", @"terminalType": @"shell",
          @"workingDir": @"/root", @"rows": @24, @"cols": @80 });
      NSString *session = launch[@"sessionId"];
      Check([session hasPrefix:@"macos-arm64-"], launch.description);
      Check([launch[@"rootDirectory"] hasPrefix:TestSupport.path], @"Root escaped isolated support directory");
      NSDictionary *configuration = Call(@"arm64RuntimeConfiguration", @{});
      Check([configuration[@"rootSource"] isEqual:@"bundled"], configuration.description);
      Check(![configuration[@"bindAppSandbox"] boolValue] && [configuration[@"mounts"] count] == 0,
            @"Default macOS runtime unexpectedly mounted host directories");
      if (strcmp(argv[2], "1") == 0)
        Execute(session, @"cat /root/macos-smoke-persisted", @"native-root-persisted");
      Execute(session, @"printf native-root-persisted > /root/macos-smoke-persisted", @"");
      Execute(session, @"uname -m; pwd", @"aarch64");
      Execute(session, @"printf MACOS_ARM64_OK", @"MACOS_ARM64_OK");
      Execute(session, @"export OPERIT_SMOKE=persistent; cd /tmp", @"");
      Execute(session, @"printf '%s:%s' \"$OPERIT_SMOKE\" \"$PWD\"", @"persistent:/tmp");
      Execute(session, @"python3 -c 'print(6 * 7)'", @"42");
      Execute(session, @"test ! -d /Users && printf NO_HOST_MOUNTS", @"NO_HOST_MOUNTS");
      Call(@"terminalResize", @{ @"sessionId": session, @"rows": @40, @"cols": @100 });
      Execute(session, @"stty size", @"40 100");
      Capture capture;
      NSDictionary *request = @{ @"sessionId": session, @"command": @"printf STREAM_A; sleep 1; printf STREAM_B", @"timeoutMs": @15000 };
      NSDictionary *stream = Decode(operit_macos_arm64_terminal_execute_streaming("terminalExecute", Encode(request).UTF8String, &capture, Stream));
      Check(stream[@"error"] == nil && [Output(stream[@"result"]) containsString:@"STREAM_B"], stream.description);
      Check(capture.data.length > 0, @"Streaming callback received no output");
      Capture canceled;
      canceled.cancel = true;
      request = @{ @"sessionId": session, @"command": @"printf STREAM_A; sleep 10", @"timeoutMs": @15000 };
      NSDictionary *cancel = Decode(operit_macos_arm64_terminal_execute_streaming("terminalExecute", Encode(request).UTF8String, &canceled, Stream));
      Check(cancel[@"error"] != nil, @"Streaming cancellation did not interrupt execution");
      Execute(session, @"printf RECOVERED", @"RECOVERED");
      Call(@"terminalClose", @{ @"sessionId": session });
      Check([Call(@"terminalList", @{})[@"sessions"] count] == 0, @"Closed session still listed");
      fprintf(stderr, "ARM64_MACOS_SMOKE_PASSED\n");
      return 0;
    } catch (const std::exception &error) {
      fprintf(stderr, "SMOKE_FAILED=%s\n", error.what());
      return 1;
    }
  }
}
