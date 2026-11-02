#import "Arm64RootFilesystem.h"

#include <algorithm>
#include <array>
#include <cerrno>
#include <cstring>
#include <cstdlib>
#include <dirent.h>
#include <fcntl.h>
#include <limits.h>
#include <map>
#include <set>
#include <stdexcept>
#include <stdio.h>
#include <string>
#include <sys/file.h>
#include <sys/stat.h>
#include <unistd.h>
#include <utility>
#include <vector>
#include <zlib.h>

static NSString *ORTDirectoryError(NSString *operation, NSString *path, int code) {
  return [NSString stringWithFormat:@"%@ (%@): %s (POSIX errno %d)", operation, path, strerror(code), code];
}
static NSString *ORTBookmarkError(NSString *operation, NSError *error) {
  return [NSString stringWithFormat:@"%@: %@ (%@ %ld)", operation,
          error.localizedDescription ?: @"No bookmark diagnostic", error.domain ?: @"Foundation", (long)error.code];
}

// Foundation preserves the /var spelling when a URL's final component no
// longer exists. Resolve the surviving ancestor to compare saved bookmark paths
// with /private/var aliases even after a provider moves the selected directory.
static NSString *ORTSavedDirectoryPath(NSString *path) {
  NSString *ancestor = path.stringByStandardizingPath;
  NSMutableArray<NSString *> *suffix = [NSMutableArray array];
  char resolved[PATH_MAX];
  while (ancestor.length && !realpath(ancestor.fileSystemRepresentation, resolved)) {
    if ([ancestor isEqualToString:@"/"]) return path;
    [suffix insertObject:ancestor.lastPathComponent atIndex:0];
    ancestor = ancestor.stringByDeletingLastPathComponent;
  }
  if (!ancestor.length) return path;
  NSString *result = [NSFileManager.defaultManager stringWithFileSystemRepresentation:resolved length:strlen(resolved)];
  for (NSString *component in suffix) result = [result stringByAppendingPathComponent:component];
  return result;
}

