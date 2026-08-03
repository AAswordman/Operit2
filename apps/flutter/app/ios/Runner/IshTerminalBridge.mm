#import <Foundation/Foundation.h>
#import <UIKit/UIKit.h>

#include <stdlib.h>
#include <string.h>

extern "C" {
#import "LinuxInterop.h"
#include "tools/fakefs.h"
}

static const NSUInteger ORTIshOutputCapacity = 1024 * 1024;
static const NSTimeInterval ORTIshStartTimeout = 30.0;

@interface ORTIshTerminalSession : NSObject
@property(nonatomic, copy) NSString *sessionId;
@property(nonatomic, copy) NSString *sessionName;
@property(nonatomic, copy) NSString *workingDir;
@property(nonatomic, assign) nsobj_t terminal;
@property(nonatomic, assign) struct linux_tty *tty;
@property(nonatomic, strong) NSMutableData *pendingOutput;
@property(nonatomic, strong) NSMutableData *screenOutput;
@property(nonatomic, assign) NSInteger rows;
@property(nonatomic, assign) NSInteger cols;
@property(nonatomic, assign) BOOL commandRunning;
@property(nonatomic, assign) BOOL closed;
@property(nonatomic, assign) NSInteger exitCode;
@property(nonatomic, assign) BOOL hasExitCode;
@property(nonatomic, strong) dispatch_semaphore_t startSignal;
@end

@implementation ORTIshTerminalSession

/// Initializes the mutable state owned by one iSH PTY session.
- (instancetype)init {
  self = [super init];
  if (self) {
    _pendingOutput = [NSMutableData data];
    _screenOutput = [NSMutableData data];
    _startSignal = dispatch_semaphore_create(0);
  }
  return self;
}

@end

static NSMutableDictionary<NSString *, ORTIshTerminalSession *> *ORTIshSessions;
static NSMutableDictionary<NSString *, NSString *> *ORTIshSessionKeys;
static ORTIshTerminalSession *ORTIshPendingSession;
static NSLock *ORTIshStateLock;
static NSLock *ORTIshStartLock;
static NSString *ORTIshRootPath;
static BOOL ORTIshKernelStarted;
static uint64_t ORTIshNextSessionId;

/// Initializes the process-global iSH bridge state.
static void ORTIshInitializeState(void) {
  static dispatch_once_t onceToken;
  dispatch_once(&onceToken, ^{
    ORTIshSessions = [NSMutableDictionary dictionary];
    ORTIshSessionKeys = [NSMutableDictionary dictionary];
    ORTIshStateLock = [NSLock new];
    ORTIshStartLock = [NSLock new];
    ORTIshNextSessionId = 1;
  });
}

/// Returns a JSON envelope containing one successful bridge result.
static NSDictionary *ORTIshResult(id result) {
  return @{ @"result" : result ?: [NSNull null] };
}

/// Returns a JSON envelope containing one bridge error.
static NSDictionary *ORTIshError(NSString *message) {
  return @{ @"error" : message ?: @"iSH terminal bridge failed" };
}

