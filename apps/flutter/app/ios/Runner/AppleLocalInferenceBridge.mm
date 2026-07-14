#import <Foundation/Foundation.h>
#include <sherpa-onnx/c-api/c-api.h>

#include <cmath>
#include <stdlib.h>
#include <string.h>

static NSString *const kEmbeddedSherpaEngine = @"embedded:sherpa-onnx@1.13.2";

// Raises one structured Apple local inference validation error.
static void raiseInferenceError(NSString *message) __attribute__((noreturn));
static void raiseInferenceError(NSString *message) {
  @throw [NSException exceptionWithName:@"OperitAppleLocalInference" reason:message userInfo:nil];
}

// Copies one NSString into an owned UTF-8 C string.
static char *copyCString(NSString *value) {
  const char *utf8 = value.UTF8String;
  const size_t length = strlen(utf8);
  char *copy = static_cast<char *>(malloc(length + 1));
  if (copy == nullptr) {
    return nullptr;
  }
  memcpy(copy, utf8, length + 1);
  return copy;
}

// Serializes one JSON object into an owned UTF-8 C string.
static char *copyJsonObject(NSDictionary<NSString *, id> *object) {
  NSError *error = nil;
  NSData *data = [NSJSONSerialization dataWithJSONObject:object options:0 error:&error];
  if (data == nil || error != nil) {
    return nullptr;
  }
  NSString *json = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
  if (json == nil) {
    return nullptr;
  }
  return copyCString(json);
}

// Builds one JSON envelope carrying a native bridge error.
static char *copyErrorEnvelope(NSString *message) {
  return copyJsonObject(@{@"error" : message});
}

// Builds one JSON envelope carrying an exact local inference result payload.
static char *copyResultEnvelope(NSDictionary<NSString *, id> *result) {
  NSError *error = nil;
  NSData *data = [NSJSONSerialization dataWithJSONObject:result options:0 error:&error];
  if (data == nil || error != nil) {
    return nullptr;
  }
  NSString *resultJson = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
  if (resultJson == nil) {
    return nullptr;
  }
  return copyJsonObject(@{@"resultJson" : resultJson});
}

// Parses one JSON string and requires a dictionary root.
static NSDictionary<NSString *, id> *parseJsonObject(NSString *json, NSString *label) {
  NSData *data = [json dataUsingEncoding:NSUTF8StringEncoding];
  NSError *error = nil;
  id value = data == nil ? nil : [NSJSONSerialization JSONObjectWithData:data options:0 error:&error];
  if (![value isKindOfClass:NSDictionary.class] || error != nil) {
    raiseInferenceError([NSString stringWithFormat:@"%@ must be a JSON object", label]);
  }
  return (NSDictionary<NSString *, id> *)value;
}

// Returns one required non-empty string field.
static NSString *requiredString(NSDictionary<NSString *, id> *object, NSString *key) {
  id value = object[key];
  if (![value isKindOfClass:NSString.class]) {
    raiseInferenceError([NSString stringWithFormat:@"Apple local inference field is not a string: %@", key]);
  }
  NSString *text = [(NSString *)value
      stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet];
  if (text.length == 0) {
    raiseInferenceError([NSString stringWithFormat:@"Apple local inference field is empty: %@", key]);
  }
  return text;
}

// Returns one required finite numeric field.
static double requiredNumber(NSDictionary<NSString *, id> *object, NSString *key) {
  id value = object[key];
  if (![value isKindOfClass:NSNumber.class]) {
    raiseInferenceError([NSString stringWithFormat:@"Apple local inference field is not numeric: %@", key]);
  }
  const double number = ((NSNumber *)value).doubleValue;
  if (!std::isfinite(number)) {
    raiseInferenceError([NSString stringWithFormat:@"Apple local inference field is not finite: %@", key]);
  }
  return number;
}

