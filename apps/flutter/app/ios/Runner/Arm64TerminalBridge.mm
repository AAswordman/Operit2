#import <Foundation/Foundation.h>
#import <TargetConditionals.h>
#import "Arm64RootFilesystem.h"
#import "OperitFlutterBridge.h"

#include <operit/arm64_runtime.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

static const NSUInteger ORTOutputCapacity = 1024 * 1024;
static const NSUInteger ORTCommandCapacity = 16 * 1024 * 1024;
static const uint64_t ORTRunBudget = 16384;
static const int64_t ORTLinuxAgain = -11;
static const int ORTLinuxInterrupt = 2;

static NSDictionary *ORTResult(id value) { return @{ @"result": value ?: NSNull.null }; }
static NSDictionary *ORTError(NSString *error) {
  return @{ @"error": error ?: @"ARM64 terminal operation failed" };
}
static char *ORTEncode(NSDictionary *value) {
  NSData *data = [NSJSONSerialization dataWithJSONObject:value options:0 error:nullptr];
  if (data == nil) return strdup("{\"error\":\"ARM64 terminal response encoding failed\"}");
  char *result = static_cast<char *>(malloc(data.length + 1));
  if (result != nullptr) {
    memcpy(result, data.bytes, data.length);
    result[data.length] = 0;
  }
  return result;
}
static NSString *ORTString(NSDictionary *request, NSString *key, NSString **error) {
  id value = request[key];
  if (![value isKindOfClass:NSString.class] || [value length] == 0 ||
      [value rangeOfString:[NSString stringWithFormat:@"%C", (unichar)0]].location != NSNotFound) {
    *error = [NSString stringWithFormat:@"ARM64 terminal request has invalid %@", key];
    return nil;
  }
  return value;
}
static uint32_t ORTDimension(NSDictionary *request, NSString *key, NSString **error) {
  id value = request[key];
  if (![value isKindOfClass:NSNumber.class] || CFGetTypeID((__bridge CFTypeRef)value) == CFBooleanGetTypeID() ||
      [value doubleValue] != [value unsignedLongLongValue] || [value unsignedLongLongValue] == 0 ||
      [value unsignedLongLongValue] > UINT16_MAX) {
    *error = [NSString stringWithFormat:@"ARM64 terminal request has invalid %@", key];
    return 0;
  }
  return [value unsignedIntValue];
}
static NSArray<NSNumber *> *ORTBytes(NSData *data) {
  NSMutableArray *bytes = [NSMutableArray arrayWithCapacity:data.length];
  const uint8_t *source = static_cast<const uint8_t *>(data.bytes);
  for (NSUInteger i = 0; i < data.length; ++i) [bytes addObject:@(source[i])];
  return bytes;
}
static NSData *ORTInput(NSDictionary *request, NSString **error) {
  id input = request[@"bytes"];
  if (![input isKindOfClass:NSArray.class] || [input count] > ORTCommandCapacity) {
    *error = @"ARM64 terminal input must be a byte array of at most 16 MiB";
    return nil;
  }
  NSMutableData *data = [NSMutableData dataWithLength:[input count]];
  uint8_t *bytes = static_cast<uint8_t *>(data.mutableBytes);
  NSUInteger index = 0;
  for (id value in input) {
    if (![value isKindOfClass:NSNumber.class] || CFGetTypeID((__bridge CFTypeRef)value) == CFBooleanGetTypeID() ||
        [value doubleValue] != [value unsignedLongLongValue] || [value unsignedLongLongValue] > 255) {
      *error = @"ARM64 terminal input contains an invalid byte";
      return nil;
    }
    bytes[index++] = [value unsignedCharValue];
  }
  return data;
}
static NSString *ORTQuote(NSString *value) {
  return [NSString stringWithFormat:@"'%@'", [value stringByReplacingOccurrencesOfString:@"'" withString:@"'\"'\"'"]];
}
static BOOL ORTPathContains(NSString *parent, NSString *path) {
  return [parent isEqualToString:path] || [path hasPrefix:[parent stringByAppendingString:@"/"]];
}
static NSString *ORTRuntimeError(ort_arm64_vm *vm, NSString *operation) {
  const char *message = ort_arm64_last_error(vm);
  NSString *detail = message && *message ? [NSString stringWithUTF8String:message] : @"runtime returned failure";
  return [NSString stringWithFormat:@"ARM64 %@: %@", operation, detail ?: @"invalid runtime diagnostic"];
}

@interface ORTArm64Binding : NSObject
@property(nonatomic, strong) ORTArm64DirectoryAccess *directory;
@property(nonatomic, copy) NSString *mountPoint;
@property(nonatomic, assign) BOOL readOnly;
@property(nonatomic, assign) BOOL automatic;
@property(nonatomic, assign) BOOL workspace;
@end
@implementation ORTArm64Binding
@end

// Immutable after publication. Sessions retain their snapshot, including all
// security scopes, until their VM and command files have been released.
@interface ORTArm64RootConfiguration : NSObject
@property(nonatomic, strong) ORTArm64DirectoryAccess *root;
@property(nonatomic, copy) NSString *source;
@property(nonatomic, copy) NSString *identifier;
@property(nonatomic, copy) NSArray<ORTArm64Binding *> *mounts;
@property(nonatomic, assign) BOOL bindAppSandbox;
@end
@implementation ORTArm64RootConfiguration
@end

@interface ORTArm64Execution : NSObject
@property(nonatomic, copy) NSString *identifier;
@property(nonatomic, strong) NSMutableData *frames;
@property(nonatomic, strong) dispatch_semaphore_t changed;
@property(nonatomic, assign) NSUInteger outputCount;
@property(nonatomic, assign) BOOL started;
@property(nonatomic, assign) BOOL done;
@property(nonatomic, assign) BOOL abandoned;
@property(nonatomic, assign) NSInteger exitCode;
@property(nonatomic, assign) NSTimeInterval cancelDeadline;
@property(nonatomic, copy) NSString *error;
@property(nonatomic, assign) int scriptDirectory;
@property(nonatomic, copy) NSString *scriptName;
- (void)removeScript;
@end
@implementation ORTArm64Execution
- (instancetype)init {
  if ((self = [super init])) {
    _identifier = NSUUID.UUID.UUIDString;
    _scriptDirectory = -1;
    _frames = [NSMutableData data];
    _changed = dispatch_semaphore_create(0);
  }
  return self;
}
- (void)removeScript {
  if (_scriptDirectory >= 0) {
    if (_scriptName) unlinkat(_scriptDirectory, _scriptName.fileSystemRepresentation, 0);
    close(_scriptDirectory);
    _scriptDirectory = -1;
  }
}
- (void)dealloc { [self removeScript]; }
@end

// Mutable session state, including every VM API call, belongs to queue.
@interface ORTArm64TerminalSession : NSObject
@property(nonatomic, copy) NSString *sessionId;
@property(nonatomic, copy) NSString *sessionName;
@property(nonatomic, copy) NSString *workingDir;
@property(nonatomic, copy) NSString *rootPath;
@property(nonatomic, strong) ORTArm64RootConfiguration *configuration;
@property(nonatomic, copy) NSString *configurationId;
@property(nonatomic, strong) dispatch_queue_t queue;
@property(nonatomic, strong) dispatch_semaphore_t startup;
@property(nonatomic, assign) ort_arm64_vm *vm;
@property(nonatomic, assign) BOOL terminalOpen;
@property(nonatomic, assign) BOOL pumpScheduled;
@property(nonatomic, assign) BOOL outputDrained;
@property(nonatomic, assign) BOOL ptyHangup;
@property(nonatomic, assign) BOOL ready;
@property(nonatomic, assign) BOOL closed;
@property(nonatomic, assign) BOOL hasExitCode;
@property(nonatomic, assign) NSInteger exitCode;
@property(nonatomic, assign) uint32_t rows;
@property(nonatomic, assign) uint32_t cols;
@property(nonatomic, assign) BOOL commandRunning;
@property(nonatomic, copy) NSString *failure;
@property(nonatomic, copy) NSString *protocolToken;
@property(nonatomic, strong) NSData *protocolPrefix;
@property(nonatomic, strong) NSMutableData *protocolBuffer;
@property(nonatomic, strong) NSMutableData *pendingOutput;
@property(nonatomic, strong) NSMutableData *screenOutput;
@property(nonatomic, strong) NSMutableData *queuedInput;
@property(nonatomic, strong) ORTArm64Execution *execution;
- (void)schedulePump:(NSTimeInterval)delay;
- (void)pump;
- (void)receive:(NSData *)data;
- (void)publish:(NSData *)data;
- (void)fail:(NSString *)error;
- (NSString *)close;
- (NSString *)interrupt;
- (void)releaseVM;
@end