/// Serializes one bridge envelope into a malloc-owned UTF-8 C string.
static char *ORTIshEncodeResponse(NSDictionary *response) {
  NSError *error = nil;
  NSData *data = [NSJSONSerialization dataWithJSONObject:response options:0 error:&error];
  if (data == nil) {
    return strdup("{\"error\":\"iSH terminal response encoding failed\"}");
  }
  return strdup([[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding].UTF8String);
}

/// Reads a required non-empty string request field.
static NSString *ORTIshRequiredString(NSDictionary *request, NSString *key, NSString **errorOut) {
  id value = request[key];
  if (![value isKindOfClass:[NSString class]] || [((NSString *)value) length] == 0) {
    *errorOut = [NSString stringWithFormat:@"iSH terminal request has invalid %@", key];
    return nil;
  }
  return value;
}

/// Reads one positive integer terminal dimension from a request object.
static NSInteger ORTIshPositiveDimension(NSDictionary *request, NSString *key, NSString **errorOut) {
  id value = request[key];
  if (![value isKindOfClass:[NSNumber class]] || [value integerValue] <= 0) {
    *errorOut = [NSString stringWithFormat:@"iSH terminal request has invalid %@", key];
    return 0;
  }
  return [value integerValue];
}

/// Resolves one active iSH session by its stable bridge identifier.
static ORTIshTerminalSession *ORTIshSession(NSString *sessionId, NSString **errorOut) {
  ORTIshInitializeState();
  [ORTIshStateLock lock];
  ORTIshTerminalSession *session = ORTIshSessions[sessionId];
  [ORTIshStateLock unlock];
  if (session == nil) {
    *errorOut = [NSString stringWithFormat:@"iSH terminal session does not exist: %@", sessionId];
  }
  return session;
}

/// Converts terminal bytes into required UTF-8 text for the Rust bridge protocol.
static NSString *ORTIshText(NSData *data, NSString **errorOut) {
  NSString *text = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
  if (text == nil) {
    *errorOut = @"iSH terminal emitted non-UTF-8 output";
  }
  return text;
}

/// Drains output accumulated for one iSH session.
static NSData *ORTIshDrainOutput(ORTIshTerminalSession *session) {
  @synchronized (session) {
    NSData *output = [session.pendingOutput copy];
    [session.pendingOutput setLength:0];
    return output;
  }
}

/// Writes raw bytes into one iSH Linux PTY.
static BOOL ORTIshWrite(ORTIshTerminalSession *session, NSData *input, NSString **errorOut) {
  @synchronized (session) {
    if (session.tty == NULL || session.closed) {
      *errorOut = @"iSH terminal session is not running";
      return NO;
    }
    struct linux_tty *tty = session.tty;
    async_do_in_workqueue(^{
      tty->ops->send_input(tty, static_cast<const char *>(input.bytes), input.length);
    });
  }
  return YES;
}

/// Updates the terminal geometry through the iSH Linux PTY callback.
static BOOL ORTIshResize(ORTIshTerminalSession *session, NSInteger rows, NSInteger cols, NSString **errorOut) {
  @synchronized (session) {
    if (session.tty == NULL || session.closed) {
      *errorOut = @"iSH terminal session is not running";
      return NO;
    }
    session.rows = rows;
    session.cols = cols;
    struct linux_tty *tty = session.tty;
    async_do_in_workqueue(^{
      tty->ops->resize(tty, (int)cols, (int)rows);
    });
  }
  return YES;
}

/// Imports the bundled Alpine root archive into the persistent iSH fakefs directory.
static BOOL ORTIshPrepareRoot(NSString **errorOut) {
  NSFileManager *fileManager = [NSFileManager defaultManager];
  NSURL *supportDirectory = [fileManager URLsForDirectory:NSApplicationSupportDirectory
                                                inDomains:NSUserDomainMask].firstObject;
  NSURL *runtimeDirectory = [supportDirectory URLByAppendingPathComponent:@"operit-ish"];
  NSURL *rootDirectory = [runtimeDirectory URLByAppendingPathComponent:@"rootfs"];
  NSError *directoryError = nil;
  if (![fileManager createDirectoryAtURL:runtimeDirectory
             withIntermediateDirectories:YES
                              attributes:nil
                                   error:&directoryError]) {
    *errorOut = directoryError.localizedDescription;
    return NO;
  }
  if ([fileManager fileExistsAtPath:rootDirectory.path]) {
    ORTIshRootPath = rootDirectory.path;
    return YES;
  }
  NSURL *archive = [NSBundle.mainBundle URLForResource:@"ish-root" withExtension:@"tar.gz"];
  if (archive == nil) {
    *errorOut = @"Bundled iSH Alpine root archive is missing";
    return NO;
  }
  struct fakefsify_error fakefsError = {};
  if (!fakefs_import(archive.fileSystemRepresentation, rootDirectory.fileSystemRepresentation,
                     &fakefsError, (struct progress){})) {
    NSString *message = fakefsError.message == NULL ? @"iSH root import failed"
                                                    : [NSString stringWithUTF8String:fakefsError.message];
    free(fakefsError.message);
    *errorOut = message;
    return NO;
  }
  ORTIshRootPath = rootDirectory.path;
  return YES;
}

/// Starts the iSH Linux kernel after its fakefs root has been prepared.
static BOOL ORTIshEnsureKernel(NSString **errorOut) {
  ORTIshInitializeState();
  [ORTIshStateLock lock];
  if (ORTIshKernelStarted) {
    [ORTIshStateLock unlock];
    return YES;
  }
  BOOL prepared = ORTIshPrepareRoot(errorOut);
  if (prepared) {
    actuate_kernel("");
    ORTIshKernelStarted = YES;
  }
  [ORTIshStateLock unlock];
  return prepared;
}

/// Starts an interactive Alpine shell and waits until iSH has attached its PTY.
static ORTIshTerminalSession *ORTIshStartSession(NSString *sessionName, NSString *workingDir,
                                                 NSInteger rows, NSInteger cols, NSString **errorOut) {
  if (!ORTIshEnsureKernel(errorOut)) {
    return nil;
  }
  ORTIshInitializeState();
  ORTIshTerminalSession *session = [ORTIshTerminalSession new];
  [ORTIshStateLock lock];
  session.sessionId = [NSString stringWithFormat:@"ios-ish-%llu", ORTIshNextSessionId++];
  [ORTIshStateLock unlock];
  session.sessionName = sessionName;
  session.workingDir = workingDir;
  session.rows = rows;
  session.cols = cols;

  [ORTIshStartLock lock];
  ORTIshPendingSession = session;
  const char *argv[] = {"/bin/sh", "-i", NULL};
  NSString *workingDirectoryVariable = [NSString stringWithFormat:@"OPERIT_WORKING_DIR=%@", workingDir];
  const char *environment[] = {
      "TERM=xterm-256color", "HOME=/root", "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
      workingDirectoryVariable.UTF8String, NULL};
  linux_start_session("/bin/sh", argv, environment, ^(int result, int pid, nsobj_t terminal) {
    @synchronized (session) {
      session.terminal = terminal;
      if (result != 0 || terminal == NULL) {
        session.exitCode = result;
        session.hasExitCode = YES;
        session.closed = YES;
      }
    }
    dispatch_semaphore_signal(session.startSignal);
  });
  long waitResult = dispatch_semaphore_wait(session.startSignal,
                                             dispatch_time(DISPATCH_TIME_NOW,
                                                           (int64_t)(ORTIshStartTimeout * NSEC_PER_SEC)));
  ORTIshPendingSession = nil;
  [ORTIshStartLock unlock];
  if (waitResult != 0) {
    *errorOut = @"iSH terminal did not start within 30 seconds";
    return nil;
  }
  @synchronized (session) {
    if (session.closed || session.terminal == NULL) {
      *errorOut = [NSString stringWithFormat:@"iSH shell launch failed with status %ld", (long)session.exitCode];
      return nil;
    }
  }
  if (!ORTIshResize(session, rows, cols, errorOut)) {
    return nil;
  }
  NSString *directoryCommand = [NSString stringWithFormat:@"cd -- \"$OPERIT_WORKING_DIR\"\n"];
  if (!ORTIshWrite(session, [directoryCommand dataUsingEncoding:NSUTF8StringEncoding], errorOut)) {
    return nil;
  }
  [ORTIshStateLock lock];
  ORTIshSessions[session.sessionId] = session;
  ORTIshSessionKeys[[NSString stringWithFormat:@"shell\n%@", sessionName]] = session.sessionId;
  [ORTIshStateLock unlock];
  return session;
}

/// Formats one session into the shared terminal list model.
static NSDictionary *ORTIshSessionEntry(ORTIshTerminalSession *session) {
  @synchronized (session) {
    return @{
      @"sessionId" : session.sessionId,
      @"sessionName" : session.sessionName,
      @"terminalType" : @"shell",
      @"sessionKind" : @"pty",
      @"workingDir" : session.workingDir,
      @"commandRunning" : @(session.commandRunning),
    };
  }
}

/// Executes one shell command and waits for its exact iSH completion marker.
static NSDictionary *ORTIshExecute(ORTIshTerminalSession *session, NSString *command,
                                   uint64_t timeoutMs, NSString **errorOut) {
  NSString *marker = [NSString stringWithFormat:@"__OPERIT_ISH_%@__", NSUUID.UUID.UUIDString];
  NSString *wrapped = [NSString stringWithFormat:@"%@\nprintf '\036%@:%%s\037' \"$?\"\n", command, marker];
  @synchronized (session) {
    if (session.commandRunning) {
      *errorOut = @"iSH terminal already has a command in progress";
      return nil;
    }
    session.commandRunning = YES;
    [session.pendingOutput setLength:0];
  }
  if (!ORTIshWrite(session, [wrapped dataUsingEncoding:NSUTF8StringEncoding], errorOut)) {
    @synchronized (session) {
      session.commandRunning = NO;
    }
    return nil;
  }
  NSMutableData *output = [NSMutableData data];
  NSData *markerData = [marker dataUsingEncoding:NSUTF8StringEncoding];
  NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:(NSTimeInterval)timeoutMs / 1000.0];
  for (;;) {
    [output appendData:ORTIshDrainOutput(session)];
    NSRange markerRange = [output rangeOfData:markerData options:0 range:NSMakeRange(0, output.length)];
    if (markerRange.location != NSNotFound) {
      NSUInteger statusStart = markerRange.location + markerRange.length + 1;
      const uint8_t *bytes = static_cast<const uint8_t *>(output.bytes);
      NSUInteger statusEnd = statusStart;
      while (statusEnd < output.length && bytes[statusEnd] != '\037') {
        statusEnd++;
      }
      if (statusEnd == output.length) {
        [NSThread sleepForTimeInterval:0.01];
        continue;
      }
      NSData *statusData = [output subdataWithRange:NSMakeRange(statusStart, statusEnd - statusStart)];
      NSString *statusText = ORTIshText(statusData, errorOut);
      NSInteger exitCode = statusText.integerValue;
      NSData *visibleData = [output subdataWithRange:NSMakeRange(0, markerRange.location)];
      NSString *visibleOutput = ORTIshText(visibleData, errorOut);
      @synchronized (session) {
        session.commandRunning = NO;
      }
      if (visibleOutput == nil || statusText == nil) {
        return nil;
      }
      return @{
        @"sessionId" : session.sessionId,
        @"terminalType" : @"shell",
        @"output" : visibleOutput,
        @"exitCode" : @(exitCode),
        @"timedOut" : @NO,
      };
    }
    if ([deadline timeIntervalSinceNow] <= 0) {
      NSString *interrupt = @"\003";
      ORTIshWrite(session, [interrupt dataUsingEncoding:NSUTF8StringEncoding], errorOut);
      NSString *visibleOutput = ORTIshText(output, errorOut);
      @synchronized (session) {
        session.commandRunning = NO;
      }
      if (visibleOutput == nil) {
        return nil;
      }
      return @{
        @"sessionId" : session.sessionId,
        @"terminalType" : @"shell",
        @"output" : visibleOutput,
        @"exitCode" : @124,
        @"timedOut" : @YES,
      };
    }
    [NSThread sleepForTimeInterval:0.01];
  }
}