// Returns one required positive integer field.
static NSInteger requiredPositiveInteger(NSDictionary<NSString *, id> *object, NSString *key) {
  const double number = requiredNumber(object, key);
  const NSInteger integer = static_cast<NSInteger>(number);
  if (number != static_cast<double>(integer) || integer <= 0) {
    raiseInferenceError([NSString stringWithFormat:@"Apple local inference field must be a positive integer: %@", key]);
  }
  return integer;
}

// Returns one required array of non-empty strings.
static NSArray<NSString *> *requiredStringArray(NSDictionary<NSString *, id> *object, NSString *key) {
  id value = object[key];
  if (![value isKindOfClass:NSArray.class]) {
    raiseInferenceError([NSString stringWithFormat:@"Apple local inference field is not an array: %@", key]);
  }
  NSMutableArray<NSString *> *strings = [NSMutableArray array];
  for (id item in (NSArray *)value) {
    if (![item isKindOfClass:NSString.class] || ((NSString *)item).length == 0) {
      raiseInferenceError([NSString stringWithFormat:@"Apple local inference array contains an invalid string: %@", key]);
    }
    [strings addObject:(NSString *)item];
  }
  return strings;
}

// Validates the exact application-hosted Sherpa engine identifier.
static void validateEmbeddedEngine(NSDictionary<NSString *, id> *request) {
  NSString *engine = requiredString(request, @"engineLibraryDirectory");
  if (![engine isEqualToString:kEmbeddedSherpaEngine]) {
    raiseInferenceError([NSString stringWithFormat:@"Apple local inference engine is invalid: %@", engine]);
  }
}

// Validates one structured options JSON object.
static void validateOptions(NSDictionary<NSString *, id> *request) {
  parseJsonObject(requiredString(request, @"optionsJson"), @"Apple local inference options");
}

// Returns one canonical existing model directory.
static NSString *requiredDirectory(NSString *path) {
  NSString *canonical = path.stringByStandardizingPath.stringByResolvingSymlinksInPath;
  BOOL isDirectory = NO;
  if (![NSFileManager.defaultManager fileExistsAtPath:canonical isDirectory:&isDirectory] || !isDirectory) {
    raiseInferenceError([NSString stringWithFormat:@"Apple local inference directory is missing: %@", canonical]);
  }
  return canonical;
}

// Returns one canonical existing file.
static NSString *requiredFile(NSString *path) {
  NSString *canonical = path.stringByStandardizingPath.stringByResolvingSymlinksInPath;
  BOOL isDirectory = NO;
  if (![NSFileManager.defaultManager fileExistsAtPath:canonical isDirectory:&isDirectory] || isDirectory) {
    raiseInferenceError([NSString stringWithFormat:@"Apple local inference file is missing: %@", canonical]);
  }
  return canonical;
}

// Resolves one declared model file below the installed model directory.
static NSString *declaredModelFile(NSString *modelDirectory, NSString *relativePath) {
  if (relativePath.length == 0 || relativePath.isAbsolutePath) {
    raiseInferenceError([NSString stringWithFormat:@"Apple local model file path is invalid: %@", relativePath]);
  }
  NSString *file = requiredFile([modelDirectory stringByAppendingPathComponent:relativePath]);
  NSString *directoryPrefix = [modelDirectory stringByAppendingString:@"/"];
  if (![file hasPrefix:directoryPrefix]) {
    raiseInferenceError([NSString stringWithFormat:@"Apple local model file escapes its directory: %@", relativePath]);
  }
  return file;
}

// Resolves one declared model directory below the installed model directory.
static NSString *declaredModelDirectory(NSString *modelDirectory, NSString *relativePath) {
  if (relativePath.length == 0 || relativePath.isAbsolutePath) {
    raiseInferenceError([NSString stringWithFormat:@"Apple local model directory path is invalid: %@", relativePath]);
  }
  NSString *directory = requiredDirectory([modelDirectory stringByAppendingPathComponent:relativePath]);
  NSString *directoryPrefix = [modelDirectory stringByAppendingString:@"/"];
  if (![directory hasPrefix:directoryPrefix]) {
    raiseInferenceError([NSString stringWithFormat:@"Apple local model directory escapes its root: %@", relativePath]);
  }
  return directory;
}

