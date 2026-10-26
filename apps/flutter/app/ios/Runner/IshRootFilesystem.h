#import <Foundation/Foundation.h>

// Called under the runtime startup lock, before Linux can access the root.
NSString *ORTIshPrepareRootAtURL(NSURL *runtimeDirectory, NSURL *archive, NSString **errorOut);
