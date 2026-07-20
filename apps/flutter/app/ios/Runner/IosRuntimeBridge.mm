#import <Foundation/Foundation.h>

#import <NodeMobile/NodeMobile.h>
#import <OperitToybox/operit_toybox.h>
#import <Python/Python.h>

#import <arpa/inet.h>
#import <netinet/in.h>
#import <sys/socket.h>
#import <sys/types.h>
#import <unistd.h>

#include <cstdlib>
#include <cstring>

#import "OperitFlutterBridge.h"

static NSString *const ORTIosPythonTerminalType = @"python";
static NSString *const ORTIosNodeTerminalType = @"node";
static NSString *const ORTIosToyboxTerminalType = @"toybox";
static NSString *const ORTIosScientificFrameworkName = @"OperitPythonScientific";
static NSInteger const ORTIosNodeStartupTimeoutMilliseconds = 10000;

@interface ORTIosTerminalSession : NSObject
@property(nonatomic, copy) NSString *sessionId;
@property(nonatomic, copy) NSString *sessionName;
@property(nonatomic, copy) NSString *terminalType;
@property(nonatomic, copy) NSString *workingDirectory;
@property(nonatomic) NSUInteger rows;
@property(nonatomic) NSUInteger columns;
@property(nonatomic) BOOL active;
@property(nonatomic) BOOL commandRunning;
@property(nonatomic) int toyboxExitCode;
@property(nonatomic, strong) NSMutableArray<NSString *> *runtimeOutputLines;
@property(nonatomic, strong) NSMutableString *toyboxInput;
@property(nonatomic, strong) NSMutableString *toyboxOutput;
@property(nonatomic, strong) NSMutableString *toyboxScreen;
@end

@implementation ORTIosTerminalSession

/// Initializes the mutable state used by one embedded interpreter session.
- (instancetype)init {
  self = [super init];
  if (self) {
    _active = YES;
    _commandRunning = NO;
    _toyboxExitCode = 0;
    _runtimeOutputLines = [NSMutableArray array];
    _toyboxInput = [NSMutableString string];
    _toyboxOutput = [NSMutableString string];
    _toyboxScreen = [NSMutableString string];
  }
  return self;
}

@end

static NSMutableDictionary<NSString *, ORTIosTerminalSession *> *ORTIosSessions;
static NSMutableDictionary<NSString *, NSString *> *ORTIosNamedSessions;
static NSUInteger ORTIosNextPythonSessionIdentifier = 1;
static NSUInteger ORTIosNextToyboxSessionIdentifier = 1;
static dispatch_once_t ORTIosSessionStoreOnce;
static dispatch_once_t ORTIosPythonOnce;
static dispatch_once_t ORTIosNodeOnce;
static NSString *ORTIosPythonInitializationError;
static NSString *ORTIosNodeInitializationError;
static wchar_t *ORTIosPythonHome;
static NSString *ORTIosNodeReadyPath;
static dispatch_queue_t ORTIosToyboxQueue;
static dispatch_once_t ORTIosToyboxQueueOnce;

/// Creates the shared terminal-session stores exactly once for the process.
static void ORTIosEnsureSessionStore(void) {
  dispatch_once(&ORTIosSessionStoreOnce, ^{
    ORTIosSessions = [NSMutableDictionary dictionary];
    ORTIosNamedSessions = [NSMutableDictionary dictionary];
  });
}

/// Returns a structured success response for the Rust iOS runtime bridge.
static NSDictionary *ORTIosSuccess(NSDictionary *result) {
  return @{ @"result" : result };
}

/// Returns a structured error response for the Rust iOS runtime bridge.
static NSDictionary *ORTIosFailure(NSString *message) {
  return @{ @"error" : message };
}

/// Serializes one bridge response into caller-owned UTF-8 C storage.
static char *ORTIosSerializeResponse(NSDictionary *response) {
  NSError *error = nil;
  NSData *data = [NSJSONSerialization dataWithJSONObject:response options:0 error:&error];
  if (data == nil) {
    NSString *message = [NSString stringWithFormat:@"iOS runtime response encoding failed: %@", error.localizedDescription];
    data = [NSJSONSerialization dataWithJSONObject:ORTIosFailure(message) options:0 error:nil];
  }
  char *value = static_cast<char *>(malloc(data.length + 1));
  if (value == nullptr) {
    return nullptr;
  }
  memcpy(value, data.bytes, data.length);
  value[data.length] = '\0';
  return value;
}

/// Requires a JSON object value with the requested key to be a non-empty string.
static NSString *ORTIosRequiredString(NSDictionary *request, NSString *key, NSString **error) {
  id value = request[key];
  if (![value isKindOfClass:[NSString class]] || [value length] == 0) {
    *error = [NSString stringWithFormat:@"iOS runtime request requires non-empty %@", key];
    return nil;
  }
  return value;
}

/// Requires a positive JSON integer value that fits the local unsigned size type.
static NSUInteger ORTIosRequiredPositiveInteger(NSDictionary *request, NSString *key, NSString **error) {
  id value = request[key];
  if (![value isKindOfClass:[NSNumber class]] || [value unsignedIntegerValue] == 0) {
    *error = [NSString stringWithFormat:@"iOS runtime request requires positive %@", key];
    return 0;
  }
  return [value unsignedIntegerValue];
}