/// Handles one typed iSH terminal bridge command.
static NSDictionary *ORTIshHandleCommand(NSString *command, NSDictionary *request) {
  NSString *error = nil;
  if ([command isEqualToString:@"terminalStart"]) {
    NSString *sessionName = ORTIshRequiredString(request, @"sessionName", &error);
    NSString *terminalType = ORTIshRequiredString(request, @"terminalType", &error);
    NSString *workingDir = ORTIshRequiredString(request, @"workingDir", &error);
    NSInteger rows = ORTIshPositiveDimension(request, @"rows", &error);
    NSInteger cols = ORTIshPositiveDimension(request, @"cols", &error);
    if (error != nil) return ORTIshError(error);
    if (![terminalType isEqualToString:@"shell"]) return ORTIshError(@"Unsupported iSH terminal type");
    ORTIshTerminalSession *session = ORTIshStartSession(sessionName, workingDir, rows, cols, &error);
    return session == nil ? ORTIshError(error) : ORTIshResult(@{ @"sessionId" : session.sessionId });
  }
  if ([command isEqualToString:@"terminalRead"]) {
    ORTIshTerminalSession *session = ORTIshSession(ORTIshRequiredString(request, @"sessionId", &error), &error);
    if (session == nil) return ORTIshError(error);
    NSString *output = ORTIshText(ORTIshDrainOutput(session), &error);
    return output == nil ? ORTIshError(error) : ORTIshResult(@{ @"output" : output });
  }
  if ([command isEqualToString:@"terminalWrite"]) {
    ORTIshTerminalSession *session = ORTIshSession(ORTIshRequiredString(request, @"sessionId", &error), &error);
    NSString *input = ORTIshRequiredString(request, @"input", &error);
    if (session == nil || input == nil) return ORTIshError(error);
    NSData *data = [input dataUsingEncoding:NSUTF8StringEncoding];
    return ORTIshWrite(session, data, &error) ? ORTIshResult(@{ @"acceptedChars" : @(input.length) }) : ORTIshError(error);
  }
  if ([command isEqualToString:@"terminalResize"]) {
    ORTIshTerminalSession *session = ORTIshSession(ORTIshRequiredString(request, @"sessionId", &error), &error);
    NSInteger rows = ORTIshPositiveDimension(request, @"rows", &error);
    NSInteger cols = ORTIshPositiveDimension(request, @"cols", &error);
    if (session == nil || error != nil) return ORTIshError(error);
    return ORTIshResize(session, rows, cols, &error) ? ORTIshResult(@{}) : ORTIshError(error);
  }
  if ([command isEqualToString:@"terminalPoll"]) {
    ORTIshTerminalSession *session = ORTIshSession(ORTIshRequiredString(request, @"sessionId", &error), &error);
    if (session == nil) return ORTIshError(error);
    @synchronized (session) {
      return ORTIshResult(@{ @"exitCode" : session.hasExitCode ? @(session.exitCode) : [NSNull null] });
    }
  }
  if ([command isEqualToString:@"terminalClose"]) {
    ORTIshTerminalSession *session = ORTIshSession(ORTIshRequiredString(request, @"sessionId", &error), &error);
    if (session == nil) return ORTIshError(error);
    @synchronized (session) {
      if (session.tty != NULL) {
        struct linux_tty *tty = session.tty;
        async_do_in_workqueue(^{ tty->ops->hangup(tty); });
      }
      session.closed = YES;
      session.exitCode = 0;
      session.hasExitCode = YES;
    }
    return ORTIshResult(@{});
  }
  if ([command isEqualToString:@"terminalList"]) {
    ORTIshInitializeState();
    [ORTIshStateLock lock];
    NSArray<ORTIshTerminalSession *> *sessions = ORTIshSessions.allValues;
    [ORTIshStateLock unlock];
    NSMutableArray *entries = [NSMutableArray arrayWithCapacity:sessions.count];
    for (ORTIshTerminalSession *session in sessions) [entries addObject:ORTIshSessionEntry(session)];
    return ORTIshResult(@{ @"sessions" : entries });
  }
  if ([command isEqualToString:@"terminalCreateOrGet"]) {
    NSString *sessionName = ORTIshRequiredString(request, @"sessionName", &error);
    NSString *terminalType = ORTIshRequiredString(request, @"terminalType", &error);
    if (error != nil) return ORTIshError(error);
    if (![terminalType isEqualToString:@"shell"]) return ORTIshError(@"Unsupported iSH terminal type");
    NSString *key = [NSString stringWithFormat:@"shell\n%@", sessionName];
    ORTIshInitializeState();
    [ORTIshStateLock lock];
    NSString *existingId = ORTIshSessionKeys[key];
    ORTIshTerminalSession *existing = ORTIshSessions[existingId];
    [ORTIshStateLock unlock];
    if (existing != nil) return ORTIshResult(@{ @"sessionId" : existing.sessionId, @"sessionName" : sessionName, @"terminalType" : @"shell", @"isNewSession" : @NO });
    ORTIshTerminalSession *session = ORTIshStartSession(sessionName, @"/", 24, 80, &error);
    return session == nil ? ORTIshError(error) : ORTIshResult(@{ @"sessionId" : session.sessionId, @"sessionName" : sessionName, @"terminalType" : @"shell", @"isNewSession" : @YES });
  }
  if ([command isEqualToString:@"terminalExecute"]) {
    ORTIshTerminalSession *session = ORTIshSession(ORTIshRequiredString(request, @"sessionId", &error), &error);
    NSString *shellCommand = ORTIshRequiredString(request, @"command", &error);
    NSNumber *timeout = request[@"timeoutMs"];
    if (session == nil || shellCommand == nil || ![timeout isKindOfClass:[NSNumber class]]) return ORTIshError(error ?: @"iSH terminal request has invalid timeoutMs");
    NSDictionary *result = ORTIshExecute(session, shellCommand, timeout.unsignedLongLongValue, &error);
    return result == nil ? ORTIshError(error) : ORTIshResult(result);
  }
  if ([command isEqualToString:@"terminalScreen"]) {
    ORTIshTerminalSession *session = ORTIshSession(ORTIshRequiredString(request, @"sessionId", &error), &error);
    if (session == nil) return ORTIshError(error);
    @synchronized (session) {
      NSString *content = ORTIshText(session.screenOutput, &error);
      if (content == nil) return ORTIshError(error);
      return ORTIshResult(@{ @"sessionId" : session.sessionId, @"terminalType" : @"shell", @"rows" : @(session.rows), @"cols" : @(session.cols), @"content" : content, @"commandRunning" : @(session.commandRunning) });
    }
  }
  return ORTIshError([NSString stringWithFormat:@"Unsupported iSH terminal command: %@", command]);
}

