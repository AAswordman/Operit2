#import "IshRootFilesystem.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

extern "C" {
#include "tools/fakefs.h"
}

static NSString *ORTIshRootError(struct fakefsify_error *error) {
  NSString *message = error->message == NULL ? @"iSH root preparation failed"
      : [NSString stringWithUTF8String:error->message];
  free(error->message);
  return message;
}

NSString *ORTIshPrepareRootAtURL(NSURL *runtimeDirectory, NSURL *archive, NSString **errorOut) {
  NSFileManager *files = NSFileManager.defaultManager;
  NSError *error = nil;
  if (![files createDirectoryAtURL:runtimeDirectory withIntermediateDirectories:YES
                       attributes:nil error:&error]) {
    *errorOut = error.localizedDescription;
    return nil;
  }
  NSURL *root = [runtimeDirectory URLByAppendingPathComponent:@"rootfs"];
  NSURL *staging = [runtimeDirectory URLByAppendingPathComponent:@"rootfs.importing"];
  if ([files fileExistsAtPath:root.path]) {
    struct fakefsify_error preparation = {};
    if (fakefs_prepare(root.fileSystemRepresentation, &preparation)) return root.path;
    BOOL incomplete = preparation.type == fakefsify_error::ERR_INCOMPLETE;
    NSString *message = ORTIshRootError(&preparation);
    if (!incomplete) {
      *errorOut = message;
      return nil;
    }
    // Preserve all old files. Only a valid, empty version-0 database proves
    // that the legacy importer never committed its metadata transaction.
    NSURL *retained = [runtimeDirectory URLByAppendingPathComponent:
        [@"rootfs.incomplete-" stringByAppendingString:NSUUID.UUID.UUIDString]];
    if (![files moveItemAtURL:root toURL:retained error:&error]) {
      *errorOut = error.localizedDescription;
      return nil;
    }
    NSLog(@"iSH retained incomplete root at %@", retained.lastPathComponent);
  }
  if (archive == nil) {
    *errorOut = @"Bundled iSH Alpine root archive is missing";
    return nil;
  }
  // This name is exclusively owned by imports; final roots are never deleted.
  if ([files fileExistsAtPath:staging.path] && ![files removeItemAtURL:staging error:&error]) {
    *errorOut = error.localizedDescription;
    return nil;
  }
  struct fakefsify_error preparation = {};
  BOOL ready = fakefs_import(archive.fileSystemRepresentation, staging.fileSystemRepresentation,
                            &preparation, (struct progress){})
      && fakefs_prepare(staging.fileSystemRepresentation, &preparation);
  if (!ready) {
    *errorOut = ORTIshRootError(&preparation);
    [files removeItemAtURL:staging error:NULL];
    return nil;
  }
  // Both names are siblings. Publish the fully closed database and data tree
  // together with one atomic rename; a killed importer cannot publish a root.
  if (rename(staging.fileSystemRepresentation, root.fileSystemRepresentation) != 0) {
    *errorOut = [NSString stringWithFormat:@"iSH root commit failed: %s", strerror(errno)];
    return nil;
  }
  return root.path;
}