/// Returns the permanent application-support workspace used by embedded interpreters.
static NSString *ORTIosWorkspaceDirectory(NSString **error) {
  NSURL *applicationSupport = [[NSFileManager defaultManager]
      URLForDirectory:NSApplicationSupportDirectory
             inDomain:NSUserDomainMask
    appropriateForURL:nil
               create:YES
                error:nil];
  if (applicationSupport == nil) {
    *error = @"iOS application support directory is unavailable";
    return nil;
  }
  NSURL *workspace = [applicationSupport URLByAppendingPathComponent:@"operit-runtime/workspace" isDirectory:YES];
  if (![[NSFileManager defaultManager] createDirectoryAtURL:workspace
                                 withIntermediateDirectories:YES
                                                  attributes:nil
                                                       error:nil]) {
    *error = @"iOS embedded runtime workspace cannot be created";
    return nil;
  }
  return workspace.path;
}

/// Locates a required resource directory embedded by the iOS Runner target.
static NSString *ORTIosResourceDirectory(NSString *name, NSString **error) {
  NSString *path = [[NSBundle mainBundle] pathForResource:name ofType:nil];
  if (path == nil) {
    *error = [NSString stringWithFormat:@"iOS embedded runtime resource is missing: %@", name];
  }
  return path;
}

/// Adds one resource path at the beginning of the embedded interpreter sys.path list.
static BOOL ORTIosInsertPythonPath(NSString *path, NSString **error) {
  PyObject *sys = PyImport_ImportModule("sys");
  if (sys == nullptr) {
    *error = @"CPython sys module cannot be imported";
    PyErr_Clear();
    return NO;
  }
  PyObject *paths = PyObject_GetAttrString(sys, "path");
  Py_DECREF(sys);
  if (paths == nullptr) {
    *error = @"CPython sys.path is unavailable";
    PyErr_Clear();
    return NO;
  }
  PyObject *pythonPath = PyUnicode_FromString(path.UTF8String);
  if (pythonPath == nullptr) {
    Py_DECREF(paths);
    *error = @"CPython cannot encode an embedded resource path";
    PyErr_Clear();
    return NO;
  }
  int result = PyList_Insert(paths, 0, pythonPath);
  Py_DECREF(pythonPath);
  Py_DECREF(paths);
  if (result != 0) {
    *error = @"CPython cannot add an embedded resource path";
    PyErr_Clear();
    return NO;
  }
  return YES;
}

/// Initializes the one CPython instance bundled with the iOS application.
static void ORTIosInitializePython(void) {
  @autoreleasepool {
    NSString *error = nil;
    NSString *pythonRoot = ORTIosResourceDirectory(@"python", &error);
    if (pythonRoot == nil) {
      ORTIosPythonInitializationError = error;
      return;
    }
    ORTIosPythonHome = Py_DecodeLocale(pythonRoot.fileSystemRepresentation, nullptr);
    if (ORTIosPythonHome == nullptr) {
      ORTIosPythonInitializationError = @"CPython cannot decode its embedded resource directory";
      return;
    }
    Py_SetPythonHome(ORTIosPythonHome);
    Py_Initialize();
    if (!Py_IsInitialized()) {
      ORTIosPythonInitializationError = @"CPython embedded runtime initialization failed";
      return;
    }
    PyGILState_STATE gil = PyGILState_Ensure();
    BOOL pathsReady = ORTIosInsertPythonPath([pythonRoot stringByAppendingPathComponent:@"lib/python3.13/site-packages"], &error);
    NSString *scientificPath = [[[NSBundle mainBundle] privateFrameworksPath]
        stringByAppendingPathComponent:[NSString stringWithFormat:@"%@.framework/Resources/python/site-packages", ORTIosScientificFrameworkName]];
    if (pathsReady) {
      pathsReady = ORTIosInsertPythonPath(scientificPath, &error);
    }
    NSString *runtimeSourcePath = [pythonRoot stringByAppendingPathComponent:@"operit_terminal_runtime.py"];
    NSString *runtimeSource = [NSString stringWithContentsOfFile:runtimeSourcePath encoding:NSUTF8StringEncoding error:nil];
    if (runtimeSource == nil) {
      pathsReady = NO;
      error = @"iOS CPython terminal runtime source is missing";
    }
    if (pathsReady && PyRun_SimpleString(runtimeSource.UTF8String) != 0) {
      pathsReady = NO;
      error = @"iOS CPython terminal runtime source failed to initialize";
      PyErr_Clear();
    }
    PyGILState_Release(gil);
    if (!pathsReady) {
      ORTIosPythonInitializationError = error;
    }
  }
}

/// Ensures that the embedded CPython interpreter has been initialized successfully.
static BOOL ORTIosEnsurePython(NSString **error) {
  dispatch_once(&ORTIosPythonOnce, ORTIosInitializePython);
  if (ORTIosPythonInitializationError != nil) {
    *error = ORTIosPythonInitializationError;
    return NO;
  }
  return YES;
}