// Resolves and joins one declared model file list for Sherpa rules.
static NSString *joinedModelFiles(
    NSString *modelDirectory,
    NSDictionary<NSString *, id> *driver,
    NSString *key) {
  NSMutableArray<NSString *> *paths = [NSMutableArray array];
  for (NSString *relativePath in requiredStringArray(driver, key)) {
    [paths addObject:declaredModelFile(modelDirectory, relativePath)];
  }
  return [paths componentsJoinedByString:@","];
}

// Parses one externally tagged local model driver object.
static NSDictionary<NSString *, id> *requiredDriver(
    NSDictionary<NSString *, id> *request,
    NSString **tag) {
  NSDictionary<NSString *, id> *root =
      parseJsonObject(requiredString(request, @"driverJson"), @"Apple local inference driver");
  if (root.count != 1) {
    raiseInferenceError(@"Apple local inference driver must have one exact tag");
  }
  NSString *driverTag = root.allKeys.firstObject;
  id driver = root[driverTag];
  if (![driver isKindOfClass:NSDictionary.class]) {
    raiseInferenceError(@"Apple local inference driver payload must be a JSON object");
  }
  *tag = driverTag;
  return (NSDictionary<NSString *, id> *)driver;
}

// Returns a bounded Apple inference thread count.
static int32_t inferenceThreadCount(void) {
  const NSUInteger count = NSProcessInfo.processInfo.activeProcessorCount;
  return static_cast<int32_t>(MIN(MAX(count, 1u), 4u));
}

// Parses one exact numeric speaker identifier.
static int32_t requiredSpeaker(NSDictionary<NSString *, id> *request) {
  NSString *voice = requiredString(request, @"voice");
  NSScanner *scanner = [NSScanner scannerWithString:voice];
  NSInteger value = 0;
  if (![scanner scanInteger:&value] || !scanner.isAtEnd || value < INT32_MIN || value > INT32_MAX) {
    raiseInferenceError(@"Apple local TTS voice must be a numeric speaker id");
  }
  return static_cast<int32_t>(value);
}

// Validates one speaker identifier against manifest metadata.
static void validateSpeaker(int32_t speaker, int32_t speakerCount) {
  if (speaker < 0 || speaker >= speakerCount) {
    raiseInferenceError([NSString stringWithFormat:@"Apple local TTS speaker must be between 0 and %d", speakerCount - 1]);
  }
}

// Returns one validated output path whose parent directory already exists.
static NSString *requiredOutputPath(NSDictionary<NSString *, id> *request) {
  NSString *output = requiredString(request, @"outputPath").stringByStandardizingPath;
  if ([NSFileManager.defaultManager fileExistsAtPath:output]) {
    raiseInferenceError([NSString stringWithFormat:@"Apple local TTS output already exists: %@", output]);
  }
  requiredDirectory(output.stringByDeletingLastPathComponent);
  return output;
}

