#import "IshTerminalBridge.mm"

int main(int argc, const char **argv) {
  @autoreleasepool {
    if (argc != 4) return 2;
    ORTIshInitializeState();
    NSString *error = nil;
    NSString *prepared = ORTIshPrepareRootAtURL([NSURL fileURLWithPath:@(argv[2])],
                                               [NSURL fileURLWithPath:@(argv[3])], &error);
    if (prepared == nil) { fprintf(stderr, "PREPARATION_FAILED: %s\n", error.UTF8String); return 3; }
    ORTIshRootPath = [@(argv[1]) isEqualToString:@"fail"] ? [prepared stringByAppendingPathComponent:@"missing"] : prepared;
    int result = actuate_kernel("earlyprintk");
    ORTIshKernelResult = @(result);
    fprintf(stderr, "BOOT_RESULT=%d\n", result);
    if ([@(argv[1]) isEqualToString:@"fail"]) {
      if (result >= 0) return 4;
      if (ORTIshEnsureKernel(&error)) return 5;
      fprintf(stderr, "FAILURE_RETURNED_WITHOUT_PANIC: %s\n", error.UTF8String);
      return 0;
    }
    if (result != 0) return 6;
    ORTIshTerminalSession *session = ORTIshStartSession(@"smoke", @"/root", 24, 80, &error);
    if (session == nil) { fprintf(stderr, "SESSION_FAILED: %s\n", error.UTF8String); return 7; }
    NSDictionary *output = ORTIshExecute(session, @"printf '__ISH_ROOT_READY__\\n'; cat /etc/alpine-release", 10000, &error);
    fprintf(stderr, "SHELL_RESULT=%s ERROR=%s\n", output.description.UTF8String, error.UTF8String);
    return output != nil && [output.description containsString:@"__ISH_ROOT_READY__"] ? 0 : 8;
  }
}