/// Invokes one named Python runtime function with a required session identifier and optional input.
static NSString *ORTIosCallPythonStringFunction(NSString *functionName,
                                                 NSString *sessionId,
                                                 NSString *input,
                                                 NSString **error) {
  if (!ORTIosEnsurePython(error)) {
    return nil;
  }
  PyGILState_STATE gil = PyGILState_Ensure();
  PyObject *mainModule = PyImport_AddModule("__main__");
  PyObject *function = PyObject_GetAttrString(mainModule, functionName.UTF8String);
  if (function == nullptr || !PyCallable_Check(function)) {
    Py_XDECREF(function);
    PyGILState_Release(gil);
    *error = [NSString stringWithFormat:@"CPython runtime function is unavailable: %@", functionName];
    PyErr_Clear();
    return nil;
  }
  PyObject *result = nullptr;
  if (input == nil) {
    result = PyObject_CallFunction(function, "s", sessionId.UTF8String);
  } else {
    result = PyObject_CallFunction(function, "ss", sessionId.UTF8String, input.UTF8String);
  }
  Py_DECREF(function);
  if (result == nullptr) {
    PyGILState_Release(gil);
    *error = [NSString stringWithFormat:@"CPython runtime function failed: %@", functionName];
    PyErr_Clear();
    return nil;
  }
  NSString *value = nil;
  if (result != Py_None) {
    const char *text = PyUnicode_AsUTF8(result);
    if (text == nullptr) {
      Py_DECREF(result);
      PyGILState_Release(gil);
      *error = [NSString stringWithFormat:@"CPython runtime function returned non-text output: %@", functionName];
      PyErr_Clear();
      return nil;
    }
    value = [NSString stringWithUTF8String:text];
  }
  Py_DECREF(result);
  PyGILState_Release(gil);
  return value ?: @"";
}

/// Creates one persistent CPython terminal namespace.
static BOOL ORTIosCreatePythonTerminal(NSString *sessionId, NSString **error) {
  return ORTIosCallPythonStringFunction(@"operit_create_terminal", sessionId, nil, error) != nil;
}

/// Runs the Node Mobile runtime service on its own persistent application thread.
static void ORTIosRunNodeService(NSString *scriptPath, NSString *readyPath, NSString *resourceRoot) {
  @autoreleasepool {
    char *arguments[] = {
      const_cast<char *>("node"),
      const_cast<char *>(scriptPath.fileSystemRepresentation),
      const_cast<char *>(readyPath.fileSystemRepresentation),
      const_cast<char *>(resourceRoot.fileSystemRepresentation),
      nullptr,
    };
    node_start(4, arguments);
  }
}

/// Starts the bundled Node Mobile service and waits for its loopback endpoint publication.
static void ORTIosInitializeNode(void) {
  @autoreleasepool {
    NSString *error = nil;
    NSString *resourceRoot = ORTIosResourceDirectory(@"node", &error);
    if (resourceRoot == nil) {
      ORTIosNodeInitializationError = error;
      return;
    }
    NSString *scriptPath = [resourceRoot stringByAppendingPathComponent:@"operit_node_terminal_service.js"];
    if (![[NSFileManager defaultManager] isReadableFileAtPath:scriptPath]) {
      ORTIosNodeInitializationError = @"iOS Node terminal service source is missing";
      return;
    }
    NSString *workspace = ORTIosWorkspaceDirectory(&error);
    if (workspace == nil) {
      ORTIosNodeInitializationError = error;
      return;
    }
    ORTIosNodeReadyPath = [workspace stringByAppendingPathComponent:@"node-terminal-port"];
    [[NSFileManager defaultManager] removeItemAtPath:ORTIosNodeReadyPath error:nil];
    dispatch_async(dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0), ^{
      ORTIosRunNodeService(scriptPath, ORTIosNodeReadyPath, resourceRoot);
    });
    NSInteger remaining = ORTIosNodeStartupTimeoutMilliseconds;
    while (remaining > 0) {
      NSString *port = [NSString stringWithContentsOfFile:ORTIosNodeReadyPath encoding:NSUTF8StringEncoding error:nil];
      if (port.length > 0) {
        return;
      }
      [NSThread sleepForTimeInterval:0.02];
      remaining -= 20;
    }
    ORTIosNodeInitializationError = @"embedded Node Mobile service did not publish a loopback endpoint";
  }
}

/// Reads the loopback port published by the persistent Node Mobile runtime service.
static uint16_t ORTIosNodePort(NSString **error) {
  NSString *portText = [NSString stringWithContentsOfFile:ORTIosNodeReadyPath encoding:NSUTF8StringEncoding error:nil];
  NSInteger port = portText.integerValue;
  if (port <= 0 || port > UINT16_MAX) {
    *error = @"embedded Node Mobile service endpoint is invalid";
    return 0;
  }
  return static_cast<uint16_t>(port);
}

/// Sends one JSON request to the loopback-only Node Mobile service and reads one JSON response.
static NSDictionary *ORTIosCallNodeService(NSDictionary *request, NSString **error) {
  dispatch_once(&ORTIosNodeOnce, ORTIosInitializeNode);
  if (ORTIosNodeInitializationError != nil) {
    *error = ORTIosNodeInitializationError;
    return nil;
  }
  uint16_t port = ORTIosNodePort(error);
  if (port == 0) {
    return nil;
  }
  NSData *requestData = [NSJSONSerialization dataWithJSONObject:request options:0 error:nil];
  int descriptor = socket(AF_INET, SOCK_STREAM, 0);
  if (descriptor < 0) {
    *error = @"embedded Node Mobile service socket cannot be created";
    return nil;
  }
  sockaddr_in address = {};
  address.sin_family = AF_INET;
  address.sin_port = htons(port);
  address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
  if (connect(descriptor, reinterpret_cast<sockaddr *>(&address), sizeof(address)) != 0) {
    close(descriptor);
    *error = @"embedded Node Mobile service is unreachable";
    return nil;
  }
  const uint8_t *bytes = static_cast<const uint8_t *>(requestData.bytes);
  size_t written = 0;
  while (written < requestData.length) {
    ssize_t count = send(descriptor, bytes + written, requestData.length - written, 0);
    if (count <= 0) {
      close(descriptor);
      *error = @"embedded Node Mobile service request write failed";
      return nil;
    }
    written += static_cast<size_t>(count);
  }
  shutdown(descriptor, SHUT_WR);
  NSMutableData *responseData = [NSMutableData data];
  uint8_t buffer[4096];
  for (;;) {
    ssize_t count = recv(descriptor, buffer, sizeof(buffer), 0);
    if (count < 0) {
      close(descriptor);
      *error = @"embedded Node Mobile service response read failed";
      return nil;
    }
    if (count == 0) {
      break;
    }
    [responseData appendBytes:buffer length:static_cast<NSUInteger>(count)];
  }
  close(descriptor);
  NSDictionary *response = [NSJSONSerialization JSONObjectWithData:responseData options:0 error:nil];
  if (![response isKindOfClass:[NSDictionary class]]) {
    *error = @"embedded Node Mobile service response is invalid";
    return nil;
  }
  if (![response[@"ok"] boolValue]) {
    *error = [response[@"error"] isKindOfClass:[NSString class]]
        ? response[@"error"]
        : @"embedded Node Mobile service rejected the request";
    return nil;
  }
  NSDictionary *result = response[@"result"];
  if (![result isKindOfClass:[NSDictionary class]]) {
    *error = @"embedded Node Mobile service result is invalid";
    return nil;
  }
  return result;
}