// Runs one streaming transducer request through the linked Sherpa C API.
static NSDictionary<NSString *, id> *transcribeLocalSpeech(NSDictionary<NSString *, id> *request) {
  validateEmbeddedEngine(request);
  validateOptions(request);
  NSString *modelDirectory = requiredDirectory(requiredString(request, @"modelDirectory"));
  NSString *audioPath = requiredFile(requiredString(request, @"audioPath"));
  NSString *tag = nil;
  NSDictionary<NSString *, id> *driver = requiredDriver(request, &tag);
  if (![tag isEqualToString:@"SherpaOnnxStreamingTransducer"]) {
    raiseInferenceError([NSString stringWithFormat:@"Apple local STT driver is unsupported: %@", tag]);
  }

  NSString *encoder = declaredModelFile(modelDirectory, requiredString(driver, @"encoder"));
  NSString *decoder = declaredModelFile(modelDirectory, requiredString(driver, @"decoder"));
  NSString *joiner = declaredModelFile(modelDirectory, requiredString(driver, @"joiner"));
  NSString *tokens = declaredModelFile(modelDirectory, requiredString(driver, @"tokens"));
  NSString *modelType = requiredString(driver, @"modelType");

  SherpaOnnxOnlineRecognizerConfig config = {};
  config.feat_config.sample_rate = 16000;
  config.feat_config.feature_dim = 80;
  config.model_config.transducer.encoder = encoder.UTF8String;
  config.model_config.transducer.decoder = decoder.UTF8String;
  config.model_config.transducer.joiner = joiner.UTF8String;
  config.model_config.tokens = tokens.UTF8String;
  config.model_config.num_threads = inferenceThreadCount();
  config.model_config.provider = "cpu";
  config.model_config.model_type = modelType.UTF8String;
  config.decoding_method = "greedy_search";
  config.max_active_paths = 4;

  const SherpaOnnxWave *wave = SherpaOnnxReadWave(audioPath.UTF8String);
  if (wave == nullptr || wave->samples == nullptr || wave->sample_rate <= 0 || wave->num_samples <= 0) {
    if (wave != nullptr) {
      SherpaOnnxFreeWave(wave);
    }
    raiseInferenceError(@"Apple local STT WAV input is invalid");
  }
  const SherpaOnnxOnlineRecognizer *recognizer = SherpaOnnxCreateOnlineRecognizer(&config);
  if (recognizer == nullptr) {
    SherpaOnnxFreeWave(wave);
    raiseInferenceError(@"Apple Sherpa ONNX failed to create the STT recognizer");
  }
  const SherpaOnnxOnlineStream *stream = SherpaOnnxCreateOnlineStream(recognizer);
  if (stream == nullptr) {
    SherpaOnnxDestroyOnlineRecognizer(recognizer);
    SherpaOnnxFreeWave(wave);
    raiseInferenceError(@"Apple Sherpa ONNX failed to create the STT stream");
  }
  SherpaOnnxOnlineStreamAcceptWaveform(stream, wave->sample_rate, wave->samples, wave->num_samples);
  SherpaOnnxOnlineStreamInputFinished(stream);
  while (SherpaOnnxIsOnlineStreamReady(recognizer, stream) != 0) {
    SherpaOnnxDecodeOnlineStream(recognizer, stream);
  }
  const SherpaOnnxOnlineRecognizerResult *result =
      SherpaOnnxGetOnlineStreamResult(recognizer, stream);
  if (result == nullptr || result->text == nullptr || result->json == nullptr) {
    if (result != nullptr) {
      SherpaOnnxDestroyOnlineRecognizerResult(result);
    }
    SherpaOnnxDestroyOnlineStream(stream);
    SherpaOnnxDestroyOnlineRecognizer(recognizer);
    SherpaOnnxFreeWave(wave);
    raiseInferenceError(@"Apple Sherpa ONNX returned no STT result");
  }
  NSString *text = [NSString stringWithUTF8String:result->text];
  NSString *resultJson = [NSString stringWithUTF8String:result->json];
  SherpaOnnxDestroyOnlineRecognizerResult(result);
  SherpaOnnxDestroyOnlineStream(stream);
  SherpaOnnxDestroyOnlineRecognizer(recognizer);
  SherpaOnnxFreeWave(wave);
  if (text == nil || resultJson == nil) {
    raiseInferenceError(@"Apple Sherpa ONNX returned invalid UTF-8 STT output");
  }
  return @{@"text" : text, @"resultJson" : resultJson};
}