@implementation ORTArm64DirectoryAccess {
  NSURL *_URL;
  NSURL *_scopeURL;
  BOOL _accessingScope;
  int _descriptor;
  NSString *_identity;
  NSData *_bookmark;
}
- (instancetype)initWithURL:(NSURL *)URL bookmarkData:(NSData *)bookmark error:(NSString **)errorOut {
  if (!(self = [super init])) return nil;
  _descriptor = -1;
  NSError *error = nil;
  BOOL stale = NO;
  NSURL *scope = URL;
  if (bookmark) {
    // macOS restores an explicit security scope; iOS bookmarks carry the grant implicitly.
    scope = [NSURL URLByResolvingBookmarkData:bookmark
        options:NSURLBookmarkResolutionWithoutUI | NSURLBookmarkResolutionWithoutMounting
        relativeToURL:nil bookmarkDataIsStale:&stale error:&error];
    if (!scope) {
      if (errorOut) *errorOut = ORTBookmarkError(@"Resolve ARM64 directory bookmark", error);
      return nil;
    }
    _bookmark = [bookmark copy];
  }
  NSString *zero = [NSString stringWithFormat:@"%C", (unichar)0];
  if (!scope.isFileURL || !scope.path.isAbsolutePath ||
      [scope.path rangeOfString:zero].location != NSNotFound ||
      (scope.host.length && ![scope.host isEqualToString:@"localhost"]) ||
      (URL && (!URL.isFileURL || !URL.path.isAbsolutePath ||
          [URL.path rangeOfString:zero].location != NSNotFound ||
          (URL.host.length && ![URL.host isEqualToString:@"localhost"])))) {
    if (errorOut) *errorOut = @"ARM64 directory must be an absolute file URL";
    return nil;
  }
  // Start on the original URL before deriving canonical paths; rebuilding a URL
  // from its path discards the sandbox extension supplied by a document picker.
  _scopeURL = scope;
  _accessingScope = [scope startAccessingSecurityScopedResource];
  NSURL *canonicalScope = scope.URLByStandardizingPath.URLByResolvingSymlinksInPath;
  NSURL *selection = URL ?: scope;
  if (bookmark && URL) {
    // A saved path is useful for selecting a child of the granted directory,
    // but must not override a bookmark that follows a renamed/moved directory.
    id original = [NSURL resourceValuesForKeys:@[NSURLPathKey] fromBookmarkData:bookmark][NSURLPathKey];
    if ([original isKindOfClass:NSString.class] && [original isAbsolutePath]) {
      NSString *oldScope = ORTSavedDirectoryPath(original);
      NSString *oldSelection = ORTSavedDirectoryPath(URL.path);
      NSString *oldPrefix = [oldScope isEqualToString:@"/"] ? @"/" : [oldScope stringByAppendingString:@"/"];
      if ([oldSelection isEqualToString:oldScope]) selection = scope;
      else if ([oldSelection hasPrefix:oldPrefix])
        selection = [scope URLByAppendingPathComponent:[oldSelection substringFromIndex:oldPrefix.length] isDirectory:YES];
    }
  }
  _URL = selection.URLByStandardizingPath.URLByResolvingSymlinksInPath;
  NSString *scopePrefix = [canonicalScope.path isEqualToString:@"/"] ? @"/" :
      [canonicalScope.path stringByAppendingString:@"/"];
  if (bookmark && ![_URL.path isEqualToString:canonicalScope.path] && ![_URL.path hasPrefix:scopePrefix]) {
    if (errorOut) *errorOut = @"ARM64 directory is outside its bookmark's authorized directory";
    return nil;
  }
  _descriptor = open(_URL.fileSystemRepresentation, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
  if (_descriptor < 0) {
    if (errorOut) *errorOut = ORTDirectoryError(@"Open ARM64 directory", _URL.path, errno);
    return nil;
  }
  struct stat info = {};
  if (fstat(_descriptor, &info) != 0) {
    if (errorOut) *errorOut = ORTDirectoryError(@"Stat ARM64 directory", _URL.path, errno);
    return nil;
  }
  _identity = [NSString stringWithFormat:@"%llu:%llu", (unsigned long long)info.st_dev, (unsigned long long)info.st_ino];
  if (stale) {
    _bookmark = [scope bookmarkDataWithOptions:0 includingResourceValuesForKeys:@[NSURLPathKey] relativeToURL:nil error:&error];
    if (!_bookmark) {
      if (errorOut) *errorOut = ORTBookmarkError(@"Refresh stale ARM64 directory bookmark", error);
      return nil;
    }
  }
  return self;
}
+ (instancetype)accessURL:(NSURL *)URL bookmarkData:(NSData *)bookmarkData error:(NSString **)errorOut {
  if (errorOut) *errorOut = nil;
  return [[self alloc] initWithURL:URL bookmarkData:bookmarkData error:errorOut];
}
- (NSURL *)URL { return _URL; }
- (NSString *)path { return _URL.path; }
- (NSString *)identity { return _identity; }
- (int)descriptor { return _descriptor; }
- (BOOL)checkAvailability:(NSString **)errorOut {
  if (errorOut) *errorOut = nil;
  int current = open(_URL.fileSystemRepresentation, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
  if (current < 0) {
    if (errorOut) *errorOut = ORTDirectoryError(@"Reopen ARM64 directory", self.path, errno);
    return NO;
  }
  struct stat original = {}, present = {};
  int result = fstat(_descriptor, &original);
  if (result == 0) result = fstat(current, &present);
  int saved = errno;
  close(current);
  if (result != 0) {
    if (errorOut) *errorOut = ORTDirectoryError(@"Revalidate ARM64 directory", self.path, saved);
    return NO;
  }
  if (original.st_dev != present.st_dev || original.st_ino != present.st_ino) {
    if (errorOut) *errorOut = [NSString stringWithFormat:@"ARM64 directory was replaced; select it again: %@", self.path];
    return NO;
  }
  return YES;
}
- (NSData *)bookmarkWithError:(NSString **)errorOut {
  if (errorOut) *errorOut = nil;
  if (_bookmark) return _bookmark;
  NSError *error = nil;
  NSData *bookmark = [_scopeURL bookmarkDataWithOptions:0 includingResourceValuesForKeys:@[NSURLPathKey] relativeToURL:nil error:&error];
  if (!bookmark && errorOut) *errorOut = ORTBookmarkError(@"Create ARM64 directory bookmark", error);
  return bookmark;
}
- (void)dealloc {
  if (_descriptor >= 0) close(_descriptor);
  if (_accessingScope) [_scopeURL stopAccessingSecurityScopedResource];
}
@end

namespace {
constexpr const char *kRootName = "rootfs-arm64";
constexpr const char *kMarker = ".operit-arm64-root.json";
constexpr uint64_t kMaxArchive = 512ULL * 1024 * 1024;
constexpr uint64_t kMaxExpanded = 2ULL * 1024 * 1024 * 1024;
constexpr uint64_t kMaxFile = 512ULL * 1024 * 1024;
constexpr uint64_t kMaxTar = kMaxExpanded + 128ULL * 1024 * 1024;
constexpr size_t kMaxEntries = 100000;
constexpr size_t kMaxMetadata = 1024 * 1024;

class FD {
 public:
  explicit FD(int value = -1) : value_(value) {}
  ~FD() { if (value_ >= 0) close(value_); }
  FD(FD &&other) noexcept : value_(other.value_) { other.value_ = -1; }
  FD &operator=(FD &&other) noexcept {
    if (this != &other) {
      if (value_ >= 0) close(value_);
      value_ = other.value_;
      other.value_ = -1;
    }
    return *this;
  }
  FD(const FD &) = delete;
  FD &operator=(const FD &) = delete;
  int get() const { return value_; }
 private:
  int value_;
};

[[noreturn]] void Fail(const std::string &message) {
  throw std::runtime_error(message);
}

[[noreturn]] void SystemFail(const std::string &operation) {
  int saved = errno;
  Fail(operation + ": " + strerror(saved));
}

void Sync(int fd) {
  // Some Apple filesystems do not implement directory fsync.
  if (fsync(fd) != 0 && errno != EINVAL && errno != ENOTSUP) SystemFail("Sync rootfs");
}

ssize_t Read(int fd, void *data, size_t size) {
  ssize_t result;
  do { result = read(fd, data, size); } while (result < 0 && errno == EINTR);
  if (result < 0) SystemFail("Read rootfs");
  return result;
}

void Write(int fd, const void *data, size_t size) {
  const auto *bytes = static_cast<const uint8_t *>(data);
  while (size != 0) {
    ssize_t count = write(fd, bytes, size);
    if (count < 0 && errno == EINTR) continue;
    if (count < 0) SystemFail("Write rootfs");
    if (count == 0) Fail("Rootfs write made no progress");
    bytes += count;
    size -= static_cast<size_t>(count);
  }
}

std::string Field(const uint8_t *bytes, size_t size) {
  return std::string(reinterpret_cast<const char *>(bytes), strnlen(
      reinterpret_cast<const char *>(bytes), size));
}

uint64_t Octal(const uint8_t *bytes, size_t size) {
  uint64_t value = 0;
  size_t offset = 0;
  while (offset < size && bytes[offset] == ' ') ++offset;
  while (offset < size && bytes[offset] >= '0' && bytes[offset] <= '7') {
    if (value > (UINT64_MAX - 7) / 8) Fail("Tar numeric field overflow");
    value = value * 8 + bytes[offset++] - '0';
  }
  while (offset < size && (bytes[offset] == 0 || bytes[offset] == ' ')) ++offset;
  if (offset != size) Fail("Unsupported or invalid tar numeric field");
  return value;
}

uint64_t Decimal(const std::string &value) {
  if (value.empty()) Fail("Empty tar decimal field");
  uint64_t result = 0;
  for (unsigned char byte : value) {
    if (byte < '0' || byte > '9' || result > (UINT64_MAX - 9) / 10)
      Fail("Invalid tar decimal field");
    result = result * 10 + byte - '0';
  }
  return result;
}

std::vector<std::string> Components(const std::string &path) {
  std::vector<std::string> parts;
  size_t start = 0;
  while (start < path.size()) {
    size_t end = path.find('/', start);
    if (end == std::string::npos) end = path.size();
    if (end != start) parts.push_back(path.substr(start, end - start));
    start = end + 1;
  }
  return parts;
}

std::string RelativePath(const std::string &input) {
  if (input.empty() || input.front() == '/' || input.size() >= PATH_MAX ||
      input.find('\0') != std::string::npos) Fail("Invalid or absolute tar path");
  std::string result;
  for (const auto &part : Components(input)) {
    if (part == "..") Fail("Tar path contains parent traversal");
    if (part == ".") continue;
    if (part.size() > NAME_MAX) Fail("Tar path component is too long");
    if (!result.empty()) result += '/';
    result += part;
  }
  if (result == kMarker || result.compare(0, strlen(kMarker) + 1, std::string(kMarker) + "/") == 0)
    Fail("Tar archive supplies reserved installation metadata");
  return result;
}

void CheckSymlink(const std::string &path, const std::string &target) {
  if (target.empty() || target.size() >= PATH_MAX || target.find('\0') != std::string::npos)
    Fail("Invalid tar symlink target");
  size_t depth = 0;
  if (target.front() != '/') depth = Components(path).size() - 1;
  for (const auto &part : Components(target)) {
    if (part == ".") continue;
    if (part == "..") {
      if (depth == 0) Fail("Tar symlink escapes the guest root");
      --depth;
    } else {
      if (part.size() > NAME_MAX) Fail("Tar symlink component is too long");
      ++depth;
    }
  }
  // Absolute links are Linux guest paths. Only the runtime resolves them;
  // extraction and verification never follow archive-created symlinks.
}

class Gzip {
 public:
  explicit Gzip(const char *path) : file_(open(path, O_RDONLY | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK)) {
    if (file_.get() < 0) SystemFail("Open ARM64 root archive");
    struct stat info = {};
    if (fstat(file_.get(), &info) != 0) SystemFail("Stat ARM64 root archive");
    if (!S_ISREG(info.st_mode) || info.st_size <= 0 || static_cast<uint64_t>(info.st_size) > kMaxArchive)
      Fail("ARM64 root archive must be a regular gzip file of at most 512 MiB");
    if (inflateInit2(&stream_, MAX_WBITS + 16) != Z_OK) Fail("Initialize gzip decoder");
    initialized_ = true;
  }
  ~Gzip() { if (initialized_) inflateEnd(&stream_); }
  size_t readBytes(void *output, size_t size) {
    auto *bytes = static_cast<uint8_t *>(output);
    size_t written = 0;
    while (written < size && !ended_) {
      if (stream_.avail_in == 0) {
        ssize_t count = Read(file_.get(), input_.data(), input_.size());
        if (count == 0) Fail("Truncated gzip stream");
        compressed_ += static_cast<uint64_t>(count);
        if (compressed_ > kMaxArchive) Fail("Compressed rootfs exceeds 512 MiB");
        stream_.next_in = input_.data();
        stream_.avail_in = static_cast<uInt>(count);
      }
      uInt request = static_cast<uInt>(std::min(size - written, size_t(65536)));
      stream_.next_out = bytes + written;
      stream_.avail_out = request;
      uInt before = stream_.avail_in;
      int status = inflate(&stream_, Z_NO_FLUSH);
      size_t produced = request - stream_.avail_out;
      written += produced;
      expanded_ += produced;
      if (expanded_ > kMaxTar) Fail("Rootfs tar stream exceeds extraction limit");
      if (status == Z_STREAM_END) {
        uint8_t extra = 0;
        if (stream_.avail_in != 0 || Read(file_.get(), &extra, 1) != 0)
          Fail("Trailing bytes or multiple gzip streams in root archive");
        ended_ = true;
      } else if (status != Z_OK || (produced == 0 && stream_.avail_in == before)) {
        Fail("Invalid gzip stream or gzip checksum");
      }
    }
    return written;
  }
  void exact(void *bytes, size_t size) {
    if (readBytes(bytes, size) != size) Fail("Truncated tar archive");
  }
  void padding(uint64_t size) {
    size_t count = static_cast<size_t>((512 - size % 512) % 512);
    std::array<uint8_t, 512> block = {};
    exact(block.data(), count);
    if (!std::all_of(block.begin(), block.begin() + count, [](uint8_t b) { return b == 0; }))
      Fail("Nonzero tar padding");
  }
  void finish() {
    std::array<uint8_t, 65536> block = {};
    uint64_t padding = 0;
    for (;;) {
      size_t count = readBytes(block.data(), block.size());
      if (count == 0) break;
      padding += count;
      if (padding > kMaxMetadata || !std::all_of(block.begin(), block.begin() + count,
                                               [](uint8_t b) { return b == 0; }))
        Fail("Invalid data after tar end marker");
    }
  }
 private:
  FD file_;
  z_stream stream_ = {};
  std::array<uint8_t, 65536> input_ = {};
  uint64_t compressed_ = 0;
  uint64_t expanded_ = 0;
  bool initialized_ = false;
  bool ended_ = false;
};

using DirectoryModes = std::map<std::string, mode_t>;

void RememberDirectory(DirectoryModes &directories, const std::string &path) {
  if (directories.find(path) != directories.end()) return;
  // Implicit parents consume inodes and metadata just like explicit tar entries.
  if (directories.size() >= kMaxEntries) Fail("Rootfs archive creates too many directories");
  directories.emplace(path, 0755);
}

FD Parent(int root, const std::string &path, DirectoryModes *create = nullptr) {
  FD current(fcntl(root, F_DUPFD_CLOEXEC, 0));
  if (current.get() < 0) SystemFail("Duplicate root directory");
  auto parts = Components(path);
  std::string relative;
  for (size_t index = 0; index + 1 < parts.size(); ++index) {
    const auto &part = parts[index];
    if (!relative.empty()) relative += '/';
    relative += part;
    if (create) RememberDirectory(*create, relative);
    if (create && mkdirat(current.get(), part.c_str(), 0700) != 0 && errno != EEXIST)
      SystemFail("Create rootfs parent");
    FD next(openat(current.get(), part.c_str(), O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC));
    if (next.get() < 0) SystemFail("Open rootfs parent without symlinks");
    current = std::move(next);
  }
  return current;
}

std::string Leaf(const std::string &path) {
  size_t slash = path.rfind('/');
  return slash == std::string::npos ? path : path.substr(slash + 1);
}

FD OpenFile(int root, const std::string &path) {
  FD parent = Parent(root, path);
  FD file(openat(parent.get(), Leaf(path).c_str(), O_RDONLY | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK));
  if (file.get() < 0) SystemFail("Open installed " + path);
  struct stat info = {};
  if (fstat(file.get(), &info) != 0) SystemFail("Stat installed " + path);
  if (!S_ISREG(info.st_mode)) Fail("Installed rootfs file is not regular: " + path);
  return file;
}

void VerifyELF(int root, const std::string &path) {
  FD file = OpenFile(root, path);
  std::array<uint8_t, 64> header = {};
  size_t count = 0;
  while (count < header.size()) {
    ssize_t size = Read(file.get(), header.data() + count, header.size() - count);
    if (size == 0) break;
    count += static_cast<size_t>(size);
  }
  struct stat info = {};
  if (fstat(file.get(), &info) != 0) SystemFail("Stat ARM64 executable");
  const uint8_t elf[] = {0x7f, 'E', 'L', 'F', 2, 1, 1};
  if (count != header.size() || memcmp(header.data(), elf, sizeof(elf)) != 0 ||
      header[18] != 183 || header[19] != 0 || (info.st_mode & 0111) == 0)
    Fail("Rootfs must contain an executable AArch64 ELF64 " + path);
}

void VerifyRoot(int root) {
  VerifyELF(root, "bin/busybox");
  VerifyELF(root, "sbin/apk");
  // Keep the real package manager, its repositories, and its installed database.
  for (const auto &path : {"etc/apk/repositories", "lib/apk/db/installed"}) {
    FD file = OpenFile(root, path);
    struct stat info = {};
    if (fstat(file.get(), &info) != 0) SystemFail("Stat package metadata");
    if (info.st_size == 0) Fail("Rootfs package metadata is empty");
  }
}

struct Overrides {
  bool hasPath = false;
  bool hasLink = false;
  bool hasSize = false;
  std::string path;
  std::string link;
  uint64_t size = 0;
};

void Pax(const std::string &data, Overrides &pending, bool global) {
  size_t offset = 0;
  while (offset < data.size()) {
    size_t space = data.find(' ', offset);
    if (space == std::string::npos || space - offset > 20) Fail("Malformed PAX record");
    uint64_t length = Decimal(data.substr(offset, space - offset));
    if (length <= space - offset + 3 || length > data.size() - offset)
      Fail("Invalid PAX record length");
    size_t end = offset + static_cast<size_t>(length);
    size_t equals = data.find('=', space + 1);
    if (equals == std::string::npos || equals == space + 1 || equals >= end - 1 || data[end - 1] != '\n')
      Fail("Malformed PAX key/value");
    std::string key = data.substr(space + 1, equals - space - 1);
    std::string value = data.substr(equals + 1, end - equals - 2);
    if (value.find('\0') != std::string::npos || key.find('\0') != std::string::npos ||
        key.compare(0, 10, "GNU.sparse") == 0) Fail("Unsupported PAX metadata");
    if (key == "path" || key == "linkpath" || key == "size") {
      if (global) Fail("Global PAX path and size overrides are unsupported");
      if (key == "path") { pending.hasPath = true; pending.path = value; }
      if (key == "linkpath") { pending.hasLink = true; pending.link = value; }
      if (key == "size") { pending.hasSize = true; pending.size = Decimal(value); }
    }
    offset = end;
  }
}

struct Link {
  std::string path;
  std::string target;
  bool hard;
};

mode_t Extract(int root, const char *archive) {
  Gzip gzip(archive);
  DirectoryModes directories = {{"", 0755}};
  std::set<std::string> seen;
  std::set<std::string> regular;
  std::vector<Link> links;
  std::map<std::string, std::string> hardTargets;
  Overrides pending;
  uint64_t expanded = 0;
  uint64_t metadata = 0;
  size_t entries = 0;
  bool extensionPending = false;
  for (;;) {
    std::array<uint8_t, 512> header = {};
    gzip.exact(header.data(), header.size());
    if (std::all_of(header.begin(), header.end(), [](uint8_t b) { return b == 0; })) {
      if (extensionPending) Fail("Tar ends with unapplied extension metadata");
      gzip.exact(header.data(), header.size());
      if (!std::all_of(header.begin(), header.end(), [](uint8_t b) { return b == 0; }))
        Fail("Tar requires two zero end blocks");
      gzip.finish();
      break;
    }
    if (++entries > kMaxEntries) Fail("Rootfs archive has too many entries");
    uint64_t checksum = Octal(header.data() + 148, 8);
    uint64_t sum = 0;
    for (size_t i = 0; i < header.size(); ++i) sum += i >= 148 && i < 156 ? ' ' : header[i];
    if (sum != checksum) Fail("Invalid tar header checksum");
    std::string magic = Field(header.data() + 257, 6);
    if (!magic.empty() && magic != "ustar" && magic != "ustar ") Fail("Unsupported tar format");
    std::string name = Field(header.data(), 100);
    if (magic == "ustar") {
      std::string prefix = Field(header.data() + 345, 155);
      if (!prefix.empty()) name = prefix + '/' + name;
    }
    // Validate raw names even when a PAX/GNU path override is present.
    RelativePath(name);
    uint64_t size = Octal(header.data() + 124, 12);
    uint64_t mode = Octal(header.data() + 100, 8);
    char type = static_cast<char>(header[156]);
    if (type == 'x' || type == 'g' || type == 'L' || type == 'K') {
      metadata += size;
      if (size > kMaxMetadata || metadata > 8 * kMaxMetadata) Fail("Excessive tar extension metadata");
      std::string data(static_cast<size_t>(size), '\0');
      gzip.exact(data.data(), data.size());
      gzip.padding(size);
      if (type == 'x' || type == 'g') Pax(data, pending, type == 'g');
      else {
        if (data.empty() || data.back() != '\0') Fail("Unterminated GNU tar long name");
        data.pop_back();
        if (type == 'L') { pending.hasPath = true; pending.path = data; }
        else { pending.hasLink = true; pending.link = data; }
      }
      extensionPending = true;
      continue;
    }
    std::string path = RelativePath(pending.hasPath ? pending.path : name);
    std::string target = pending.hasLink ? pending.link : Field(header.data() + 157, 100);
    if (pending.hasSize) size = pending.size;
    if (pending.hasLink && type != '1' && type != '2') Fail("Tar link override on a non-link entry");
    pending = Overrides{};
    extensionPending = false;
    if (!seen.insert(path).second) Fail("Duplicate rootfs archive path");
    if (path.empty()) {
      if (type != '5' || size != 0) Fail("Tar root entry must be an empty directory");
      directories[""] = static_cast<mode_t>(mode & 01777);
      continue;
    }
    FD parent = Parent(root, path, &directories);
    std::string leaf = Leaf(path);
    if (type == '0' || type == '\0') {
      if (size > kMaxFile || size > kMaxExpanded - expanded) Fail("Rootfs file or expanded size limit exceeded");
      expanded += size;
      FD file(openat(parent.get(), leaf.c_str(), O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600));
      if (file.get() < 0) SystemFail("Create rootfs file");
      std::array<uint8_t, 65536> buffer = {};
      uint64_t remaining = size;
      while (remaining != 0) {
        size_t count = static_cast<size_t>(std::min(remaining, uint64_t(buffer.size())));
        gzip.exact(buffer.data(), count);
        Write(file.get(), buffer.data(), count);
        remaining -= count;
      }
      if (fchmod(file.get(), static_cast<mode_t>(mode & 0777)) != 0) SystemFail("Set rootfs file mode");
      Sync(file.get());
      regular.insert(path);
      gzip.padding(size);
    } else if (type == '5') {
      if (size != 0) Fail("Tar directory contains file data");
      RememberDirectory(directories, path);
      if (mkdirat(parent.get(), leaf.c_str(), 0700) != 0 && errno != EEXIST) SystemFail("Create rootfs directory");
      FD directory(openat(parent.get(), leaf.c_str(), O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC));
      if (directory.get() < 0) SystemFail("Open rootfs directory");
      directories[path] = static_cast<mode_t>(mode & 01777);
    } else if (type == '1' || type == '2') {
      if (size != 0) Fail("Tar link contains file data");
      if (type == '1') {
        target = RelativePath(target);
        if (target.empty()) Fail("Empty tar hardlink target");
        hardTargets.emplace(path, target);
      } else CheckSymlink(path, target);
      links.push_back({path, target, type == '1'});
    } else Fail("Unsupported tar node type; devices, FIFOs and sparse files are not allowed");
  }
  // Resolve hardlinks only through archive-declared regular files/hardlinks.
  // No symlink exists yet, and all parent lookup is descriptor-relative.
  for (const auto &link : links) {
    if (!link.hard) continue;
    std::string target = link.target;
    std::set<std::string> chain = {link.path};
    while (!regular.count(target)) {
      auto next = hardTargets.find(target);
      if (next == hardTargets.end() || !chain.insert(target).second || chain.size() > 64)
        Fail("Tar hardlink has a missing, cyclic, or non-regular target");
      target = next->second;
    }
    FD sourceParent = Parent(root, target);
    struct stat info = {};
    if (fstatat(sourceParent.get(), Leaf(target).c_str(), &info, AT_SYMLINK_NOFOLLOW) != 0)
      SystemFail("Stat rootfs hardlink target");
    if (!S_ISREG(info.st_mode)) Fail("Tar hardlink target is not regular");
    FD parent = Parent(root, link.path);
    if (linkat(sourceParent.get(), Leaf(target).c_str(), parent.get(), Leaf(link.path).c_str(), 0) != 0)
      SystemFail("Create rootfs hardlink");
  }
  for (const auto &link : links) {
    if (link.hard) continue;
    FD parent = Parent(root, link.path);
    if (symlinkat(link.target.c_str(), parent.get(), Leaf(link.path).c_str()) != 0)
      SystemFail("Create rootfs symlink");
  }
  std::vector<std::pair<std::string, mode_t>> modes(directories.begin(), directories.end());
  std::sort(modes.begin(), modes.end(), [](const auto &a, const auto &b) { return a.first.size() > b.first.size(); });
  for (const auto &entry : modes) {
    if (entry.first.empty()) continue;
    FD parent = Parent(root, entry.first);
    FD directory(openat(parent.get(), Leaf(entry.first).c_str(), O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC));
    if (directory.get() < 0 || fchmod(directory.get(), entry.second) != 0) SystemFail("Set rootfs directory mode");
    Sync(directory.get());
  }
  VerifyRoot(root);
  if ((directories[""] & 0500) != 0500) Fail("Rootfs root mode must allow owner read and search");
  return directories[""];
}

bool Installed(int parent) {
  struct stat info = {};
  if (fstatat(parent, kRootName, &info, AT_SYMLINK_NOFOLLOW) != 0) {
    if (errno == ENOENT) return false;
    SystemFail("Stat ARM64 root directory");
  }
  if (!S_ISDIR(info.st_mode)) Fail("Existing ARM64 root is not a directory; retained unchanged");
  FD root(openat(parent, kRootName, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC));
  if (root.get() < 0) SystemFail("Open existing ARM64 root");
  FD marker = OpenFile(root.get(), kMarker);
  struct stat metadata = {};
  if (fstat(marker.get(), &metadata) != 0) SystemFail("Stat ARM64 installation metadata");
  if (metadata.st_size <= 0 || metadata.st_size > 4096) Fail("Invalid ARM64 installation metadata; root retained");
  std::array<uint8_t, 4096> bytes = {};
  size_t length = 0;
  for (;;) {
    ssize_t count = Read(marker.get(), bytes.data() + length, bytes.size() - length);
    if (count == 0) break;
    length += static_cast<size_t>(count);
    if (length == bytes.size()) Fail("Oversized ARM64 installation metadata");
  }
  id value = [NSJSONSerialization JSONObjectWithData:[NSData dataWithBytes:bytes.data() length:length]
                                            options:0 error:nullptr];
  if (![value isKindOfClass:NSDictionary.class] || ![value[@"architecture"] isEqual:@"aarch64"] ||
      ![value[@"format"] isEqual:@1]) Fail("Unrecognized ARM64 installation metadata; root retained");
  VerifyRoot(root.get());
  return true;
}

void RemoveImport(int parent, const char *name) noexcept {
  struct stat info = {};
  if (fstatat(parent, name, &info, AT_SYMLINK_NOFOLLOW) != 0) return;
  if (!S_ISDIR(info.st_mode)) { unlinkat(parent, name, 0); return; }
  // Directory modes may already have been finalized before a later failure.
  if (fchmodat(parent, name, 0700, AT_SYMLINK_NOFOLLOW) != 0) return;
  int descriptor = openat(parent, name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
  if (descriptor < 0) return;
  DIR *directory = fdopendir(descriptor);
  if (!directory) { close(descriptor); return; }
  while (struct dirent *entry = readdir(directory)) {
    if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) continue;
    RemoveImport(dirfd(directory), entry->d_name);
  }
  closedir(directory);
  unlinkat(parent, name, AT_REMOVEDIR);
}

class Import {
 public:
  Import(int parent, std::string name) : parent_(parent), name_(std::move(name)) {
    if (mkdirat(parent_, name_.c_str(), 0700) != 0) SystemFail("Create private ARM64 import directory");
  }
  ~Import() { if (!published_) RemoveImport(parent_, name_.c_str()); }
  const char *name() const { return name_.c_str(); }
  void publish() {
    if (renameatx_np(parent_, name_.c_str(), parent_, kRootName, RENAME_EXCL) != 0)
      SystemFail("Atomically publish ARM64 root without replacing existing data");
    published_ = true;
    // Publication has succeeded; an unsupported parent fsync cannot roll it back.
    (void)fsync(parent_);
  }
 private:
  int parent_;
  std::string name_;
  bool published_ = false;
};
}  // namespace

NSString *ORTArm64PrepareRootAtURL(NSURL *runtimeDirectory, NSURL *archive, NSString **errorOut) {
  if (errorOut) *errorOut = nil;
  try {
    if (!runtimeDirectory.isFileURL || !runtimeDirectory.path.isAbsolutePath)
      Fail("ARM64 runtime directory must be an absolute file URL");
    NSError *error = nil;
    NSFileManager *files = NSFileManager.defaultManager;
    // This is the bridge's Application Support directory. It is separate from
    // the old runtime; neither legacy rootfs names nor their contents are used.
    if (![files createDirectoryAtURL:runtimeDirectory withIntermediateDirectories:YES attributes:nil error:&error])
      Fail(error.localizedDescription.UTF8String ?: "Create ARM64 runtime directory");
    FD directory(open(runtimeDirectory.fileSystemRepresentation, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC));
    if (directory.get() < 0) SystemFail("Open ARM64 runtime directory without symlinks");
    FD lock(openat(directory.get(), ".arm64-root.lock", O_RDWR | O_CREAT | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK, 0600));
    if (lock.get() < 0) SystemFail("Open ARM64 root installation lock");
    struct stat lockInfo = {};
    if (fstat(lock.get(), &lockInfo) != 0) SystemFail("Stat ARM64 installation lock");
    if (!S_ISREG(lockInfo.st_mode) || lockInfo.st_nlink != 1) Fail("Invalid ARM64 installation lock");
    int result;
    do { result = flock(lock.get(), LOCK_EX); } while (result < 0 && errno == EINTR);
    if (result != 0) SystemFail("Lock ARM64 root installation");
    if (!Installed(directory.get())) {
      if (!archive.isFileURL) Fail("Bundled arm64-rootfs.tar.gz is missing");
      std::string name = ".rootfs-arm64.import-";
      name += NSUUID.UUID.UUIDString.UTF8String;
      Import importing(directory.get(), std::move(name));
      FD root(openat(directory.get(), importing.name(), O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC));
      if (root.get() < 0) SystemFail("Open private ARM64 import directory");
      mode_t rootMode = Extract(root.get(), archive.fileSystemRepresentation);
      const char metadata[] = "{\"architecture\":\"aarch64\",\"format\":1}\n";
      FD marker(openat(root.get(), kMarker, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600));
      if (marker.get() < 0) SystemFail("Create ARM64 installation metadata");
      Write(marker.get(), metadata, sizeof(metadata) - 1);
      Sync(marker.get());
      if (fchmod(root.get(), rootMode) != 0) SystemFail("Set rootfs root mode");
      Sync(root.get());
      importing.publish();
    }
    return [runtimeDirectory URLByAppendingPathComponent:@"rootfs-arm64" isDirectory:YES].path;
  } catch (const std::exception &error) {
    if (errorOut) *errorOut = [NSString stringWithUTF8String:error.what()] ?: @"ARM64 root installation failed";
    return nil;
  }
}