/// Requires an active terminal session stored by the native iOS bridge.
static ORTIosTerminalSession *ORTIosRequiredSession(NSString *sessionId, NSString **error) {
  ORTIosEnsureSessionStore();
  ORTIosTerminalSession *session = ORTIosSessions[sessionId];
  if (session == nil || !session.active) {
    *error = [NSString stringWithFormat:@"iOS embedded terminal session does not exist: %@", sessionId];
    return nil;
  }
  return session;
}

/// Appends complete runtime protocol lines emitted by one embedded interpreter operation.
static void ORTIosAppendRuntimeLines(ORTIosTerminalSession *session, NSString *output) {
  NSArray<NSString *> *parts = [output componentsSeparatedByString:@"\n"];
  for (NSUInteger index = 0; index + 1 < parts.count; index += 1) {
    [session.runtimeOutputLines addObject:parts[index]];
  }
}

/// Creates the serial execution queue that owns embedded Toybox state and standard descriptors.
static void ORTIosEnsureToyboxQueue(void) {
  dispatch_once(&ORTIosToyboxQueueOnce, ^{
    ORTIosToyboxQueue = dispatch_queue_create("app.operit.runtime.toybox", DISPATCH_QUEUE_SERIAL);
  });
}

/// Parses one complete Toybox applet line without assigning shell semantics to the terminal.
static NSArray<NSString *> *ORTIosParseToyboxArguments(NSString *source, NSString **error) {
  NSMutableArray<NSString *> *arguments = [NSMutableArray array];
  NSMutableString *current = [NSMutableString string];
  unichar quote = 0;
  BOOL escaped = NO;
  for (NSUInteger index = 0; index < source.length; index += 1) {
    unichar character = [source characterAtIndex:index];
    if (escaped) {
      [current appendFormat:@"%C", character];
      escaped = NO;
      continue;
    }
    if (character == '\\') {
      escaped = YES;
      continue;
    }
    if (quote != 0) {
      if (character == quote) {
        quote = 0;
      } else {
        [current appendFormat:@"%C", character];
      }
      continue;
    }
    if (character == '\'' || character == '"') {
      quote = character;
      continue;
    }
    if ([[NSCharacterSet whitespaceCharacterSet] characterIsMember:character]) {
      if (current.length > 0) {
        [arguments addObject:current.copy];
        [current setString:@""];
      }
      continue;
    }
    [current appendFormat:@"%C", character];
  }
  if (escaped || quote != 0) {
    *error = @"Toybox command line has an unterminated escape or quote";
    return nil;
  }
  if (current.length > 0) {
    [arguments addObject:current.copy];
  }
  return arguments;
}

/// Converts terminal-entered text into the display form produced by a line-buffered terminal.
static NSString *ORTIosToyboxDisplayInput(NSString *input) {
  NSString *normalized = [input stringByReplacingOccurrencesOfString:@"\r\n" withString:@"\n"];
  normalized = [normalized stringByReplacingOccurrencesOfString:@"\r" withString:@"\n"];
  return [normalized stringByReplacingOccurrencesOfString:@"\n" withString:@"\r\n"];
}

/// Returns the prompt displayed by one direct Toybox applet terminal.
static NSString *ORTIosToyboxPrompt(ORTIosTerminalSession *session) {
  NSString *name = session.workingDirectory.lastPathComponent;
  return [NSString stringWithFormat:@"%@ $ ", name.length == 0 ? @"workspace" : name];
}

/// Adds output to both the unread queue and retained transcript of one Toybox terminal.
static void ORTIosAppendToyboxOutput(ORTIosTerminalSession *session, NSString *output) {
  [session.toyboxOutput appendString:output];
  [session.toyboxScreen appendString:output];
}

/// Converts one UTF-8 output buffer returned by the embedded Toybox wrapper.
static NSString *ORTIosToyboxOutputString(const char *source, NSString **error) {
  if (source == nullptr) {
    *error = @"Toybox execution did not return an output buffer";
    return nil;
  }
  NSString *value = [[NSString alloc] initWithUTF8String:source];
  if (value == nil) {
    *error = @"Toybox emitted output that is not valid UTF-8";
  }
  return value;
}