/// Handles one iSH terminal JSON request from the Rust host bridge.
extern "C" char *operit_ios_ish_terminal_call(const char *command, const char *request_json) {
  @autoreleasepool {
    if (command == NULL || request_json == NULL) return ORTIshEncodeResponse(ORTIshError(@"iSH terminal bridge request is missing"));
    NSError *jsonError = nil;
    NSData *requestData = [NSData dataWithBytes:request_json length:strlen(request_json)];
    id requestValue = [NSJSONSerialization JSONObjectWithData:requestData options:0 error:&jsonError];
    if (jsonError != nil || ![requestValue isKindOfClass:[NSDictionary class]]) return ORTIshEncodeResponse(ORTIshError(@"iSH terminal request is not a JSON object"));
    return ORTIshEncodeResponse(ORTIshHandleCommand([NSString stringWithUTF8String:command], requestValue));
  }
}

/// Frees one response string returned by the iSH terminal bridge.
extern "C" void operit_ios_ish_terminal_free(char *value) {
  free(value);
}

/// Retains one Objective-C object passed into iSH kernel state.
extern "C" nsobj_t objc_get(nsobj_t object) {
  return CFBridgingRetain((__bridge id)object);
}

/// Releases one Objective-C object retained for iSH kernel state.
extern "C" void objc_put(nsobj_t object) {
  CFBridgingRelease(object);
}

