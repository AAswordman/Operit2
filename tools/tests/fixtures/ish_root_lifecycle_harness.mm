#import <Foundation/Foundation.h>
#import "IshRootFilesystem.h"
#include <stdarg.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>

extern "C" {
#include "tools/fakefs.h"
#include "fs/fake-db.h"
_Noreturn void die(const char *format, ...) {
  va_list args;
  va_start(args, format);
  vfprintf(stderr, format, args);
  va_end(args);
  exit(90); // A recoverable filesystem failure must never take this path.
}
}

static void cancelImport(void *cookie, double progress, const char *message, bool *cancel) {
  (void)progress;
  (void)message;
  *cancel = ++*(int *)cookie == 3;
}

int main(int argc, const char **argv) {
  @autoreleasepool {
    if (argc != 4) return 2;
    NSString *mode = @(argv[1]);
    if ([mode isEqualToString:@"import"] || [mode isEqualToString:@"cancel"]) {
      fakefsify_error error = {};
      int count = 0;
      struct progress progress = {&count, [mode isEqualToString:@"cancel"] ? cancelImport : NULL};
      bool ok = fakefs_import(argv[3], argv[2], &error, progress);
      printf("%s\n", error.message ?: "");
      free(error.message);
      return ok ? 0 : 4;
    }
    if ([mode isEqualToString:@"mount"]) {
      struct fakefs_db fs = {};
      NSString *root = @(argv[2]);
      int result = fake_db_init(&fs, [root stringByAppendingPathComponent:@"meta.db"].fileSystemRepresentation);
      if (result == 0) fake_db_deinit(&fs);
      return result == 0 ? 0 : 5;
    }
    NSString *error = nil;
    NSString *root = ORTIshPrepareRootAtURL([NSURL fileURLWithPath:@(argv[2])],
                                           [NSURL fileURLWithPath:@(argv[3])], &error);
    printf("%s\n", (root ?: error).UTF8String);
    return root != nil ? 0 : 6;
  }
}