/// Executes one direct Toybox applet line through the isolated in-process wrapper.
static NSString *ORTIosExecuteToyboxLine(ORTIosTerminalSession *session, NSString *line, NSString **error) {
  NSArray<NSString *> *arguments = ORTIosParseToyboxArguments(line, error);
  if (arguments == nil) {
    return nil;
  }
  if (arguments.count == 0) {
    return @"";
  }
  const char **argv = static_cast<const char **>(calloc(arguments.count, sizeof(const char *)));
  if (argv == nullptr) {
    *error = @"Toybox terminal cannot allocate command arguments";
    return nil;
  }
  for (NSUInteger index = 0; index < arguments.count; index += 1) {
    argv[index] = arguments[index].UTF8String;
    if (argv[index] == nullptr) {
      free(argv);
      *error = @"Toybox terminal command is not valid UTF-8";
      return nil;
    }
  }
  OperitToyboxResult result = {};
  __block int status = 0;
  ORTIosEnsureToyboxQueue();
  dispatch_sync(ORTIosToyboxQueue, ^{
    status = operit_toybox_run(
        session.workingDirectory.UTF8String,
        static_cast<int>(arguments.count),
        argv,
        "",
        &result);
  });
  free(argv);
  NSString *standardOutput = ORTIosToyboxOutputString(result.standard_output, error);
  if (standardOutput == nil) {
    operit_toybox_result_free(&result);
    return nil;
  }
  NSString *standardError = ORTIosToyboxOutputString(result.standard_error, error);
  if (standardError == nil) {
    operit_toybox_result_free(&result);
    return nil;
  }
  NSMutableString *output = [NSMutableString stringWithString:standardOutput];
  [output appendString:standardError];
  session.toyboxExitCode = result.exit_code;
  operit_toybox_result_free(&result);
  if (status != 0 && output.length == 0) {
    *error = @"Toybox execution failed without diagnostics";
    return nil;
  }
  return output;
}

/// Executes every complete direct Toybox applet line buffered by one terminal session.
static NSString *ORTIosWriteToyboxTerminal(ORTIosTerminalSession *session, NSString *input, NSString **error) {
  NSString *normalized = [input stringByReplacingOccurrencesOfString:@"\r\n" withString:@"\n"];
  normalized = [normalized stringByReplacingOccurrencesOfString:@"\r" withString:@"\n"];
  [session.toyboxInput appendString:normalized];
  NSArray<NSString *> *lines = [session.toyboxInput componentsSeparatedByString:@"\n"];
  [session.toyboxInput setString:lines.lastObject ?: @""];
  NSMutableString *emitted = [NSMutableString string];
  for (NSUInteger index = 0; index + 1 < lines.count; index += 1) {
    NSString *output = ORTIosExecuteToyboxLine(session, lines[index], error);
    if (output == nil) {
      return nil;
    }
    [emitted appendString:output];
    if (![output hasSuffix:@"\n"] && ![output hasSuffix:@"\r"]) {
      [emitted appendString:@"\r\n"];
    }
    [emitted appendString:ORTIosToyboxPrompt(session)];
  }
  return emitted;
}

/// Starts one explicit Toybox, Python, or Node terminal session through embedded native runtimes.
static NSDictionary *ORTIosStartTerminal(NSDictionary *request, NSString **error) {
  NSString *sessionName = ORTIosRequiredString(request, @"sessionName", error);
  if (sessionName == nil) {
    return nil;
  }
  NSString *terminalType = ORTIosRequiredString(request, @"terminalType", error);
  if (terminalType == nil) {
    return nil;
  }
  if (![terminalType isEqualToString:ORTIosToyboxTerminalType]
      && ![terminalType isEqualToString:ORTIosPythonTerminalType]
      && ![terminalType isEqualToString:ORTIosNodeTerminalType]) {
    *error = [NSString stringWithFormat:@"unsupported iOS embedded terminal type: %@", terminalType];
    return nil;
  }
  NSString *workingDirectory = ORTIosRequiredString(request, @"workingDir", error);
  if (workingDirectory == nil) {
    return nil;
  }
  NSUInteger rows = ORTIosRequiredPositiveInteger(request, @"rows", error);
  if (rows == 0) {
    return nil;
  }
  NSUInteger columns = ORTIosRequiredPositiveInteger(request, @"cols", error);
  if (columns == 0) {
    return nil;
  }
  ORTIosEnsureSessionStore();
  ORTIosTerminalSession *session = [[ORTIosTerminalSession alloc] init];
  session.sessionName = sessionName;
  session.terminalType = terminalType;
  session.workingDirectory = workingDirectory.stringByStandardizingPath;
  session.rows = rows;
  session.columns = columns;
  BOOL directory = NO;
  if (![[NSFileManager defaultManager] fileExistsAtPath:session.workingDirectory isDirectory:&directory] || !directory) {
    *error = [NSString stringWithFormat:@"iOS terminal working directory is unavailable: %@", session.workingDirectory];
    return nil;
  }
  if ([terminalType isEqualToString:ORTIosToyboxTerminalType]) {
    session.sessionId = [NSString stringWithFormat:@"ios-toybox-%lu", static_cast<unsigned long>(ORTIosNextToyboxSessionIdentifier++)];
    ORTIosAppendToyboxOutput(session, @"Toybox 0.8.12\r\n");
    ORTIosAppendToyboxOutput(session, ORTIosToyboxPrompt(session));
  } else if ([terminalType isEqualToString:ORTIosPythonTerminalType]) {
    session.sessionId = [NSString stringWithFormat:@"ios-python-%lu", static_cast<unsigned long>(ORTIosNextPythonSessionIdentifier++)];
    if (!ORTIosCreatePythonTerminal(session.sessionId, error)) {
      return nil;
    }
  } else {
    NSDictionary *nodeResult = ORTIosCallNodeService(@{
      @"command" : @"create",
      @"sessionName" : sessionName,
      @"workingDir" : workingDirectory,
      @"rows" : @(rows),
      @"cols" : @(columns),
    }, error);
    if (nodeResult == nil) {
      return nil;
    }
    session.sessionId = ORTIosRequiredString(nodeResult, @"sessionId", error);
    if (session.sessionId == nil) {
      return nil;
    }
  }
  ORTIosSessions[session.sessionId] = session;
  return @{ @"sessionId" : session.sessionId };
}