// Runs one configured TTS driver through the linked Sherpa C API.
static NSDictionary<NSString *, id> *synthesizeLocalSpeech(NSDictionary<NSString *, id> *request) {
  validateEmbeddedEngine(request);
  validateOptions(request);
  NSString *text = requiredString(request, @"text");
  const double speedValue = requiredNumber(request, @"speed");
  if (speedValue <= 0.0) {
    raiseInferenceError(@"Apple local TTS speed must be positive");
  }
  const int32_t speaker = requiredSpeaker(request);
  NSString *modelDirectory = requiredDirectory(requiredString(request, @"modelDirectory"));
  NSString *outputPath = requiredOutputPath(request);
  NSString *tag = nil;
  NSDictionary<NSString *, id> *driver = requiredDriver(request, &tag);

  NSString *model = @"";
  NSString *lexicon = @"";
  NSString *tokens = @"";
  NSString *vocoder = @"";
  NSString *voices = @"";
  NSString *dataDirectory = @"";
  NSString *ruleFsts = @"";
  NSString *ruleFars = @"";
  const int32_t speakerCount = static_cast<int32_t>(requiredPositiveInteger(driver, @"speakerCount"));
  validateSpeaker(speaker, speakerCount);

  SherpaOnnxOfflineTtsConfig config = {};
  config.model.num_threads = inferenceThreadCount();
  config.model.provider = "cpu";
  config.max_num_sentences = 1;
  config.silence_scale = 0.2f;

  if ([tag isEqualToString:@"SherpaOnnxVits"]) {
    model = declaredModelFile(modelDirectory, requiredString(driver, @"model"));
    lexicon = declaredModelFile(modelDirectory, requiredString(driver, @"lexicon"));
    tokens = declaredModelFile(modelDirectory, requiredString(driver, @"tokens"));
    ruleFsts = joinedModelFiles(modelDirectory, driver, @"ruleFsts");
    ruleFars = joinedModelFiles(modelDirectory, driver, @"ruleFars");
    config.model.vits.model = model.UTF8String;
    config.model.vits.lexicon = lexicon.UTF8String;
    config.model.vits.tokens = tokens.UTF8String;
    config.model.vits.noise_scale = 0.667f;
    config.model.vits.noise_scale_w = 0.8f;
    config.model.vits.length_scale = 1.0f;
  } else if ([tag isEqualToString:@"SherpaOnnxMatcha"]) {
    model = declaredModelFile(modelDirectory, requiredString(driver, @"acousticModel"));
    vocoder = declaredModelFile(modelDirectory, requiredString(driver, @"vocoder"));
    lexicon = declaredModelFile(modelDirectory, requiredString(driver, @"lexicon"));
    tokens = declaredModelFile(modelDirectory, requiredString(driver, @"tokens"));
    ruleFsts = joinedModelFiles(modelDirectory, driver, @"ruleFsts");
    ruleFars = joinedModelFiles(modelDirectory, driver, @"ruleFars");
    config.model.matcha.acoustic_model = model.UTF8String;
    config.model.matcha.vocoder = vocoder.UTF8String;
    config.model.matcha.lexicon = lexicon.UTF8String;
    config.model.matcha.tokens = tokens.UTF8String;
    config.model.matcha.noise_scale = 1.0f;
    config.model.matcha.length_scale = 1.0f;
  } else if ([tag isEqualToString:@"SherpaOnnxKitten"]) {
    model = declaredModelFile(modelDirectory, requiredString(driver, @"model"));
    voices = declaredModelFile(modelDirectory, requiredString(driver, @"voices"));
    tokens = declaredModelFile(modelDirectory, requiredString(driver, @"tokens"));
    dataDirectory = declaredModelDirectory(modelDirectory, requiredString(driver, @"dataDir"));
    config.model.kitten.model = model.UTF8String;
    config.model.kitten.voices = voices.UTF8String;
    config.model.kitten.tokens = tokens.UTF8String;
    config.model.kitten.data_dir = dataDirectory.UTF8String;
    config.model.kitten.length_scale = 1.0f;
  } else {
    raiseInferenceError([NSString stringWithFormat:@"Apple local TTS driver is unsupported: %@", tag]);
  }
  config.rule_fsts = ruleFsts.UTF8String;
  config.rule_fars = ruleFars.UTF8String;

  const SherpaOnnxOfflineTts *tts = SherpaOnnxCreateOfflineTts(&config);
  if (tts == nullptr) {
    raiseInferenceError(@"Apple Sherpa ONNX failed to create the TTS engine");
  }
  const int32_t actualSpeakerCount = SherpaOnnxOfflineTtsNumSpeakers(tts);
  if (actualSpeakerCount != speakerCount) {
    SherpaOnnxDestroyOfflineTts(tts);
    raiseInferenceError([NSString stringWithFormat:
        @"Apple local TTS speaker count mismatch: manifest=%d, engine=%d",
        speakerCount,
        actualSpeakerCount]);
  }
  SherpaOnnxGenerationConfig generation = {};
  generation.silence_scale = 0.2f;
  generation.speed = static_cast<float>(speedValue);
  generation.sid = speaker;
  const SherpaOnnxGeneratedAudio *audio =
      SherpaOnnxOfflineTtsGenerateWithConfig(tts, text.UTF8String, &generation, nullptr, nullptr);
  if (audio == nullptr || audio->samples == nullptr || audio->n <= 0 || audio->sample_rate <= 0) {
    if (audio != nullptr) {
      SherpaOnnxDestroyOfflineTtsGeneratedAudio(audio);
    }
    SherpaOnnxDestroyOfflineTts(tts);
    raiseInferenceError(@"Apple Sherpa ONNX generated invalid audio");
  }
  const int32_t written =
      SherpaOnnxWriteWave(audio->samples, audio->n, audio->sample_rate, outputPath.UTF8String);
  SherpaOnnxDestroyOfflineTtsGeneratedAudio(audio);
  SherpaOnnxDestroyOfflineTts(tts);
  if (written != 1) {
    raiseInferenceError(@"Apple Sherpa ONNX failed to write the output WAV");
  }
  NSDictionary<NSFileAttributeKey, id> *attributes =
      [NSFileManager.defaultManager attributesOfItemAtPath:outputPath error:nil];
  if ([attributes[NSFileSize] unsignedLongLongValue] <= 44) {
    raiseInferenceError(@"Apple local TTS output WAV is invalid");
  }
  return @{@"audioPath" : outputPath, @"outputFormat" : @"wav"};
}