@implementation ORTArm64TerminalSession
- (instancetype)init {
  if ((self = [super init])) {
#if TARGET_OS_OSX
    _sessionId = [@"macos-arm64-" stringByAppendingString:NSUUID.UUID.UUIDString];
#else
    _sessionId = [@"ios-arm64-" stringByAppendingString:NSUUID.UUID.UUIDString];
#endif
    _queue = dispatch_queue_create(_sessionId.UTF8String, DISPATCH_QUEUE_SERIAL);
    _startup = dispatch_semaphore_create(0);
    _protocolToken = NSUUID.UUID.UUIDString;
    _protocolPrefix = [[NSString stringWithFormat:@"\036%@:", _protocolToken] dataUsingEncoding:NSUTF8StringEncoding];
    _protocolBuffer = [NSMutableData data];
    _pendingOutput = [NSMutableData data];
    _screenOutput = [NSMutableData data];
    _queuedInput = [NSMutableData data];
  }
  return self;
}
- (void)dealloc {
  // Normally releaseVM already ran on queue. Preserve queue ownership even if
  // an unfinished startup is released by its caller.
  ort_arm64_vm *vm = _vm;
  BOOL terminalOpen = _terminalOpen;
  ORTArm64RootConfiguration *configuration = _configuration;
  ORTArm64Execution *execution = _execution;
  if (vm) dispatch_async(_queue, ^{
    if (terminalOpen) ort_arm64_terminal_close(vm);
    ort_arm64_destroy(vm);
    [execution removeScript];
    // Keep sandbox grants alive through destroy, even on unfinished startup.
    (void)[configuration class];
  });
}
- (void)releaseVM {
  if (self.vm) {
    if (self.terminalOpen) {
      if (ort_arm64_terminal_close(self.vm) != 0 && self.failure == nil)
        self.failure = ORTRuntimeError(self.vm, @"terminal close failed");
      self.terminalOpen = NO;
    }
    ort_arm64_destroy(self.vm);
    self.vm = nullptr;
  }
  self.closed = YES;
  [self.execution removeScript];
  self.configuration = nil;
  [self.queuedInput setLength:0];
  dispatch_semaphore_signal(self.startup);
  if (self.execution) dispatch_semaphore_signal(self.execution.changed);
}
- (void)fail:(NSString *)error {
  self.failure = error;
  self.commandRunning = NO;
  if (self.execution && !self.execution.done) {
    self.execution.error = error;
    self.execution.done = YES;
  }
  [self releaseVM];
}
- (NSString *)close {
  NSString *error = nil;
  if (self.vm && self.terminalOpen) {
    if (ort_arm64_terminal_close(self.vm) != 0) error = ORTRuntimeError(self.vm, @"terminal close failed");
    self.terminalOpen = NO;
  }
  if (self.execution && !self.execution.done) {
    self.execution.error = error ?: @"ARM64 terminal closed before the command completed";
    self.execution.done = YES;
  }
  self.commandRunning = NO;
  if (error && !self.failure) self.failure = error;
  [self releaseVM];
  return error;
}
- (NSString *)interrupt {
  if (!self.vm || self.closed) return self.failure ?: @"ARM64 terminal session is not running";
  if (ort_arm64_terminal_signal(self.vm, ORTLinuxInterrupt) != 0)
    return ORTRuntimeError(self.vm, @"terminal interrupt failed");
  [self schedulePump:0];
  return nil;
}
- (void)schedulePump:(NSTimeInterval)delay {
  if (self.pumpScheduled || !self.vm || self.closed) return;
  self.pumpScheduled = YES;
  __weak ORTArm64TerminalSession *weakSelf = self;
  dispatch_block_t step = ^{
    ORTArm64TerminalSession *session = weakSelf;
    session.pumpScheduled = NO;
    [session pump];
  };
  if (delay > 0)
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(delay * NSEC_PER_SEC)), self.queue, step);
  else
    dispatch_async(self.queue, step);
}
- (void)publish:(NSData *)data {
  if (data.length == 0) return;
  ORTArm64Execution *execution = self.execution;
  // The command line and prompt preceding its begin record are PTY echo of the
  // protocol wrapper, rather than command output.
  if (execution && !execution.started && !execution.done) return;
  for (NSMutableData *buffer in @[self.screenOutput, self.pendingOutput]) {
    [buffer appendData:data];
    if (buffer.length > ORTOutputCapacity)
      [buffer replaceBytesInRange:NSMakeRange(0, buffer.length - ORTOutputCapacity) withBytes:nullptr length:0];
  }
  if (execution && execution.started && !execution.done) {
    if (data.length > ORTCommandCapacity - execution.outputCount) {
      [self fail:@"ARM64 command output exceeded the 16 MiB capture limit"];
      return;
    }
    execution.outputCount += data.length;
    [execution.frames appendData:data];
    dispatch_semaphore_signal(execution.changed);
  }
}
- (void)record:(NSString *)record {
  if ([record hasPrefix:@"prompt:"]) {
    NSString *cwd = [record substringFromIndex:7];
    if ([cwd hasPrefix:@"/"]) self.workingDir = cwd;
    if (!self.ready) {
      self.ready = YES;
      dispatch_semaphore_signal(self.startup);
    }
    ORTArm64Execution *execution = self.execution;
    if (execution && !execution.done && execution.started) {
      // SIGINT can abort the entire submitted line before its status printf.
      execution.error = @"ARM64 command returned to the prompt without an exit-status record";
      execution.done = YES;
      dispatch_semaphore_signal(execution.changed);
      if (execution.abandoned) self.execution = nil;
    }
    self.commandRunning = self.execution != nil && !self.execution.done;
    return;
  }
  ORTArm64Execution *execution = self.execution;
  if (!execution) return;
  if ([record isEqualToString:[@"begin:" stringByAppendingString:execution.identifier]]) {
    execution.started = YES;
    return;
  }
  NSString *prefix = [NSString stringWithFormat:@"done:%@:", execution.identifier];
  if (![record hasPrefix:prefix]) return;
  NSString *value = [record substringFromIndex:prefix.length];
  NSRange separator = [value rangeOfString:@":"];
  if (separator.location == NSNotFound) {
    [self fail:@"ARM64 command emitted an invalid exit-status record"];
    return;
  }
  NSString *status = [value substringToIndex:separator.location];
  NSScanner *scanner = [NSScanner scannerWithString:status];
  int code = 0;
  if (status.length == 0 || ![scanner scanInt:&code] || !scanner.isAtEnd || code < 0 || code > 255) {
    [self fail:@"ARM64 command emitted an invalid exit status"];
    return;
  }
  NSString *cwd = [value substringFromIndex:separator.location + 1];
  if ([cwd hasPrefix:@"/"]) self.workingDir = cwd;
  execution.exitCode = code;
  execution.done = YES;
  [execution removeScript];
  self.commandRunning = NO;
  dispatch_semaphore_signal(execution.changed);
  if (execution.abandoned) self.execution = nil;
}
- (void)receive:(NSData *)data {
  [self.protocolBuffer appendData:data];
  while (self.protocolBuffer.length && !self.closed) {
    NSRange marker = [self.protocolBuffer rangeOfData:self.protocolPrefix options:0
                                               range:NSMakeRange(0, self.protocolBuffer.length)];
    if (marker.location == NSNotFound) {
      // Retain only the actual possible prefix suffix, so short output is live.
      NSUInteger keep = MIN(self.protocolPrefix.length - 1, self.protocolBuffer.length);
      const uint8_t *bytes = static_cast<const uint8_t *>(self.protocolBuffer.bytes);
      while (keep && memcmp(bytes + self.protocolBuffer.length - keep, self.protocolPrefix.bytes, keep) != 0) --keep;
      NSUInteger count = self.protocolBuffer.length - keep;
      [self publish:[self.protocolBuffer subdataWithRange:NSMakeRange(0, count)]];
      [self.protocolBuffer replaceBytesInRange:NSMakeRange(0, count) withBytes:nullptr length:0];
      return;
    }
    if (marker.location) {
      [self publish:[self.protocolBuffer subdataWithRange:NSMakeRange(0, marker.location)]];
      [self.protocolBuffer replaceBytesInRange:NSMakeRange(0, marker.location) withBytes:nullptr length:0];
      if (self.closed) return;
    }
    const uint8_t *bytes = static_cast<const uint8_t *>(self.protocolBuffer.bytes);
    NSUInteger end = self.protocolPrefix.length;
    while (end < self.protocolBuffer.length && bytes[end] != '\037') ++end;
    if (end == self.protocolBuffer.length) {
      if (end > 65536) [self fail:@"ARM64 terminal protocol record exceeded its size limit"];
      return;
    }
    NSString *record = [[NSString alloc] initWithBytes:bytes + self.protocolPrefix.length
                                              length:end - self.protocolPrefix.length encoding:NSUTF8StringEncoding];
    [self.protocolBuffer replaceBytesInRange:NSMakeRange(0, end + 1) withBytes:nullptr length:0];
    if (record == nil) {
      [self fail:@"ARM64 terminal protocol record is not UTF-8"];
      return;
    }
    [self record:record];
  }
}
- (void)readOutput {
  self.outputDrained = NO;
  uint8_t bytes[8192];
  NSUInteger budget = 64 * 1024;
  while (self.vm && budget) {
    BOOL capturing = self.execution && !self.execution.done;
    if (!capturing && self.pendingOutput.length >= ORTOutputCapacity) return;
    NSUInteger count = MIN(sizeof(bytes), budget);
    if (!capturing) count = MIN(count, ORTOutputCapacity - self.pendingOutput.length);
    int64_t size = ort_arm64_terminal_read(self.vm, bytes, count);
    if (size == ORTLinuxAgain || size == 0) { self.outputDrained = YES; return; }
    // A PTY master reports EIO after its final slave closes. Obtain the real
    // process status from run before classifying that hangup as a failure.
    if (size == -5) { self.ptyHangup = YES; self.outputDrained = YES; return; }
    if (size < 0 || (uint64_t)size > count) {
      [self fail:[NSString stringWithFormat:@"ARM64 terminal read failed (Linux errno %lld)", (long long)-size]];
      return;
    }
    budget -= (NSUInteger)size;
    [self receive:[NSData dataWithBytes:bytes length:(NSUInteger)size]];
  }
}
- (void)pump {
  @autoreleasepool {
    if (!self.vm || self.closed) return;
    if (self.execution.abandoned && NSProcessInfo.processInfo.systemUptime >= self.execution.cancelDeadline) {
      [self fail:@"ARM64 command did not stop after SIGINT; its VM was closed"];
      return;
    }
    if (self.queuedInput.length) {
      int64_t size = ort_arm64_terminal_write(self.vm, self.queuedInput.bytes, MIN(self.queuedInput.length, (NSUInteger)4096));
      if (size < 0 && size != ORTLinuxAgain) {
        [self fail:[NSString stringWithFormat:@"ARM64 terminal write failed (Linux errno %lld)", (long long)-size]];
        return;
      }
      if (size > 0) {
        if ((uint64_t)size > MIN(self.queuedInput.length, (NSUInteger)4096)) {
          [self fail:@"ARM64 terminal write returned an invalid byte count"];
          return;
        }
        [self.queuedInput replaceBytesInRange:NSMakeRange(0, (NSUInteger)size) withBytes:nullptr length:0];
      }
    }
    [self readOutput];
    if (!self.vm) return;
    if (!self.execution && self.pendingOutput.length >= ORTOutputCapacity) {
      [self schedulePump:0.01];
      return;
    }
    ort_arm64_result result = ort_arm64_run(self.vm, ORTRunBudget);
    NSString *runFailure = nil;
    if (result.status == ORT_ARM64_FAULT || result.status == ORT_ARM64_UNSUPPORTED) {
      NSString *operation = [NSString stringWithFormat:@"guest stopped at 0x%llx (status %d, syscall %llu)",
                            (unsigned long long)result.pc, (int)result.status,
                            (unsigned long long)result.last_unsupported_syscall];
      // Byte I/O also updates last_error, so retain the run diagnostic first.
      runFailure = ORTRuntimeError(self.vm, operation);
    }
    [self readOutput];
    if (!self.vm) return;
    if (result.status == ORT_ARM64_EXITED) {
      // Output read per pump is bounded; retain the VM until all final PTY
      // bytes have been drained, even if the reader is applying backpressure.
      if (!self.outputDrained) {
        [self schedulePump:0.01];
        return;
      }
      if (self.protocolBuffer.length) {
        NSData *tail = [self.protocolBuffer copy];
        [self.protocolBuffer setLength:0];
        [self publish:tail];
      }
      self.hasExitCode = YES;
      self.exitCode = result.exit_code;
      self.commandRunning = NO;
      if (self.execution && !self.execution.done) {
        if (self.execution.started) self.execution.exitCode = result.exit_code;
        else self.execution.error = @"ARM64 shell exited before executing the command";
        self.execution.done = YES;
      }
      if (!self.ready) self.failure = [NSString stringWithFormat:@"ARM64 shell exited during startup (%d)", result.exit_code];
      [self releaseVM];
      return;
    }
    if (runFailure) {
      [self fail:runFailure];
      return;
    }
    if (self.ptyHangup) {
      [self fail:@"ARM64 PTY hung up before a guest exit status was available"];
      return;
    }
    if (result.status != ORT_ARM64_BUDGET && result.status != ORT_ARM64_WAITING) {
      [self fail:@"ARM64 runtime returned an unknown execution status"];
      return;
    }
    [self schedulePump:result.status == ORT_ARM64_WAITING ? 0.005 : 0];
  }
}
@end