/// Drains terminal output from one native Toybox, Python, or Node interpreter session.
static NSString *ORTIosReadTerminal(ORTIosTerminalSession *session, NSString **error) {
  if ([session.terminalType isEqualToString:ORTIosToyboxTerminalType]) {
    NSString *output = session.toyboxOutput.copy;
    [session.toyboxOutput setString:@""];
    return output;
  }
  if ([session.terminalType isEqualToString:ORTIosPythonTerminalType]) {
    return ORTIosCallPythonStringFunction(@"operit_read_terminal", session.sessionId, nil, error);
  }
  NSDictionary *result = ORTIosCallNodeService(@{ @"command" : @"read", @"sessionId" : session.sessionId }, error);
  if (result == nil) {
    return nil;
  }
  return ORTIosRequiredString(result, @"output", error);
}

/// Writes UTF-8 terminal input into one native Toybox, Python, or Node interpreter session.
static NSDictionary *ORTIosWriteTerminal(ORTIosTerminalSession *session, NSString *input, NSString **error) {
  session.commandRunning = YES;
  if ([session.terminalType isEqualToString:ORTIosToyboxTerminalType]) {
    ORTIosAppendToyboxOutput(session, ORTIosToyboxDisplayInput(input));
    NSString *emitted = ORTIosWriteToyboxTerminal(session, input, error);
    session.commandRunning = NO;
    if (emitted == nil) {
      return nil;
    }
    ORTIosAppendToyboxOutput(session, emitted);
    ORTIosAppendRuntimeLines(session, emitted);
    return @{ @"acceptedChars" : @([input length]) };
  }
  NSString *emitted = nil;
  if ([session.terminalType isEqualToString:ORTIosPythonTerminalType]) {
    emitted = ORTIosCallPythonStringFunction(@"operit_write_terminal", session.sessionId, input, error);
  } else {
    NSDictionary *result = ORTIosCallNodeService(@{
      @"command" : @"write",
      @"sessionId" : session.sessionId,
      @"input" : input,
    }, error);
    if (result != nil) {
      emitted = ORTIosRequiredString(result, @"output", error);
    }
  }
  session.commandRunning = NO;
  if (emitted == nil) {
    return nil;
  }
  ORTIosAppendRuntimeLines(session, emitted);
  return @{ @"acceptedChars" : @([input length]) };
}

/// Returns a full terminal transcript from one native Toybox, Python, or Node interpreter session.
static NSDictionary *ORTIosTerminalScreen(ORTIosTerminalSession *session, NSString **error) {
  NSString *content = nil;
  if ([session.terminalType isEqualToString:ORTIosToyboxTerminalType]) {
    content = session.toyboxScreen.copy;
  } else if ([session.terminalType isEqualToString:ORTIosPythonTerminalType]) {
    content = ORTIosCallPythonStringFunction(@"operit_screen_terminal", session.sessionId, nil, error);
  } else {
    NSDictionary *result = ORTIosCallNodeService(@{ @"command" : @"screen", @"sessionId" : session.sessionId }, error);
    if (result == nil) {
      return nil;
    }
    content = ORTIosRequiredString(result, @"content", error);
  }
  if (content == nil) {
    return nil;
  }
  return @{
    @"sessionId" : session.sessionId,
    @"terminalType" : session.terminalType,
    @"rows" : @(session.rows),
    @"cols" : @(session.columns),
    @"content" : content,
    @"commandRunning" : @(session.commandRunning),
  };
}

/// Closes one native Toybox or interpreter terminal and removes its session store entry.
static BOOL ORTIosCloseTerminal(ORTIosTerminalSession *session, NSString **error) {
  if ([session.terminalType isEqualToString:ORTIosToyboxTerminalType]) {
    session.toyboxExitCode = 0;
  } else if ([session.terminalType isEqualToString:ORTIosPythonTerminalType]) {
    if (ORTIosCallPythonStringFunction(@"operit_close_terminal", session.sessionId, nil, error) == nil) {
      return NO;
    }
  } else {
    if (ORTIosCallNodeService(@{ @"command" : @"close", @"sessionId" : session.sessionId }, error) == nil) {
      return NO;
    }
  }
  session.active = NO;
  [ORTIosSessions removeObjectForKey:session.sessionId];
  NSArray<NSString *> *keys = [ORTIosNamedSessions allKeysForObject:session.sessionId];
  for (NSString *key in keys) {
    [ORTIosNamedSessions removeObjectForKey:key];
  }
  return YES;
}

/// Lists all active terminal sessions known to the iOS native bridge.
static NSDictionary *ORTIosListTerminals(void) {
  ORTIosEnsureSessionStore();
  NSMutableArray<NSDictionary *> *entries = [NSMutableArray array];
  for (ORTIosTerminalSession *session in ORTIosSessions.allValues) {
    if (session.active) {
      [entries addObject:@{
        @"sessionId" : session.sessionId,
        @"sessionName" : session.sessionName,
        @"terminalType" : session.terminalType,
        @"sessionKind" : @"embedded-interpreter",
        @"workingDir" : session.workingDirectory,
        @"commandRunning" : @(session.commandRunning),
      }];
    }
  }
  return @{ @"sessions" : entries };
}