// Runs one Apple local inference command through the linked Sherpa C API.
extern "C" char *operit_apple_local_inference_run(const char *method, const char *requestJson) {
  @autoreleasepool {
    @try {
      if (method == nullptr || requestJson == nullptr) {
        raiseInferenceError(@"Apple local inference received null input");
      }
      NSString *methodValue = [NSString stringWithUTF8String:method];
      NSString *requestValue = [NSString stringWithUTF8String:requestJson];
      if (methodValue == nil || requestValue == nil) {
        raiseInferenceError(@"Apple local inference input is not valid UTF-8");
      }
      NSDictionary<NSString *, id> *request =
          parseJsonObject(requestValue, @"Apple local inference request");
      if ([methodValue isEqualToString:@"transcribeLocalSpeech"]) {
        return copyResultEnvelope(transcribeLocalSpeech(request));
      }
      if ([methodValue isEqualToString:@"synthesizeLocalSpeech"]) {
        return copyResultEnvelope(synthesizeLocalSpeech(request));
      }
      raiseInferenceError([NSString stringWithFormat:@"Apple local inference method is unknown: %@", methodValue]);
    } @catch (NSException *exception) {
      return copyErrorEnvelope(exception.reason ?: @"Apple local inference failed");
    }
  }
}

// Releases a JSON buffer returned by the Apple local inference bridge.
extern "C" void operit_apple_local_inference_free(char *value) {
  free(value);
}