/// Creates the session object attached to a newly allocated iSH PTY.
extern "C" nsobj_t Terminal_terminalWithType_number(int type, int number) {
  ORTIshInitializeState();
  if (ORTIshPendingSession == nil) return NULL;
  return objc_get((__bridge nsobj_t)ORTIshPendingSession);
}

/// Records the iSH Linux TTY pointer required for input, resize, and close.
extern "C" void Terminal_setLinuxTTY(nsobj_t object, struct linux_tty *tty) {
  ORTIshTerminalSession *session = (__bridge ORTIshTerminalSession *)object;
  @synchronized (session) {
    session.tty = tty;
    if (tty == NULL) {
      session.closed = YES;
      if (!session.hasExitCode) {
        session.exitCode = 0;
        session.hasExitCode = YES;
      }
    }
  }
}

/// Appends raw iSH PTY output to the bridge-owned session buffers.
extern "C" int Terminal_sendOutput_length(nsobj_t object, const char *data, int size) {
  ORTIshTerminalSession *session = (__bridge ORTIshTerminalSession *)object;
  @synchronized (session) {
    if (size < 0 || session.pendingOutput.length + (NSUInteger)size > ORTIshOutputCapacity) return 0;
    NSData *output = [NSData dataWithBytes:data length:(NSUInteger)size];
    [session.pendingOutput appendData:output];
    [session.screenOutput appendData:output];
    return size;
  }
}