/// Gets or creates a named Toybox, Python, or Node embedded terminal.
static NSDictionary *ORTIosCreateOrGetTerminal(NSDictionary *request, NSString **error) {
  NSString *sessionName = ORTIosRequiredString(request, @"sessionName", error);
  if (sessionName == nil) {
    return nil;
  }
  NSString *terminalType = ORTIosRequiredString(request, @"terminalType", error);
  if (terminalType == nil) {
    return nil;
  }
  NSString *key = [NSString stringWithFormat:@"%@:%@", terminalType, sessionName];
  ORTIosEnsureSessionStore();
  NSString *existingId = ORTIosNamedSessions[key];
  ORTIosTerminalSession *existing = existingId == nil ? nil : ORTIosSessions[existingId];
  if (existing != nil && existing.active) {
    return @{
      @"sessionId" : existing.sessionId,
      @"sessionName" : existing.sessionName,
      @"terminalType" : existing.terminalType,
      @"isNewSession" : @NO,
    };
  }
  NSString *workingDirectory = ORTIosWorkspaceDirectory(error);
  if (workingDirectory == nil) {
    return nil;
  }
  NSDictionary *startRequest = @{
    @"sessionName" : sessionName,
    @"terminalType" : terminalType,
    @"workingDir" : workingDirectory,
    @"rows" : @24,
    @"cols" : @80,
  };
  NSDictionary *started = ORTIosStartTerminal(startRequest, error);
  if (started == nil) {
    return nil;
  }
  NSString *sessionId = started[@"sessionId"];
  ORTIosNamedSessions[key] = sessionId;
  return @{
    @"sessionId" : sessionId,
    @"sessionName" : sessionName,
    @"terminalType" : terminalType,
    @"isNewSession" : @YES,
  };
}

/// Handles one terminal-specific command received from the Rust iOS terminal host.
static NSDictionary *ORTIosHandleTerminalCommand(NSString *command, NSDictionary *request, NSString **error) {
  if ([command isEqualToString:@"terminalStart"]) {
    return ORTIosStartTerminal(request, error);
  }
  if ([command isEqualToString:@"terminalCreateOrGet"]) {
    return ORTIosCreateOrGetTerminal(request, error);
  }
  if ([command isEqualToString:@"terminalList"]) {
    return ORTIosListTerminals();
  }
  NSString *sessionId = ORTIosRequiredString(request, @"sessionId", error);
  if (sessionId == nil) {
    return nil;
  }
  ORTIosTerminalSession *session = ORTIosRequiredSession(sessionId, error);
  if (session == nil) {
    return nil;
  }
  if ([command isEqualToString:@"terminalRead"]) {
    NSString *output = ORTIosReadTerminal(session, error);
    return output == nil ? nil : @{ @"output" : output };
  }
  if ([command isEqualToString:@"terminalWrite"]) {
    NSString *input = ORTIosRequiredString(request, @"input", error);
    return input == nil ? nil : ORTIosWriteTerminal(session, input, error);
  }
  if ([command isEqualToString:@"terminalResize"]) {
    NSUInteger rows = ORTIosRequiredPositiveInteger(request, @"rows", error);
    if (rows == 0) {
      return nil;
    }
    NSUInteger columns = ORTIosRequiredPositiveInteger(request, @"cols", error);
    if (columns == 0) {
      return nil;
    }
    session.rows = rows;
    session.columns = columns;
    if ([session.terminalType isEqualToString:ORTIosNodeTerminalType]) {
      if (ORTIosCallNodeService(@{
        @"command" : @"resize",
        @"sessionId" : session.sessionId,
        @"rows" : @(rows),
        @"cols" : @(columns),
      }, error) == nil) {
        return nil;
      }
    }
    return @{};
  }
  if ([command isEqualToString:@"terminalPoll"]) {
    return @{ @"exitCode" : [NSNull null] };
  }
  if ([command isEqualToString:@"terminalClose"]) {
    return ORTIosCloseTerminal(session, error) ? @{} : nil;
  }
  if ([command isEqualToString:@"terminalScreen"]) {
    return ORTIosTerminalScreen(session, error);
  }
  if ([command isEqualToString:@"terminalExecute"]) {
    NSString *source = ORTIosRequiredString(request, @"command", error);
    if (source == nil) {
      return nil;
    }
    if (ORTIosWriteTerminal(session, [source stringByAppendingString:@"\n"], error) == nil) {
      return nil;
    }
    NSString *output = ORTIosReadTerminal(session, error);
    if (output == nil) {
      return nil;
    }
    return @{
      @"sessionId" : session.sessionId,
      @"terminalType" : session.terminalType,
      @"output" : output,
      @"exitCode" : @([session.terminalType isEqualToString:ORTIosToyboxTerminalType] ? session.toyboxExitCode : 0),
      @"timedOut" : @NO,
    };
  }
  *error = [NSString stringWithFormat:@"unsupported iOS terminal bridge command: %@", command];
  return nil;
}

/// Creates one managed runtime process using the matching embedded terminal interpreter.
static NSDictionary *ORTIosStartManagedRuntime(NSDictionary *request, NSString **error) {
  NSString *program = ORTIosRequiredString(request, @"program", error);
  if (program == nil) {
    return nil;
  }
  NSString *terminalType = [program isEqualToString:@"python"] ? ORTIosPythonTerminalType : ORTIosNodeTerminalType;
  if (![program isEqualToString:@"python"] && ![program isEqualToString:@"node"]) {
    *error = [NSString stringWithFormat:@"iOS embedded runtime does not provide %@", program];
    return nil;
  }
  NSString *workingDirectory = ORTIosWorkspaceDirectory(error);
  if (workingDirectory == nil) {
    return nil;
  }
  NSDictionary *started = ORTIosStartTerminal(@{
    @"sessionName" : [NSString stringWithFormat:@"runtime:%@", program],
    @"terminalType" : terminalType,
    @"workingDir" : workingDirectory,
    @"rows" : @24,
    @"cols" : @80,
  }, error);
  if (started == nil) {
    return nil;
  }
  return @{ @"id" : started[@"sessionId"] };
}