// The registry queue serializes root preparation, mount configuration and
// create-or-get. It is never held by a session pump or an output callback.
static dispatch_queue_t ORTRegistryQueue;
static NSMutableDictionary<NSString *, ORTArm64TerminalSession *> *ORTSessions;
static NSMutableDictionary<NSString *, NSString *> *ORTSessionNames;
static ORTArm64RootConfiguration *ORTConfiguration;
// A failed startup restore must not silently start a different, bundled root.
static NSString *ORTInitialConfigurationError;
static void ORTInitialize(void) {
  static dispatch_once_t once;
  dispatch_once(&once, ^{
    ORTRegistryQueue = dispatch_queue_create("operit.arm64.terminal.registry", DISPATCH_QUEUE_SERIAL);
    ORTSessions = [NSMutableDictionary dictionary];
    ORTSessionNames = [NSMutableDictionary dictionary];
  });
}
static NSString *ORTPOSIXError(NSString *operation, NSString *path, int code) {
  return [NSString stringWithFormat:@"ARM64 %@ (%@): %s (POSIX errno %d)", operation, path, strerror(code), code];
}
static BOOL ORTBoolean(NSDictionary *request, NSString *key, BOOL fallback, NSString **error) {
  id value = request[key];
  if (!value) return fallback;
  if (![value isKindOfClass:NSNumber.class] || CFGetTypeID((__bridge CFTypeRef)value) != CFBooleanGetTypeID()) {
    *error = [NSString stringWithFormat:@"ARM64 runtime configuration has invalid %@; expected a boolean", key];
    return fallback;
  }
  return [value boolValue];
}
static ORTArm64DirectoryAccess *ORTDirectory(NSDictionary *request, NSString *pathKey, NSString *bookmarkKey,
                                            NSString **error) {
  id path = request[pathKey];
  NSURL *URL = nil;
  if ([path isKindOfClass:NSURL.class]) URL = path;
  else if ([path isKindOfClass:NSString.class] && [path length] &&
           [path rangeOfString:[NSString stringWithFormat:@"%C", (unichar)0]].location == NSNotFound) {
    URL = [path hasPrefix:@"file:"] ? [NSURL URLWithString:path] :
        ([path isAbsolutePath] ? [NSURL fileURLWithPath:path isDirectory:YES] : nil);
  }
  if (path && (!URL.isFileURL || !URL.path.isAbsolutePath ||
      (URL.host.length && ![URL.host isEqualToString:@"localhost"]))) {
    *error = [NSString stringWithFormat:@"ARM64 %@ must be an absolute directory path or file URL", pathKey];
    return nil;
  }
  id value = request[bookmarkKey];
  NSData *bookmark = nil;
  if ([value isKindOfClass:NSData.class]) bookmark = value;
  else if ([value isKindOfClass:NSString.class] && [value length] <= 2 * 1024 * 1024)
    bookmark = [[NSData alloc] initWithBase64EncodedString:value options:0];
  if (value && (!bookmark.length || bookmark.length > 1024 * 1024)) {
    *error = [NSString stringWithFormat:@"ARM64 %@ must contain valid directory bookmark data", bookmarkKey];
    return nil;
  }
  if (!URL && !bookmark) {
    *error = [NSString stringWithFormat:@"ARM64 directory requires %@ or %@", pathKey, bookmarkKey];
    return nil;
  }
  return [ORTArm64DirectoryAccess accessURL:URL bookmarkData:bookmark error:error];
}
static NSString *ORTGuestPath(NSDictionary *request, NSString *key, NSString **error) {
  NSString *path = ORTString(request, key, error);
  if (!path) return nil;
  if (![path hasPrefix:@"/"] || [path isEqualToString:@"/"]) {
    *error = [NSString stringWithFormat:@"ARM64 %@ must be an absolute guest path below /", key];
    return nil;
  }
  NSMutableArray *components = [NSMutableArray array];
  for (NSString *component in path.pathComponents) {
    if ([component isEqualToString:@"/"]) continue;
    if ([component isEqualToString:@"."] || [component isEqualToString:@".."] ||
        [component lengthOfBytesUsingEncoding:NSUTF8StringEncoding] > NAME_MAX) {
      *error = [NSString stringWithFormat:@"ARM64 %@ contains an invalid guest path component", key];
      return nil;
    }
    [components addObject:component];
  }
  NSString *normalized = [@"/" stringByAppendingString:[components componentsJoinedByString:@"/"]];
  if (![path isEqualToString:normalized] || [path lengthOfBytesUsingEncoding:NSUTF8StringEncoding] >= PATH_MAX) {
    *error = [NSString stringWithFormat:@"ARM64 %@ must be a normalized absolute guest path", key];
    return nil;
  }
  return normalized;
}
static ORTArm64Binding *ORTBinding(NSDictionary *request, NSString **error) {
  ORTArm64Binding *binding = [ORTArm64Binding new];
  binding.mountPoint = ORTGuestPath(request, @"mountPoint", error);
  if (!binding.mountPoint) return nil;
  binding.readOnly = ORTBoolean(request, @"readOnly", NO, error);
  if (*error) return nil;
  binding.directory = ORTDirectory(request, @"hostDirectory", @"bookmark", error);
  return binding.directory ? binding : nil;
}
static BOOL ORTSameDirectory(ORTArm64DirectoryAccess *a, ORTArm64DirectoryAccess *b) {
  return [a.path isEqualToString:b.path] && [a.identity isEqualToString:b.identity];
}
static BOOL ORTSameBinding(ORTArm64Binding *a, ORTArm64Binding *b) {
  return [a.mountPoint isEqualToString:b.mountPoint] && a.readOnly == b.readOnly &&
      ORTSameDirectory(a.directory, b.directory);
}
static NSArray<ORTArm64Binding *> *ORTSortedBindings(NSArray<ORTArm64Binding *> *bindings) {
  return [bindings sortedArrayUsingComparator:^NSComparisonResult(ORTArm64Binding *a, ORTArm64Binding *b) {
    if (a.mountPoint.length < b.mountPoint.length) return NSOrderedAscending;
    if (a.mountPoint.length > b.mountPoint.length) return NSOrderedDescending;
    return [a.mountPoint compare:b.mountPoint];
  }];
}
static BOOL ORTSameConfiguration(ORTArm64RootConfiguration *a, ORTArm64RootConfiguration *b) {
  if (!a || !ORTSameDirectory(a.root, b.root) || ![a.source isEqualToString:b.source] ||
      a.bindAppSandbox != b.bindAppSandbox || a.mounts.count != b.mounts.count) return NO;
  for (NSUInteger i = 0; i < a.mounts.count; ++i)
    if (!ORTSameBinding(a.mounts[i], b.mounts[i])) return NO;
  return YES;
}
// Find the backing directory in the same namespace/order that bind() will use.
// This also makes command scripts honor an explicit /tmp bind.
static ORTArm64DirectoryAccess *ORTBackingDirectory(ORTArm64RootConfiguration *configuration,
    NSArray<ORTArm64Binding *> *mounts, NSString *guest, NSString **relative, BOOL *readOnly) {
  ORTArm64DirectoryAccess *directory = configuration.root;
  NSString *prefix = @"/";
  *readOnly = NO;
  for (ORTArm64Binding *binding in mounts) {
    if (ORTPathContains(binding.mountPoint, guest) && binding.mountPoint.length >= prefix.length) {
      directory = binding.directory;
      prefix = binding.mountPoint;
      *readOnly = binding.readOnly;
    }
  }
  *relative = [guest isEqualToString:prefix] ? @"" :
      [guest substringFromIndex:[prefix isEqualToString:@"/"] ? 1 : prefix.length + 1];
  return directory;
}
static int ORTOpenGuestDirectory(ORTArm64RootConfiguration *configuration, NSArray<ORTArm64Binding *> *mounts,
                                 NSString *guest, BOOL create, BOOL requireWritable, NSString **error) {
  NSString *relative = nil;
  BOOL readOnly = NO;
  ORTArm64DirectoryAccess *backing = ORTBackingDirectory(configuration, mounts, guest, &relative, &readOnly);
  if (requireWritable && readOnly) {
    *error = [NSString stringWithFormat:@"ARM64 guest directory is mounted read-only: %@", guest];
    return -1;
  }
  int current = openat(backing.descriptor, ".", O_RDONLY | O_DIRECTORY | O_CLOEXEC | O_NOFOLLOW);
  if (current < 0) { *error = ORTPOSIXError(@"open backing directory", backing.path, errno); return -1; }
  for (NSString *component in relative.pathComponents) {
    const char *name = component.fileSystemRepresentation;
    int next = openat(current, name, O_RDONLY | O_DIRECTORY | O_CLOEXEC | O_NOFOLLOW);
    if (next < 0 && errno == ENOENT && create && !readOnly) {
      if (mkdirat(current, name, 0755) < 0 && errno != EEXIST) {
        *error = ORTPOSIXError(@"create mount parent", guest, errno);
        close(current);
        return -1;
      }
      next = openat(current, name, O_RDONLY | O_DIRECTORY | O_CLOEXEC | O_NOFOLLOW);
    }
    int saved = errno;
    close(current);
    if (next < 0) { *error = ORTPOSIXError(@"open guest directory without symlinks", guest, saved); return -1; }
    current = next;
  }
  return current;
}
static BOOL ORTCheckDirectories(ORTArm64RootConfiguration *configuration, NSString **error) {
  if (![configuration.root checkAvailability:error]) return NO;
  for (ORTArm64Binding *binding in configuration.mounts)
    if (![binding.directory checkAvailability:error]) return NO;
  return YES;
}
static BOOL ORTApplyBindings(ort_arm64_vm *vm, ORTArm64RootConfiguration *configuration, NSString **error) {
  for (ORTArm64Binding *binding in configuration.mounts) {
    if (ort_arm64_bind(vm, binding.directory.path.fileSystemRepresentation, binding.mountPoint.fileSystemRepresentation,
                       binding.readOnly ? ORT_ARM64_BIND_READ_ONLY : 0) != 0) {
      *error = ORTRuntimeError(vm, [NSString stringWithFormat:@"bind %@ to %@ failed", binding.directory.path, binding.mountPoint]);
      return NO;
    }
  }
  return YES;
}
static ort_arm64_vm *ORTCreateVM(ORTArm64RootConfiguration *configuration, NSString **error) {
  if (!ORTCheckDirectories(configuration, error)) return nullptr;
  ort_arm64_config config = {};
  config.root_directory = configuration.root.path.fileSystemRepresentation;
  config.resident_memory_limit = 512ULL * 1024 * 1024;
  config.enable_fixed_acceleration = 1;
  ort_arm64_vm *vm = ort_arm64_create(&config);
  if (!vm) { *error = ORTRuntimeError(nullptr, @"VM creation failed"); return nullptr; }
  if (!ORTApplyBindings(vm, configuration, error) || !ORTCheckDirectories(configuration, error)) {
    ort_arm64_destroy(vm);
    return nullptr;
  }
  return vm;
}
static BOOL ORTValidateConfiguration(ORTArm64RootConfiguration *configuration, NSString **error) {
  if (!ORTCheckDirectories(configuration, error)) return NO;
  NSMutableArray *previous = [NSMutableArray array];
  for (ORTArm64Binding *binding in configuration.mounts) {
    int parent = ORTOpenGuestDirectory(configuration, previous, binding.mountPoint.stringByDeletingLastPathComponent,
                                       YES, NO, error);
    if (parent < 0) return NO;
    close(parent);
    [previous addObject:binding];
  }
  // A disposable VM checks runtime mount semantics before disturbing live VMs.
  // No guest instructions run and no archive/root is copied here.
  ort_arm64_vm *probe = ORTCreateVM(configuration, error);
  if (!probe) return NO;
  const char *argv[] = {"/bin/sh", nullptr};
  const char *envp[] = {"PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin", nullptr};
  BOOL valid = ort_arm64_load(probe, "/bin/sh", argv, envp) == 0;
  if (!valid) *error = ORTRuntimeError(probe, @"validate rootfs shell/ELF interpreter failed");
  ort_arm64_destroy(probe);
  return valid && ORTCheckDirectories(configuration, error);
}
static NSDictionary *ORTConfigurationSnapshot(ORTArm64RootConfiguration *configuration) {
  if (!configuration) return ORTInitialConfigurationError ?
      @{ @"configured": @NO, @"configurationError": ORTInitialConfigurationError } : @{ @"configured": @NO };
  NSMutableDictionary *result = [@{ @"configured": @YES, @"configurationId": configuration.identifier,
    @"rootDirectory": configuration.root.path, @"rootSource": configuration.source,
    @"bindAppSandbox": @(configuration.bindAppSandbox) } mutableCopy];
  NSString *error = nil;
  NSData *bookmark = [configuration.root bookmarkWithError:&error];
  if (bookmark) result[@"rootBookmark"] = [bookmark base64EncodedStringWithOptions:0];
  else if (error) result[@"rootBookmarkError"] = error;
  NSMutableArray *mounts = [NSMutableArray array];
  for (ORTArm64Binding *binding in configuration.mounts) {
    NSMutableDictionary *entry = [@{ @"hostDirectory": binding.directory.path, @"mountPoint": binding.mountPoint,
      @"readOnly": @(binding.readOnly), @"automatic": @(binding.automatic) } mutableCopy];
    NSData *data = [binding.directory bookmarkWithError:&error];
    if (data) entry[@"bookmark"] = [data base64EncodedStringWithOptions:0];
    else if (error) entry[@"bookmarkError"] = error;
    [mounts addObject:entry];
  }
  result[@"mounts"] = mounts;
  return result;
}
static ORTArm64RootConfiguration *ORTBuildConfiguration(NSDictionary *request, NSString **error) {
  ORTArm64RootConfiguration *configuration = [ORTArm64RootConfiguration new];
  id source = request[@"rootSource"];
  if (source && (![source isKindOfClass:NSString.class] ||
                 (![source isEqualToString:@"external"] && ![source isEqualToString:@"bundled"]))) {
    *error = @"ARM64 rootSource must be external or bundled";
    return nil;
  }
  BOOL external = source ? [source isEqualToString:@"external"] :
      (request[@"rootDirectory"] != nil || request[@"rootBookmark"] != nil);
  configuration.source = external ? @"external" : @"bundled";
  configuration.bindAppSandbox = ORTBoolean(request, @"bindAppSandbox", !external, error);
  if (*error) return nil;
  if (external) configuration.root = ORTDirectory(request, @"rootDirectory", @"rootBookmark", error);
  else {
    NSURL *support = [NSFileManager.defaultManager URLsForDirectory:NSApplicationSupportDirectory inDomains:NSUserDomainMask].firstObject;
    if (!support) { *error = @"iOS application support directory is unavailable"; return nil; }
    NSURL *archive = [NSBundle.mainBundle URLForResource:@"arm64-rootfs" withExtension:@"tar.gz"];
    NSString *root = ORTArm64PrepareRootAtURL([support URLByAppendingPathComponent:@"operit-arm64"], archive, error);
    if (root) configuration.root = [ORTArm64DirectoryAccess accessURL:[NSURL fileURLWithPath:root isDirectory:YES]
                                                       bookmarkData:nil error:error];
  }
  if (!configuration.root) return nil;
  id values = request[@"mounts"] ?: @[];
  if (![values isKindOfClass:NSArray.class] || [values count] > 128) {
    *error = @"ARM64 mounts must be an array of at most 128 directory bindings";
    return nil;
  }
  NSMutableArray<ORTArm64Binding *> *mounts = [NSMutableArray array];
  for (id value in values) {
    if (![value isKindOfClass:NSDictionary.class]) { *error = @"ARM64 mount must be an object"; return nil; }
    BOOL automatic = ORTBoolean(value, @"automatic", NO, error);
    if (*error) return nil;
    // Snapshots can be restored verbatim: rebuild current sandbox paths rather
    // than trying to reopen a previous installation's container UUID.
    if (automatic) continue;
    ORTArm64Binding *binding = ORTBinding(value, error);
    if (!binding) return nil;
    for (ORTArm64Binding *existing in mounts) if ([existing.mountPoint isEqualToString:binding.mountPoint]) {
      *error = [NSString stringWithFormat:@"ARM64 mount point is configured more than once: %@", binding.mountPoint];
      return nil;
    }
    [mounts addObject:binding];
  }
  if (configuration.bindAppSandbox) {
    if (!external) {
      NSFileManager *files = NSFileManager.defaultManager;
      NSURL *support = [files URLsForDirectory:NSApplicationSupportDirectory inDomains:NSUserDomainMask].firstObject;
      NSURL *documents = [files URLsForDirectory:NSDocumentDirectory inDomains:NSUserDomainMask].firstObject;
      if (!support || !documents) { *error = @"iOS sandbox storage directories are unavailable"; return nil; }
      for (NSURL *URL in @[documents, [support URLByAppendingPathComponent:@"Operit2"]]) {
        NSError *failure = nil;
        if (![files createDirectoryAtURL:URL withIntermediateDirectories:YES attributes:nil error:&failure]) {
          *error = [NSString stringWithFormat:@"Create ARM64 sandbox directory (%@): %@ (%@ %ld)",
                    URL.path, failure.localizedDescription, failure.domain, (long)failure.code];
          return nil;
        }
      }
    }
    NSString *container = NSHomeDirectory().stringByStandardizingPath;
    ORTArm64DirectoryAccess *directory = [ORTArm64DirectoryAccess accessURL:[NSURL fileURLWithPath:container isDirectory:YES]
                                                            bookmarkData:nil error:error];
    if (!directory) return nil;
    for (NSString *path in [NSOrderedSet orderedSetWithArray:@[container, directory.path]]) {
      if ([path isEqualToString:@"/"] || ![path hasPrefix:@"/"]) { *error = @"iOS app container path is invalid"; return nil; }
      BOOL supplied = NO;
      for (ORTArm64Binding *binding in mounts) if ([binding.mountPoint isEqualToString:path]) supplied = YES;
      if (supplied) continue;
      ORTArm64Binding *binding = [ORTArm64Binding new];
      binding.directory = directory;
      binding.mountPoint = path;
      binding.automatic = YES;
      [mounts addObject:binding];
    }
  }
  configuration.mounts = ORTSortedBindings(mounts);
  configuration.identifier = NSUUID.UUID.UUIDString;
  return configuration;
}
// Registry queue only. Validation finishes before any old session is closed.
static BOOL ORTInstallConfiguration(ORTArm64RootConfiguration *configuration, BOOL closeExisting,
                                     NSString **error) {
  BOOL same = ORTSameConfiguration(ORTConfiguration, configuration);
  NSMutableArray<ORTArm64TerminalSession *> *active = [NSMutableArray array];
  for (ORTArm64TerminalSession *session in ORTSessions.allValues) {
    __block BOOL running = NO;
    dispatch_sync(session.queue, ^{ running = !session.closed; });
    if (running) [active addObject:session];
  }
  if (!same && active.count && !closeExisting) {
    NSMutableArray *identifiers = [NSMutableArray array];
    for (ORTArm64TerminalSession *session in active) [identifiers addObject:session.sessionId];
    *error = [NSString stringWithFormat:@"ARM64 root/mount configuration is in use by sessions %@; close them or set closeExistingSessions=true",
              [identifiers componentsJoinedByString:@", "]];
    return NO;
  }
  if (same) {
    if (!ORTCheckDirectories(configuration, error)) return NO;
    configuration.identifier = ORTConfiguration.identifier;
  } else {
    if (!ORTValidateConfiguration(configuration, error)) return NO;
    NSMutableArray *failures = [NSMutableArray array];
    for (ORTArm64TerminalSession *session in active) dispatch_sync(session.queue, ^{
      NSString *failure = [session close];
      session.failure = failure ?: @"ARM64 runtime configuration changed; terminal session was closed";
      if (failure) [failures addObject:[NSString stringWithFormat:@"%@: %@", session.sessionId, failure]];
    });
    if (failures.count) {
      *error = [NSString stringWithFormat:@"ARM64 configuration was not switched; closing old sessions failed: %@",
                [failures componentsJoinedByString:@"; "]];
      return NO;
    }
    [ORTSessions removeAllObjects];
    [ORTSessionNames removeAllObjects];
  }
  ORTConfiguration = configuration;
  return YES;
}
NSDictionary *ORTArm64ConfigureRuntime(NSDictionary *request, NSString **errorOut) {
  ORTInitialize();
  __block NSString *error = nil;
  __block NSDictionary *result = nil;
  if (![request isKindOfClass:NSDictionary.class]) error = @"ARM64 runtime configuration must be an object";
  else dispatch_sync(ORTRegistryQueue, ^{
    BOOL closeExisting = ORTBoolean(request, @"closeExistingSessions", NO, &error);
    if (error) return;
    NSArray<NSString *> *previousSessions = ORTSessions.allKeys;
    ORTArm64RootConfiguration *configuration = ORTBuildConfiguration(request, &error);
    if (configuration && ORTInstallConfiguration(configuration, closeExisting, &error)) {
      ORTInitialConfigurationError = nil;
      NSMutableDictionary *snapshot = [ORTConfigurationSnapshot(ORTConfiguration) mutableCopy];
      NSMutableArray<NSString *> *closedSessions = [NSMutableArray array];
      for (NSString *identifier in previousSessions)
        if (!ORTSessions[identifier]) [closedSessions addObject:identifier];
      snapshot[@"closedSessionIds"] = closedSessions;
      result = snapshot;
    } else if (!ORTConfiguration) {
      ORTInitialConfigurationError = error ?: @"ARM64 runtime configuration failed";
    }
  });
  if (errorOut) *errorOut = error;
  return result;
}
NSDictionary *ORTArm64RuntimeConfiguration(void) {
  ORTInitialize();
  __block NSDictionary *result = nil;
  dispatch_sync(ORTRegistryQueue, ^{ result = ORTConfigurationSnapshot(ORTConfiguration); });
  return result;
}
static BOOL ORTPrepareRoot(NSString **error) {
  if (ORTConfiguration) return YES;
  if (ORTInitialConfigurationError) { *error = ORTInitialConfigurationError; return NO; }
#if TARGET_OS_OSX
  // Workspace bindings will be supplied by the unified workspace mount layer.
  NSDictionary *defaults = @{ @"bindAppSandbox": @NO };
#else
  NSDictionary *defaults = @{};
#endif
  ORTArm64RootConfiguration *configuration = ORTBuildConfiguration(defaults, error);
  return configuration && ORTInstallConfiguration(configuration, NO, error);
}
static BOOL ORTMount(NSDictionary *request, NSString **error) {
  BOOL closeExisting = ORTBoolean(request, @"closeExistingSessions", NO, error);
  if (*error) return NO;
  ORTArm64Binding *binding = ORTBinding(request, error);
  if (!binding || !ORTPrepareRoot(error)) return NO;
  NSString *relative = nil;
  BOOL readOnly = NO;
  ORTArm64DirectoryAccess *backing = ORTBackingDirectory(ORTConfiguration, ORTConfiguration.mounts,
                                                        binding.mountPoint, &relative, &readOnly);
  NSString *existingPath = relative.length ? [backing.path stringByAppendingPathComponent:relative] : backing.path;
  if (readOnly == binding.readOnly && [existingPath isEqualToString:binding.directory.path]) {
    if (!ORTCheckDirectories(ORTConfiguration, error)) return NO;
    // An identical effective binding is already visible to all existing VMs.
    return YES;
  }
  for (ORTArm64Binding *existing in ORTConfiguration.mounts)
    if ([existing.mountPoint isEqualToString:binding.mountPoint]) {
      *error = @"ARM64 mount point already has a different binding; replace it with arm64RuntimeConfigure";
      return NO;
    }
  if (ORTConfiguration.mounts.count >= 130) { *error = @"ARM64 directory binding limit exceeded"; return NO; }
  ORTArm64RootConfiguration *configuration = [ORTArm64RootConfiguration new];
  configuration.root = ORTConfiguration.root;
  configuration.source = ORTConfiguration.source;
  configuration.bindAppSandbox = ORTConfiguration.bindAppSandbox;
  configuration.identifier = NSUUID.UUID.UUIDString;
  configuration.mounts = ORTSortedBindings([ORTConfiguration.mounts arrayByAddingObject:binding]);
  return ORTInstallConfiguration(configuration, closeExisting, error);
}
// Workspace mounts belong to future sessions. Existing VMs retain their own
// immutable configuration and sandbox grants until they close.
static BOOL ORTPrepareWorkspaceMount(NSDictionary *request, NSString **error) {
  ORTArm64Binding *binding = ORTBinding(request, error);
  if (!binding || !ORTPrepareRoot(error)) return NO;
  binding.workspace = YES;
  binding.automatic = YES;
  NSMutableArray<ORTArm64Binding *> *mounts = [NSMutableArray array];
  for (ORTArm64Binding *existing in ORTConfiguration.mounts)
    if (!existing.workspace) [mounts addObject:existing];
  NSString *relative = nil;
  BOOL readOnly = NO;
  ORTArm64DirectoryAccess *backing = ORTBackingDirectory(ORTConfiguration, mounts,
                                                        binding.mountPoint, &relative, &readOnly);
  NSString *effective = relative.length ? [backing.path stringByAppendingPathComponent:relative] : backing.path;
  if (readOnly || ![effective isEqualToString:binding.directory.path]) {
    for (ORTArm64Binding *existing in mounts)
      if ([existing.mountPoint isEqualToString:binding.mountPoint]) {
        *error = @"ARM64 workspace conflicts with an existing directory binding";
        return NO;
      }
    [mounts addObject:binding];
  }
  ORTArm64RootConfiguration *configuration = [ORTArm64RootConfiguration new];
  configuration.root = ORTConfiguration.root;
  configuration.source = ORTConfiguration.source;
  configuration.bindAppSandbox = ORTConfiguration.bindAppSandbox;
  configuration.identifier = ORTConfiguration.identifier;
  configuration.mounts = ORTSortedBindings(mounts);
  if (ORTSameConfiguration(configuration, ORTConfiguration)) return ORTCheckDirectories(configuration, error);
  if (!ORTValidateConfiguration(configuration, error)) return NO;
  ORTConfiguration = configuration;
  return YES;
}
static ORTArm64TerminalSession *ORTStart(NSString *name, NSString *cwd, uint32_t rows, uint32_t cols, NSString **error) {
  if (![cwd hasPrefix:@"/"]) { *error = @"ARM64 working directory must be absolute"; return nil; }
  if (!ORTPrepareRoot(error)) return nil;
  ORTArm64TerminalSession *session = [ORTArm64TerminalSession new];
  session.sessionName = name;
  session.workingDir = cwd;
  session.rows = rows;
  session.cols = cols;
  session.configuration = ORTConfiguration;
  session.configurationId = ORTConfiguration.identifier;
  session.rootPath = ORTConfiguration.root.path;
  dispatch_sync(session.queue, ^{
    NSString *failure = nil;
    session.vm = ORTCreateVM(session.configuration, &failure);
    if (!session.vm) { [session fail:failure]; return; }
    NSString *directory = [@"OPERIT_WORKING_DIR=" stringByAppendingString:cwd];
    // The private record tracks cwd; the visible prompt expands it again after
    // each command so manual terminals show directory changes immediately.
    NSString *prompt = [NSString stringWithFormat:@"PS1=\036%@:prompt:${PWD}\037${PWD} # ", session.protocolToken];
    const char *argv[] = {"/bin/sh", "-c", "cd -- \"$OPERIT_WORKING_DIR\" || exit; exec /bin/sh -i", nullptr};
    const char *envp[] = {"TERM=xterm-256color", "HOME=/root", "USER=root", "LOGNAME=root",
      "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin", "LANG=C.UTF-8", "NODE_OPTIONS=--jitless",
      directory.UTF8String, prompt.UTF8String, "PS2=> ", nullptr};
    if (ort_arm64_load(session.vm, "/bin/sh", argv, envp) != 0) {
      [session fail:ORTRuntimeError(session.vm, @"shell load failed")];
      return;
    }
    if (ort_arm64_terminal_open(session.vm, cols, rows) != 0) {
      [session fail:ORTRuntimeError(session.vm, @"PTY open failed")];
      return;
    }
    session.terminalOpen = YES;
    [session schedulePump:0];
  });
  if (dispatch_semaphore_wait(session.startup, dispatch_time(DISPATCH_TIME_NOW, 30LL * NSEC_PER_SEC)) != 0)
    dispatch_sync(session.queue, ^{ [session fail:@"ARM64 shell did not reach its first prompt within 30 seconds"]; });
  __block NSString *failure = nil;
  dispatch_sync(session.queue, ^{ failure = session.failure; if (!session.ready && !failure) failure = @"ARM64 shell startup failed"; });
  if (failure) { *error = failure; return nil; }
  ORTSessions[session.sessionId] = session;
  ORTSessionNames[name] = session.sessionId;
  return session;
}
static ORTArm64TerminalSession *ORTSession(NSDictionary *request, NSString **error) {
  NSString *identifier = ORTString(request, @"sessionId", error);
  if (!identifier) return nil;
  __block ORTArm64TerminalSession *session = nil;
  dispatch_sync(ORTRegistryQueue, ^{ session = ORTSessions[identifier]; });
  if (!session) *error = [NSString stringWithFormat:@"ARM64 terminal session does not exist: %@", identifier];
  return session;
}
static NSDictionary *ORTEntry(ORTArm64TerminalSession *session) {
  return @{ @"sessionId": session.sessionId, @"sessionName": session.sessionName,
            @"terminalType": @"shell", @"sessionKind": @"pty", @"workingDir": session.workingDir,
            @"commandRunning": @(session.commandRunning), @"rootDirectory": session.rootPath,
            @"configurationId": session.configurationId };
}