/// Reports the available iSH PTY output capacity before back-pressuring the kernel.
extern "C" int Terminal_roomForOutput(nsobj_t object) {
  ORTIshTerminalSession *session = (__bridge ORTIshTerminalSession *)object;
  @synchronized (session) {
    return (int)(ORTIshOutputCapacity - session.pendingOutput.length);
  }
}

/// Dispatches an iSH kernel callback onto the iOS main queue.
extern "C" void async_do_in_ios(void (^block)(void)) {
  dispatch_async(dispatch_get_main_queue(), block);
}

/// Executes one callback on the iSH work queue and waits for its completion callback.
extern "C" void sync_do_in_workqueue(void (^block)(void (^done)(void))) {
  dispatch_semaphore_t completion = dispatch_semaphore_create(0);
  async_do_in_workqueue(^{ block(^{ dispatch_semaphore_signal(completion); }); });
  dispatch_semaphore_wait(completion, DISPATCH_TIME_FOREVER);
}

/// Logs iSH kernel diagnostics through the iOS unified console.
extern "C" void ConsoleLog(const char *data, unsigned len) {
  NSLog(@"%.*s", (int)len, data);
}

/// Records an iSH kernel panic in the iOS unified console.
extern "C" void ReportPanic(const char *message) {
  NSLog(@"iSH kernel panic: %s", message);
}

