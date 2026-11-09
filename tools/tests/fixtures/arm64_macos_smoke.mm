#import <Foundation/Foundation.h>
#import <objc/runtime.h>
#import "OperitFlutterBridge.h"
#include <stdexcept>
#include "../../../hosts/ios/linux-runtime/tests/native_descriptor_budget.h"

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
  for (NSNumber *value in result[@"outputBytes"] ?: result[@"bytes"] ?: result[@"contentBytes"]) {
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
struct Capture {
  NSMutableData *data = [NSMutableData data]; bool cancel = false, live = false;
  NSString *session = nil, *command = nil;
  NSMutableString *terminal = [NSMutableString string];
};
static bool Stream(void *context, const uint8_t *bytes, size_t count) {
  auto &capture = *static_cast<Capture *>(context);
  if (count) [capture.data appendBytes:bytes length:count];
  NSString *text = [[NSString alloc] initWithData:capture.data encoding:NSUTF8StringEncoding];
  if (capture.session) {
    [capture.terminal appendString:Output(Call(@"terminalRead", @{ @"sessionId": capture.session }))];
    if ([text containsString:@"STREAM_A"] && ![text containsString:@"STREAM_B"]) {
      Check([capture.terminal containsString:capture.command] && [capture.terminal containsString:@"STREAM_A"], @"Command and live output missing from terminal");
      Check([Call(@"terminalScreen", @{ @"sessionId": capture.session })[@"commandRunning"] boolValue], @"Output arrived after completion");
      capture.live = true;
    }
  }
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
      // Replayed cursor replies and partial manual input must not prefix an automated command.
      for (NSString *input in @[@"\033[0;8R\033[0;40R\033[0;24R\033[0;32R\033[0;16R\033[0;48R\033[0;63R\033[1;13R\033[12;15R\033[18;15R\033[18;30R\033[18;45R\033[18;60R\033[19;25R\033[19;10R", @"unfinished"]) {
        NSData *data = [input dataUsingEncoding:NSUTF8StringEncoding];
        NSMutableArray *bytes = [NSMutableArray array];
        for (NSUInteger i = 0; i < data.length; ++i) [bytes addObject:@(static_cast<const uint8_t *>(data.bytes)[i])];
        Call(@"terminalWrite", @{ @"sessionId": session, @"bytes": bytes });
        Execute(session, @"printf CLEAN_INPUT", @"CLEAN_INPUT");
      }
      Execute(session, @"export OPERIT_SMOKE=persistent; cd /tmp", @"");
      Execute(session, @"printf '%s:%s' \"$OPERIT_SMOKE\" \"$PWD\"", @"persistent:/tmp");
      Execute(session, @"python3 -c 'print(6 * 7)'", @"42");
      const auto idleDescriptors = NativeDescriptorBudget::count();
      for (unsigned repeat = 0; repeat < 4; ++repeat) {
        Execute(session, @"python3 -c 'import json, ssl, asyncio, subprocess; subprocess.run([\"/bin/echo\", \"PYTHON_FD_OK\"], check=True)'", @"PYTHON_FD_OK");
        Check(NativeDescriptorBudget::count() == idleDescriptors, @"Completed Python guest retained native descriptors");
      }
      Execute(session, @"test ! -d /Users && printf NO_HOST_MOUNTS", @"NO_HOST_MOUNTS");
      Call(@"terminalResize", @{ @"sessionId": session, @"rows": @40, @"cols": @100 });
      Execute(session, @"stty size", @"40 100");
      NSDictionary *large = Call(@"terminalExecute", @{ @"sessionId": session, @"command": @"head -c 1048577 /dev/zero", @"timeoutMs": @30000 });
      Check([large[@"exitCode"] intValue] == 0 && ![large[@"timedOut"] boolValue] && [large[@"outputBytes"] count] == 1048577, @"Unread terminal blocked command capture");
      Check([Call(@"terminalRead", @{ @"sessionId": session })[@"bytes"] count] <= 1024 * 1024, @"Unread terminal exceeded its buffer limit");
      NSString *heredoc = @"cat <<'EOF'\nVISIBLE_HEREDOC\nEOF";
      Execute(session, heredoc, @"VISIBLE_HEREDOC");
      Check([Output(Call(@"terminalRead", @{ @"sessionId": session })) containsString:[heredoc stringByReplacingOccurrencesOfString:@"\n" withString:@"\r\n"]], @"Multiline command missing from terminal");
      Capture capture;
      capture.session = session; capture.command = @"printf 'STREAM_%s' A; sleep 1; printf STREAM_B";
      NSDictionary *request = @{ @"sessionId": session, @"command": capture.command, @"timeoutMs": @15000 };
      NSDictionary *stream = Decode(operit_macos_arm64_terminal_execute_streaming("terminalExecute", Encode(request).UTF8String, &capture, Stream));
      Check(stream[@"error"] == nil && [Output(stream[@"result"]) isEqualToString:@"STREAM_ASTREAM_B"], stream.description);
      Check(capture.live && ![capture.terminal containsString:@".operit-command-"] && ![capture.terminal containsString:@"begin:"], @"Terminal stream missing or exposed internal protocol");
      Check([Output(Call(@"terminalScreen", @{ @"sessionId": session })) containsString:capture.command], @"Command missing from terminal history");
      Capture canceled;
      canceled.cancel = true;
      request = @{ @"sessionId": session, @"command": @"printf STREAM_A; sleep 10", @"timeoutMs": @15000 };
      NSDictionary *cancel = Decode(operit_macos_arm64_terminal_execute_streaming("terminalExecute", Encode(request).UTF8String, &canceled, Stream));
      Check(cancel[@"error"] != nil, @"Streaming cancellation did not interrupt execution");
      Execute(session, @"printf RECOVERED", @"RECOVERED");
      // Host workspaces share their actual files, and adding one never closes another VM.
      NSString *workspace = [TestSupport.path stringByAppendingPathComponent:@"workspace 中文 ' $literal"];
      Check([NSFileManager.defaultManager createDirectoryAtPath:workspace withIntermediateDirectories:YES attributes:nil error:nil], @"Create workspace");
      NSString *hostFile = [workspace stringByAppendingPathComponent:@"host.txt"];
      Check([@"host-original" writeToFile:hostFile atomically:YES encoding:NSUTF8StringEncoding error:nil], @"Write workspace source");
      Call(@"terminalWorkspaceMount", @{ @"hostDirectory": workspace, @"mountPoint": workspace });
      Execute(session, @"printf ORIGINAL_ALIVE", @"ORIGINAL_ALIVE");
      NSDictionary *workspaceLaunch = Call(@"terminalStart", @{ @"sessionName": @"workspace-smoke", @"terminalType": @"shell",
          @"workingDir": workspace, @"rows": @24, @"cols": @80 });
      NSString *workspaceSession = workspaceLaunch[@"sessionId"];
      Execute(workspaceSession, @"pwd; cat host.txt; printf guest-written > guest.txt", @"host-original");
      Check([[NSString stringWithContentsOfFile:[workspace stringByAppendingPathComponent:@"guest.txt"] encoding:NSUTF8StringEncoding error:nil]
             isEqualToString:@"guest-written"], @"Guest changes did not reach the original host workspace");
      Check([@"host-updated-live" writeToFile:hostFile atomically:YES encoding:NSUTF8StringEncoding error:nil], @"Update host file");
      Execute(workspaceSession, @"cat host.txt", @"host-updated-live");
      NSString *secondWorkspace = [TestSupport.path stringByAppendingPathComponent:@"second-workspace"];
      Check([NSFileManager.defaultManager createDirectoryAtPath:secondWorkspace withIntermediateDirectories:YES attributes:nil error:nil], @"Create second workspace");
      Call(@"terminalWorkspaceMount", @{ @"hostDirectory": secondWorkspace, @"mountPoint": secondWorkspace });
      NSDictionary *secondLaunch = Call(@"terminalCreateOrGet", @{ @"sessionName": @"workspace-smoke", @"terminalType": @"shell" });
      Check(![secondLaunch[@"sessionId"] isEqualToString:workspaceSession], @"Reused VM has an obsolete workspace namespace");
      Execute(workspaceSession, @"cat host.txt", @"host-updated-live");
      Execute(session, @"printf STILL_ALIVE", @"STILL_ALIVE");
      Call(@"terminalClose", @{ @"sessionId": workspaceSession });
      Call(@"terminalClose", @{ @"sessionId": secondLaunch[@"sessionId"] });
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
