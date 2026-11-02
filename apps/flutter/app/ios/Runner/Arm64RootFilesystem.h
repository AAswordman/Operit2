#import <Foundation/Foundation.h>

NS_ASSUME_NONNULL_BEGIN

// Only the managed, bundled installation uses Alpine installation metadata.
FOUNDATION_EXPORT NSString * _Nullable ORTArm64PrepareRootAtURL(
    NSURL *runtimeDirectory, NSURL * _Nullable archive, NSString * _Nullable * _Nullable errorOut);

// Retains the original security-scoped URL (or a resolved iOS bookmark) and an
// open directory fd. Callers must retain this object until all VM/file I/O ends.
// A plain path is also accepted when the app already has filesystem access.
@interface ORTArm64DirectoryAccess : NSObject
@property(nonatomic, readonly) NSURL *URL;
@property(nonatomic, readonly) NSString *path;
@property(nonatomic, readonly) NSString *identity;
@property(nonatomic, readonly) int descriptor; // Borrowed; never close this fd.
+ (nullable instancetype)accessURL:(nullable NSURL *)URL
                     bookmarkData:(nullable NSData *)bookmarkData
                            error:(NSString * _Nullable * _Nullable)errorOut;
- (BOOL)checkAvailability:(NSString * _Nullable * _Nullable)errorOut;
// Returns the scope's bookmark; pair it with path when a child directory is used.
- (nullable NSData *)bookmarkWithError:(NSString * _Nullable * _Nullable)errorOut;
- (instancetype)init NS_UNAVAILABLE;
+ (instancetype)new NS_UNAVAILABLE;
@end

// Native entry point, also available as arm64RuntimeConfigure through the JSON
// terminal bridge. rootDirectory is an absolute path/file URL (or native NSURL),
// rootBookmark is base64 (or native NSData). Omit both to use the bundled root.
// rootSource may explicitly select external/bundled; bundled ignores root fields.
// mounts: [{hostDirectory, bookmark?, mountPoint, readOnly?: false}].
// bindAppSandbox defaults to YES for bundled roots, NO for external roots.
// closeExistingSessions defaults to NO; changes otherwise fail while VMs live.
// Returned snapshots can be restored verbatim; automatic sandbox binds are rebuilt.
// Configuration is process-local. Persist returned bookmarks and restore before
// starting terminals. This call may block; invoke it on a worker queue.
FOUNDATION_EXPORT NSDictionary * _Nullable ORTArm64ConfigureRuntime(
    NSDictionary *configuration, NSString * _Nullable * _Nullable errorOut);
// Does not initialize a bundled root when no configuration has been selected.
FOUNDATION_EXPORT NSDictionary *ORTArm64RuntimeConfiguration(void);

NS_ASSUME_NONNULL_END