/// Returns the initialized iSH fakefs root path to the Linux kernel.
extern "C" const char *DefaultRootPath(void) {
  return ORTIshRootPath.fileSystemRepresentation;
}

/// Performs no App-specific iSH filesystem migration in the embedded runtime.
extern "C" void FsInitialize(void) {
}

/// Returns the iOS pasteboard retained for iSH clipboard operations.
extern "C" nsobj_t UIPasteboard_get(void) {
  return objc_get((__bridge nsobj_t)UIPasteboard.generalPasteboard);
}

/// Returns the iOS pasteboard change counter for iSH clipboard operations.
extern "C" long UIPasteboard_changeCount(void) {
  return UIPasteboard.generalPasteboard.changeCount;
}

/// Replaces the iOS pasteboard UTF-8 content from iSH clipboard operations.
extern "C" void UIPasteboard_set(const char *data, size_t len) {
  UIPasteboard.generalPasteboard.string = [[NSString alloc] initWithBytes:data length:len encoding:NSUTF8StringEncoding];
}

/// Returns the byte length of one retained NSData object supplied to iSH.
extern "C" size_t NSData_length(nsobj_t data) {
  return [(__bridge NSData *)data length];
}

/// Returns the byte pointer of one retained NSData object supplied to iSH.
extern "C" const void *NSData_bytes(nsobj_t data) {
  return [(__bridge NSData *)data bytes];
}
