#import "Arm64TerminalBridge.mm"
#include <stdexcept>
#include "../../../hosts/ios/linux-runtime/tests/native_descriptor_budget.h"

static void Check(bool condition, NSString *message) {
  if (!condition) throw std::runtime_error(message.UTF8String ?: "ARM64 simulator check failed");
}
static NSDictionary *Envelope(NSString *command, NSDictionary *request) {
  NSData *json = [NSJSONSerialization dataWithJSONObject:request options:0 error:nil];
  NSString *text = [[NSString alloc] initWithData:json encoding:NSUTF8StringEncoding];
  char *raw = operit_ios_arm64_terminal_call(command.UTF8String, text.UTF8String);
  Check(raw != nullptr, @"Bridge returned no response");
  NSData *response = [NSData dataWithBytes:raw length:strlen(raw)];
  operit_ios_arm64_terminal_free(raw);
  NSDictionary *envelope = [NSJSONSerialization JSONObjectWithData:response options:0 error:nil];
  Check([envelope isKindOfClass:NSDictionary.class], @"Bridge returned invalid JSON");
  return envelope;
}
static NSDictionary *Call(NSString *command, NSDictionary *request) {
  NSDictionary *envelope = Envelope(command, request);
  Check(envelope[@"error"] == nil, envelope.description);
  return envelope[@"result"];
}
static NSString *Output(NSDictionary *result) {
  NSMutableData *data = [NSMutableData data];
  for (NSNumber *number in result[@"outputBytes"]) {
    uint8_t byte = number.unsignedCharValue;
    [data appendBytes:&byte length:1];
  }
  return [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
}
static NSDictionary *Execute(NSString *session, NSString *command, int status, NSString *expected) {
  NSDictionary *result = Call(@"terminalExecute", @{ @"sessionId": session, @"command": command, @"timeoutMs": @30000 });
  Check([result[@"exitCode"] intValue] == status && ![result[@"timedOut"] boolValue], result.description);
  Check(expected == nil || [Output(result) containsString:expected], result.description);
  fprintf(stderr, "COMMAND_PASSED=%s\n", command.UTF8String);
  return result;
}
struct Capture {
  NSMutableData *data = [NSMutableData data];
  double first = 0;
  bool cancel = false;
  double cancelAt = 0;
};
static bool Stream(void *context, const uint8_t *bytes, size_t size) {
  auto &capture = *static_cast<Capture *>(context);
  if (capture.cancelAt && NSProcessInfo.processInfo.systemUptime >= capture.cancelAt) return false;
  if (size) [capture.data appendBytes:bytes length:size];
  NSString *text = [[NSString alloc] initWithData:capture.data encoding:NSUTF8StringEncoding];
  if ([text containsString:@"LIVE_A"] && !capture.first) {
    capture.first = NSProcessInfo.processInfo.systemUptime;
    return !capture.cancel;
  }
  return true;
}
static NSDictionary *ExecuteStream(NSString *session, NSString *command, Capture &capture) {
  NSData *request = [NSJSONSerialization dataWithJSONObject:@{ @"sessionId": session, @"command": command, @"timeoutMs": @15000 } options:0 error:nil];
  NSString *json = [[NSString alloc] initWithData:request encoding:NSUTF8StringEncoding];
  char *raw = operit_ios_arm64_terminal_execute_streaming("terminalExecute", json.UTF8String, &capture, Stream);
  Check(raw != nullptr, @"Streaming returned no response");
  NSData *data = [NSData dataWithBytes:raw length:strlen(raw)];
  operit_ios_arm64_terminal_free(raw);
  return [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
}
int main(int argc, const char **argv) {
  @autoreleasepool {
    try {
      Check(argc == 5, @"Expected scenario, runtime directory, ARM64 archive, host workspace");
      NSString *error = nil;
      NSString *root = ORTArm64PrepareRootAtURL([NSURL fileURLWithPath:@(argv[2])], [NSURL fileURLWithPath:@(argv[3])], &error);
      if (!root) {
        fprintf(stderr, "PREPARATION_FAILED=%s\n", error.UTF8String);
        return 3;
      }
      Check([root isEqualToString:ORTArm64PrepareRootAtURL([NSURL fileURLWithPath:@(argv[2])], nil, &error)], error);
      if ([@(argv[1]) isEqualToString:@"prepare"]) return 0;
      NSString *workspace = [@(argv[4]) stringByResolvingSymlinksInPath];
      NSDictionary *configuration = Call(@"arm64RuntimeConfigure", @{
        @"rootDirectory": root, @"bindAppSandbox": @NO,
        @"mounts": @[ @{ @"hostDirectory": workspace, @"mountPoint": workspace, @"readOnly": @NO } ] });
      Check([configuration[@"rootDirectory"] isEqualToString:root] &&
            [configuration[@"rootSource"] isEqualToString:@"external"] &&
            ![configuration[@"bindAppSandbox"] boolValue], @"External root was not used in place");
      NSDictionary *created = Call(@"terminalCreateOrGet", @{ @"sessionName": @"smoke", @"terminalType": @"shell" });
      NSString *session = created[@"sessionId"];
      Check(session != nil && [created[@"rootDirectory"] isEqualToString:root], @"Session did not use the configured external root");
      NSDictionary *restored = Call(@"arm64RuntimeConfigure", configuration);
      Check([restored[@"configurationId"] isEqualToString:configuration[@"configurationId"]] &&
            [restored[@"closedSessionIds"] count] == 0, @"Bookmark round trip changed a live configuration");
      Execute(session, @"uname -m; printf '__ARM64_ROOT_READY__\\n'", 0, @"aarch64");
      Execute(session, @"sh -c 'exit 7'", 7, nil);
      Execute(session, @"python3 -c \"import sqlite3; print('PYTHON_SQLITE_OK', sqlite3.connect(':memory:').execute('select 42').fetchone()[0])\"", 0, @"PYTHON_SQLITE_OK 42");
      const auto idleDescriptors = NativeDescriptorBudget::count();
      for (unsigned repeat = 0; repeat < 4; ++repeat) {
        Execute(session, @"python3 -c 'import json, ssl, asyncio, subprocess; subprocess.run([\"/bin/echo\", \"PYTHON_FD_OK\"], check=True)'", 0, @"PYTHON_FD_OK");
        Check(NativeDescriptorBudget::count() == idleDescriptors, @"Completed Python guest retained native descriptors");
      }
      Execute(session, @"node -e \"console.log('NODE_OK', process.arch)\"", 0, @"NODE_OK arm64");
      Execute(session, @"npm --version", 0, nil);
      Execute(session, @"seq 1 3000; printf '__LARGE_OUTPUT_OK__\\n'", 0, @"__LARGE_OUTPUT_OK__");
      Capture capture;
      NSDictionary *streamed = ExecuteStream(session, @"printf '\\nLIVE_A\\n'; sleep 1; printf '\\nLIVE_B\\n'", capture);
      Check(streamed[@"error"] == nil && capture.first > 0 && NSProcessInfo.processInfo.systemUptime - capture.first > .2, streamed.description);
      Check([Output(streamed[@"result"]) containsString:@"LIVE_B"], streamed.description);
      Capture canceled;
      canceled.cancel = true;
      NSDictionary *cancel = ExecuteStream(session, @"printf '\\nLIVE_A\\n'; sleep 30", canceled);
      Check([cancel[@"error"] isKindOfClass:NSString.class], @"Output cancellation was ignored");
      // The foreground interrupt is delivered asynchronously through the VM queue.
      const double deadline = NSProcessInfo.processInfo.systemUptime + 5;
      for (;;) {
        NSDictionary *screen = Call(@"terminalScreen", @{ @"sessionId": session });
        if (![screen[@"commandRunning"] boolValue]) break;
        Check(NSProcessInfo.processInfo.systemUptime < deadline, @"Canceled shell did not become idle");
        [NSThread sleepForTimeInterval:.01];
      }
      Execute(session, @"printf '__AFTER_CANCEL__\\n'", 0, @"__AFTER_CANCEL__");
      Capture silent;
      silent.cancelAt = NSProcessInfo.processInfo.systemUptime + .2;
      NSDictionary *silentResult = ExecuteStream(session, @"sleep 5", silent);
      Check([silentResult[@"error"] isKindOfClass:NSString.class] &&
            NSProcessInfo.processInfo.systemUptime - silent.cancelAt < 3, @"Silent command cancellation was ignored");
      Execute(session, @"printf '__AFTER_SILENT_CANCEL__\\n'", 0, @"__AFTER_SILENT_CANCEL__");
      Call(@"terminalResize", @{ @"sessionId": session, @"rows": @37, @"cols": @101 });
      Execute(session, @"stty size", 0, @"37 101");
      NSString *command = [NSString stringWithFormat:@"cd %@; printf '__HOST_WRITE_OK__' > written.txt; pwd; cat probe.txt", ORTQuote(workspace)];
      Execute(session, command, 0, @"HOST_READ_OK");
      Check([[NSString stringWithContentsOfFile:[workspace stringByAppendingPathComponent:@"written.txt"] encoding:NSUTF8StringEncoding error:nil] isEqualToString:@"__HOST_WRITE_OK__"], @"Guest host-mount write was lost");
      Execute(session, @"pwd", 0, workspace);
      NSString *persistent = [root stringByAppendingPathComponent:@"root/.simulator-persistent"];
      if ([NSFileManager.defaultManager fileExistsAtPath:persistent])
        Execute(session, @"cat /root/.simulator-persistent; /usr/local/bin/simulator-persistent", 0, @"PERSISTENT_DATA_OKPERSISTENT_EXEC_OK");
      else
        Execute(session, @"printf PERSISTENT_DATA_OK > /root/.simulator-persistent; printf '#!/bin/sh\\nprintf PERSISTENT_EXEC_OK' > /usr/local/bin/simulator-persistent; chmod +x /usr/local/bin/simulator-persistent", 0, nil);
      NSMutableDictionary *readOnly = [configuration mutableCopy];
      readOnly[@"mounts"] = @[ @{ @"hostDirectory": workspace, @"mountPoint": workspace, @"readOnly": @YES } ];
      NSDictionary *refused = Envelope(@"arm64RuntimeConfigure", readOnly);
      Check([refused[@"error"] isKindOfClass:NSString.class], @"Live configuration was replaced without closing its session");
      Execute(session, @"printf '__CONFIG_REJECT_PRESERVED_SESSION__\n'", 0, @"__CONFIG_REJECT_PRESERVED_SESSION__");
      readOnly[@"closeExistingSessions"] = @YES;
      NSDictionary *switched = Call(@"arm64RuntimeConfigure", readOnly);
      Check([switched[@"closedSessionIds"] containsObject:session] &&
            ![switched[@"configurationId"] isEqualToString:configuration[@"configurationId"]],
            @"Configuration switch did not retire the old session");
      Check([Call(@"terminalList", @{})[@"sessions"] count] == 0, @"Retired session is still registered");
      session = Call(@"terminalCreateOrGet", @{ @"sessionName": @"read-only", @"terminalType": @"shell" })[@"sessionId"];
      NSString *readOnlyCommand = [NSString stringWithFormat:
          @"if printf blocked > %@/probe.txt; then exit 91; fi; cat %@/probe.txt; printf '__READ_ONLY_BIND_OK__\n'",
          ORTQuote(workspace), ORTQuote(workspace)];
      Execute(session, readOnlyCommand, 0, @"__READ_ONLY_BIND_OK__");
      Check([[NSString stringWithContentsOfFile:[workspace stringByAppendingPathComponent:@"probe.txt"]
                  encoding:NSUTF8StringEncoding error:nil] isEqualToString:@"HOST_READ_OK"], @"Read-only bind changed the host file");
      Call(@"terminalClose", @{ @"sessionId": session });
      Check([Call(@"terminalList", @{})[@"sessions"] count] == 0, @"Closed session is still registered");
      fprintf(stderr, "ARM64_SIMULATOR_SMOKE_PASSED\n");
      return 0;
    } catch (const std::exception &failure) {
      fprintf(stderr, "SMOKE_FAILED=%s\n", failure.what());
      return 1;
    }
  }
}