/// Handles one managed runtime process command from the Rust iOS managed-runtime host.
static NSDictionary *ORTIosHandleManagedRuntimeCommand(NSString *command, NSDictionary *request, NSString **error) {
  if ([command isEqualToString:@"workspaceDir"]) {
    NSString *workspace = ORTIosWorkspaceDirectory(error);
    return workspace == nil ? nil : @{ @"path" : workspace };
  }
  if ([command isEqualToString:@"resolveExecutable"]) {
    NSString *program = ORTIosRequiredString(request, @"program", error);
    if (program == nil) {
      return nil;
    }
    if ([program isEqualToString:@"node"]) {
      return @{ @"executable" : @"embedded-node" };
    }
    if ([program isEqualToString:@"python"]) {
      return @{ @"executable" : @"embedded-python" };
    }
    *error = [NSString stringWithFormat:@"iOS embedded runtime does not provide %@", program];
    return nil;
  }
  if ([command isEqualToString:@"start"]) {
    return ORTIosStartManagedRuntime(request, error);
  }
  NSString *sessionId = ORTIosRequiredString(request, @"id", error);
  if (sessionId == nil) {
    return nil;
  }
  ORTIosTerminalSession *session = ORTIosRequiredSession(sessionId, error);
  if (session == nil) {
    return nil;
  }
  if ([command isEqualToString:@"writeLine"]) {
    NSString *line = ORTIosRequiredString(request, @"line", error);
    if (line == nil) {
      return nil;
    }
    return ORTIosWriteTerminal(session, [line stringByAppendingString:@"\n"], error) == nil ? nil : @{};
  }
  if ([command isEqualToString:@"writeLines"]) {
    id lines = request[@"lines"];
    if (![lines isKindOfClass:[NSArray class]]) {
      *error = @"iOS managed runtime writeLines requires a line array";
      return nil;
    }
    for (id line in lines) {
      if (![line isKindOfClass:[NSString class]]) {
        *error = @"iOS managed runtime writeLines contains non-text input";
        return nil;
      }
      if (ORTIosWriteTerminal(session, [line stringByAppendingString:@"\n"], error) == nil) {
        return nil;
      }
    }
    return @{};
  }
  if ([command isEqualToString:@"readStdoutLine"]) {
    if (session.runtimeOutputLines.count == 0) {
      NSString *output = ORTIosReadTerminal(session, error);
      if (output == nil) {
        return nil;
      }
      ORTIosAppendRuntimeLines(session, output);
    }
    if (session.runtimeOutputLines.count == 0) {
      return @{ @"line" : [NSNull null] };
    }
    NSString *line = session.runtimeOutputLines.firstObject;
    [session.runtimeOutputLines removeObjectAtIndex:0];
    return @{ @"line" : line };
  }
  if ([command isEqualToString:@"drainStderr"]) {
    return @{ @"stderr" : @"" };
  }
  if ([command isEqualToString:@"isRunning"]) {
    return @{ @"running" : @(session.active) };
  }
  if ([command isEqualToString:@"close"]) {
    return ORTIosCloseTerminal(session, error) ? @{} : nil;
  }
  *error = [NSString stringWithFormat:@"unsupported iOS managed runtime bridge command: %@", command];
  return nil;
}

/// Dispatches one C ABI runtime request to the terminal or managed-runtime implementation.
extern "C" char *operit_ios_native_runtime_call(const char *command, const char *request_json) {
  @autoreleasepool {
    NSString *commandText = command == nullptr ? nil : [NSString stringWithUTF8String:command];
    NSString *requestText = request_json == nullptr ? nil : [NSString stringWithUTF8String:request_json];
    if (commandText == nil || requestText == nil) {
      return ORTIosSerializeResponse(ORTIosFailure(@"iOS native runtime request is not UTF-8"));
    }
    NSData *requestData = [requestText dataUsingEncoding:NSUTF8StringEncoding];
    id parsed = [NSJSONSerialization JSONObjectWithData:requestData options:0 error:nil];
    NSDictionary *request = [parsed isKindOfClass:[NSDictionary class]] ? parsed : @{};
    if (![parsed isKindOfClass:[NSDictionary class]] && ![parsed isKindOfClass:[NSNull class]]) {
      return ORTIosSerializeResponse(ORTIosFailure(@"iOS native runtime request must be a JSON object or null"));
    }
    NSString *error = nil;
    NSDictionary *result = nil;
    @synchronized([ORTIosTerminalSession class]) {
      if ([commandText hasPrefix:@"terminal"]) {
        result = ORTIosHandleTerminalCommand(commandText, request, &error);
      } else {
        result = ORTIosHandleManagedRuntimeCommand(commandText, request, &error);
      }
    }
    return ORTIosSerializeResponse(result == nil ? ORTIosFailure(error ?: @"iOS native runtime command failed") : ORTIosSuccess(result));
  }
}

/// Releases UTF-8 response storage returned through the native iOS runtime C ABI.
extern "C" void operit_ios_native_runtime_free(char *value) {
  free(value);
}