static void ORTAbandon(ORTArm64TerminalSession *session, ORTArm64Execution *execution) {
  if (execution.done || session.execution != execution) return;
  // A partly submitted protocol line cannot safely be resumed by another
  // request. Close this VM instead of leaving shell syntax on its input line.
  if (session.queuedInput.length) {
    [session fail:@"ARM64 command canceled while its input was incomplete"];
    return;
  }
  NSString *error = [session interrupt];
  if (error) { [session fail:error]; return; }
  execution.abandoned = YES;
  execution.cancelDeadline = NSProcessInfo.processInfo.systemUptime + 2.0;
}
// Return from callback cancellation only after the foreground job has yielded
// back to the shell, so the caller can immediately reuse the same session.
static void ORTCancelExecution(ORTArm64TerminalSession *session, ORTArm64Execution *execution) {
  dispatch_sync(session.queue, ^{ ORTAbandon(session, execution); });
  const NSTimeInterval deadline = NSProcessInfo.processInfo.systemUptime + 2.5;
  for (;;) {
    __block BOOL done = NO;
    dispatch_sync(session.queue, ^{
      done = execution.done || session.closed;
      if (!done && NSProcessInfo.processInfo.systemUptime >= deadline) {
        [session fail:@"ARM64 foreground command did not stop after cancellation"];
        done = YES;
      }
      if (done && session.execution == execution) session.execution = nil;
    });
    if (done) return;
    dispatch_semaphore_wait(execution.changed, dispatch_time(DISPATCH_TIME_NOW, 20LL * NSEC_PER_MSEC));
  }
}
// Source a real guest file rather than placing an arbitrarily long command on
// a canonical PTY line. Sourcing preserves shell variables and working directory.
static NSString *ORTWriteCommand(ORTArm64TerminalSession *session, ORTArm64Execution *execution,
                                  NSString *command, NSString **error) {
  NSData *data = [command dataUsingEncoding:NSUTF8StringEncoding];
  if (!data || data.length > ORTCommandCapacity) {
    *error = @"ARM64 command input exceeded 16 MiB";
    return nil;
  }
  if (!session.configuration) { *error = @"ARM64 session has released its root directory"; return nil; }
  int directory = ORTOpenGuestDirectory(session.configuration, session.configuration.mounts, @"/tmp", NO, YES, error);
  if (directory < 0) return nil;
  execution.scriptDirectory = directory;
  execution.scriptName = [NSString stringWithFormat:@".operit-command-%@.sh", execution.identifier];
  int file = openat(directory, execution.scriptName.fileSystemRepresentation,
                    O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC | O_NOFOLLOW, 0600);
  if (file < 0) {
    *error = [NSString stringWithFormat:@"ARM64 command file creation failed: %s", strerror(errno)];
    // A collision must not remove an existing file owned by somebody else.
    execution.scriptName = nil;
    [execution removeScript];
    return nil;
  }
  const uint8_t *bytes = static_cast<const uint8_t *>(data.bytes);
  NSUInteger offset = 0;
  while (offset < data.length) {
    ssize_t written = write(file, bytes + offset, data.length - offset);
    if (written < 0 && errno == EINTR) continue;
    if (written <= 0) {
      *error = [NSString stringWithFormat:@"ARM64 command file write failed: %s", written < 0 ? strerror(errno) : "zero-byte write"];
      close(file);
      [execution removeScript];
      return nil;
    }
    offset += (NSUInteger)written;
  }
  if (close(file) < 0) {
    *error = [NSString stringWithFormat:@"ARM64 command file close failed: %s", strerror(errno)];
    [execution removeScript];
    return nil;
  }
  return [@"/tmp/" stringByAppendingString:execution.scriptName];
}
static NSDictionary *ORTExecute(ORTArm64TerminalSession *session, NSString *command, uint64_t timeout,
                                void *context, operit_ios_arm64_terminal_output_callback callback) {
  ORTArm64Execution *execution = [ORTArm64Execution new];
  __block NSString *error = nil;
  dispatch_sync(session.queue, ^{
    if (session.failure || session.closed) { error = session.failure ?: @"ARM64 terminal session is not running"; return; }
    if (session.execution || session.commandRunning) { error = @"ARM64 terminal already has a command in progress"; return; }
    NSString *script = ORTWriteCommand(session, execution, command, &error);
    if (!script) return;
    // Discard pending line-editor input, including stale terminal replies.
    NSString *line = [NSString stringWithFormat:
      @"\025command printf '\\036%@:begin:%@\\037'; . %@; command printf '\\036%@:done:%@:%%s:%%s\\037' \"$?\" \"$PWD\"\n",
      session.protocolToken, execution.identifier, ORTQuote(script), session.protocolToken, execution.identifier];
    NSData *input = [line dataUsingEncoding:NSUTF8StringEncoding];
    if (input.length > ORTCommandCapacity) { error = @"ARM64 command input exceeded 16 MiB"; return; }
    // Echo the submitted command to the UI without adding it to tool output.
    NSString *display = [[command stringByReplacingOccurrencesOfString:@"\r\n" withString:@"\n"]
                         stringByReplacingOccurrencesOfString:@"\n" withString:@"\r\n"];
    [session publish:[[display stringByAppendingString:@"\r\n"] dataUsingEncoding:NSUTF8StringEncoding]];
    session.execution = execution;
    session.commandRunning = YES;
    [session.queuedInput appendData:input];
    [session schedulePump:0];
  });
  if (error) return ORTError(error);
  NSMutableData *output = [NSMutableData data];
  NSTimeInterval deadline = NSProcessInfo.processInfo.systemUptime + (NSTimeInterval)timeout / 1000.0;
  for (;;) {
    __block NSData *frame = nil;
    __block BOOL done = NO;
    __block BOOL started = NO;
    __block NSInteger exitCode = 0;
    dispatch_sync(session.queue, ^{
      frame = [execution.frames copy];
      [execution.frames setLength:0];
      error = execution.error;
      done = execution.done;
      started = execution.started;
      exitCode = execution.exitCode;
    });
    [output appendData:frame];
    // Empty frames poll cancellation while a running command is silent.
    if (callback && (frame.length || (started && !done)) &&
        !callback(context, frame.length ? static_cast<const uint8_t *>(frame.bytes) : nullptr, frame.length)) {
      ORTCancelExecution(session, execution);
      return ORTError(@"ARM64 terminal execution canceled by its output callback");
    }
    if (done) {
      dispatch_sync(session.queue, ^{ if (session.execution == execution) session.execution = nil; });
      if (error) return ORTError(error);
      return ORTResult(@{ @"sessionId": session.sessionId, @"terminalType": @"shell", @"outputBytes": ORTBytes(output),
                         @"exitCode": @(exitCode), @"timedOut": @NO });
    }
    if (NSProcessInfo.processInfo.systemUptime >= deadline) {
      __block NSData *lastFrame = nil;
      dispatch_sync(session.queue, ^{
        // Recheck on the owning queue: completion can race the caller deadline.
        if (!execution.done) ORTAbandon(session, execution);
        else done = YES;
        error = execution.error;
        exitCode = execution.exitCode;
        lastFrame = [execution.frames copy];
        [execution.frames setLength:0];
        if (execution.done && session.execution == execution) session.execution = nil;
      });
      [output appendData:lastFrame];
      if (callback && lastFrame.length && !callback(context, static_cast<const uint8_t *>(lastFrame.bytes), lastFrame.length))
        return ORTError(@"ARM64 terminal execution canceled by its output callback");
      if (error) return ORTError(error);
      return ORTResult(@{ @"sessionId": session.sessionId, @"terminalType": @"shell", @"outputBytes": ORTBytes(output),
                         @"exitCode": done ? @(exitCode) : @124, @"timedOut": @(!done) });
    }
    dispatch_semaphore_wait(execution.changed, dispatch_time(DISPATCH_TIME_NOW, 20LL * NSEC_PER_MSEC));
  }
}

static NSDictionary *ORTHandle(NSString *command, NSDictionary *request, void *context,
                               operit_ios_arm64_terminal_output_callback callback) {
  ORTInitialize();
  __block NSString *error = nil;
  if ([command isEqualToString:@"arm64RuntimeConfigure"]) {
    NSDictionary *configuration = ORTArm64ConfigureRuntime(request, &error);
    if (configuration) return ORTResult(configuration);
    // A shutdown failure can close some VMs while keeping the old root. Tell
    // callers which sessions actually retired, without hiding the failure.
    NSMutableArray<NSString *> *closedSessions = [NSMutableArray array];
    dispatch_sync(ORTRegistryQueue, ^{
      for (ORTArm64TerminalSession *session in ORTSessions.allValues)
        dispatch_sync(session.queue, ^{ if (session.closed) [closedSessions addObject:session.sessionId]; });
    });
    NSMutableDictionary *failure = [ORTError(error) mutableCopy];
    failure[@"closedSessionIds"] = closedSessions;
    return failure;
  }
  if ([command isEqualToString:@"arm64RuntimeConfiguration"])
    return ORTResult(ORTArm64RuntimeConfiguration());
  if ([command isEqualToString:@"arm64RuntimeState"]) {
    // This lightweight query is also used by Rust's PTY polling path. Avoid
    // creating bookmarks or touching the directory provider on every poll.
    __block NSDictionary *state = nil;
    dispatch_sync(ORTRegistryQueue, ^{
      if (ORTConfiguration) state = @{ @"configured": @YES, @"configurationId": ORTConfiguration.identifier };
      else state = ORTInitialConfigurationError ?
          @{ @"configured": @NO, @"configurationError": ORTInitialConfigurationError } : @{ @"configured": @NO };
    });
    return ORTResult(state);
  }
  if ([command isEqualToString:@"terminalWorkspaceMount"]) {
    dispatch_sync(ORTRegistryQueue, ^{ ORTPrepareWorkspaceMount(request, &error); });
    return error ? ORTError(error) : ORTResult(@{});
  }
  if ([command isEqualToString:@"managedRuntimeMount"]) {
    dispatch_sync(ORTRegistryQueue, ^{ ORTMount(request, &error); });
    return error ? ORTError(error) : ORTResult(@{});
  }
  if ([command isEqualToString:@"terminalStart"] || [command isEqualToString:@"terminalCreateOrGet"]) {
    NSString *name = ORTString(request, @"sessionName", &error);
    NSString *type = ORTString(request, @"terminalType", &error);
    BOOL reuse = [command isEqualToString:@"terminalCreateOrGet"];
    NSString *cwd = reuse ? @"/root" : ORTString(request, @"workingDir", &error);
    uint32_t rows = reuse ? 24 : ORTDimension(request, @"rows", &error);
    uint32_t cols = reuse ? 80 : ORTDimension(request, @"cols", &error);
    if (error) return ORTError(error);
    if (![type isEqualToString:@"shell"]) return ORTError(@"Unsupported ARM64 terminal type");
    __block ORTArm64TerminalSession *session = nil;
    __block BOOL isNew = YES;
    dispatch_sync(ORTRegistryQueue, ^{
      NSString *identifier = ORTSessionNames[name];
      if (reuse && identifier) {
        ORTArm64TerminalSession *existing = ORTSessions[identifier];
        __block BOOL active = NO;
        if (existing) dispatch_sync(existing.queue, ^{
          active = !existing.closed && [existing.configurationId isEqualToString:ORTConfiguration.identifier]
              && ORTSameConfiguration(existing.configuration, ORTConfiguration);
          if (active && !ORTCheckDirectories(existing.configuration, &error)) active = NO;
        });
        if (error) return;
        if (active) { session = existing; isNew = NO; }
      }
      if (!session) session = ORTStart(name, cwd, rows, cols, &error);
    });
    return session ? ORTResult(@{ @"sessionId": session.sessionId, @"sessionName": name,
                                 @"terminalType": @"shell", @"isNewSession": @(isNew),
                                 @"rootDirectory": session.rootPath, @"configurationId": session.configurationId }) : ORTError(error);
  }
  if ([command isEqualToString:@"terminalList"]) {
    __block NSArray *sessions = nil;
    dispatch_sync(ORTRegistryQueue, ^{ sessions = ORTSessions.allValues; });
    NSMutableArray *entries = [NSMutableArray array];
    for (ORTArm64TerminalSession *session in sessions)
      dispatch_sync(session.queue, ^{ if (!session.closed) [entries addObject:ORTEntry(session)]; });
    [entries sortUsingComparator:^NSComparisonResult(NSDictionary *a, NSDictionary *b) { return [a[@"sessionId"] compare:b[@"sessionId"]]; }];
    return ORTResult(@{ @"sessions": entries });
  }
  ORTArm64TerminalSession *session = ORTSession(request, &error);
  if (!session) return ORTError(error);
  if ([command isEqualToString:@"terminalExecute"]) {
    NSString *shellCommand = ORTString(request, @"command", &error);
    id timeout = request[@"timeoutMs"];
    if (error) return ORTError(error);
    if (![timeout isKindOfClass:NSNumber.class] || CFGetTypeID((__bridge CFTypeRef)timeout) == CFBooleanGetTypeID() ||
        [timeout doubleValue] < 0 || [timeout doubleValue] != [timeout unsignedLongLongValue])
      return ORTError(@"ARM64 terminal request has invalid timeoutMs");
    return ORTExecute(session, shellCommand, [timeout unsignedLongLongValue], context, callback);
  }
  __block NSDictionary *result = nil;
  dispatch_sync(session.queue, ^{
    if ([command isEqualToString:@"terminalRead"]) {
      NSData *data = [session.pendingOutput copy];
      [session.pendingOutput setLength:0];
      [session schedulePump:0];
      // Deliver all buffered real output before reporting a VM failure.
      result = data.length || !session.failure ? ORTResult(@{ @"bytes": ORTBytes(data) }) : ORTError(session.failure);
    } else if ([command isEqualToString:@"terminalScreen"]) {
      result = ORTResult(@{ @"sessionId": session.sessionId, @"terminalType": @"shell", @"rows": @(session.rows),
                           @"cols": @(session.cols), @"contentBytes": ORTBytes(session.screenOutput),
                           @"commandRunning": @(session.commandRunning) });
    } else if ([command isEqualToString:@"terminalPoll"]) {
      result = session.failure ? ORTError(session.failure) : ORTResult(@{ @"exitCode": session.hasExitCode ? @(session.exitCode) : NSNull.null });
    } else if ([command isEqualToString:@"terminalClose"]) {
      error = [session close];
      result = error ? ORTError(error) : ORTResult(@{});
    } else if (session.closed || !session.vm) {
      result = ORTError(session.failure ?: @"ARM64 terminal session is not running");
    } else if ([command isEqualToString:@"terminalWrite"]) {
      NSData *input = ORTInput(request, &error);
      if (!input) { result = ORTError(error); return; }
      if (session.queuedInput.length) { result = ORTError(@"ARM64 command input is still being submitted"); return; }
      int64_t accepted = input.length ? ort_arm64_terminal_write(session.vm, input.bytes, input.length) : 0;
      if (accepted == ORTLinuxAgain) accepted = 0;
      if (accepted < 0 || (uint64_t)accepted > input.length) {
        result = ORTError([NSString stringWithFormat:@"ARM64 terminal write failed (Linux errno %lld)", (long long)-accepted]);
        return;
      }
      if (accepted && memchr(input.bytes, '\n', (size_t)accepted)) session.commandRunning = YES;
      [session schedulePump:0];
      result = ORTResult(@{ @"acceptedBytes": @(accepted) });
    } else if ([command isEqualToString:@"terminalResize"]) {
      uint32_t rows = ORTDimension(request, @"rows", &error);
      uint32_t cols = ORTDimension(request, @"cols", &error);
      if (error) { result = ORTError(error); return; }
      if (ort_arm64_terminal_resize(session.vm, cols, rows) != 0) {
        result = ORTError(ORTRuntimeError(session.vm, @"PTY resize failed"));
        return;
      }
      session.rows = rows;
      session.cols = cols;
      [session schedulePump:0];
      result = ORTResult(@{});
    } else if ([command isEqualToString:@"terminalSignal"]) {
      id signal = request[@"signal"];
      if (![signal isKindOfClass:NSNumber.class] || [signal intValue] != ORTLinuxInterrupt) {
        result = ORTError(@"ARM64 terminal signal must be Linux SIGINT (2)");
        return;
      }
      error = [session interrupt];
      result = error ? ORTError(error) : ORTResult(@{});
    } else {
      result = ORTError([NSString stringWithFormat:@"Unsupported ARM64 terminal command: %@", command]);
    }
  });
  if ([command isEqualToString:@"terminalClose"] && !error)
    dispatch_sync(ORTRegistryQueue, ^{
      [ORTSessions removeObjectForKey:session.sessionId];
      if ([ORTSessionNames[session.sessionName] isEqualToString:session.sessionId]) [ORTSessionNames removeObjectForKey:session.sessionName];
    });
  return result;
}
static char *ORTCall(const char *command, const char *requestJSON, void *context,
                      operit_ios_arm64_terminal_output_callback callback) {
  @autoreleasepool {
    if (!command || !requestJSON) return ORTEncode(ORTError(@"ARM64 terminal bridge request is missing"));
    NSString *name = [NSString stringWithUTF8String:command];
    NSData *data = [NSData dataWithBytes:requestJSON length:strlen(requestJSON)];
    NSError *error = nil;
    id request = [NSJSONSerialization JSONObjectWithData:data options:0 error:&error];
    if (!name || ![request isKindOfClass:NSDictionary.class])
      return ORTEncode(ORTError(error.localizedDescription ?: @"ARM64 terminal request must be a JSON object"));
    return ORTEncode(ORTHandle(name, request, context, callback));
  }
}
extern "C" char *operit_ios_arm64_terminal_call(const char *command, const char *requestJSON) {
  return ORTCall(command, requestJSON, nullptr, nullptr);
}
extern "C" char *operit_ios_arm64_terminal_execute_streaming(const char *command, const char *requestJSON,
    void *context, operit_ios_arm64_terminal_output_callback callback) {
  return ORTCall(command, requestJSON, context, callback);
}
extern "C" void operit_ios_arm64_terminal_free(char *value) { free(value); }
