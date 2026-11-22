// Adapts owned native callbacks to smoke-test assertions using producer events only.
#include <chrono>
#include <algorithm>
#include <cstdlib>
#include <condition_variable>
#include <mutex>
#include <stdexcept>
#include <vector>

struct ORTTestCommand {
  std::mutex mutex;
  std::condition_variable changed;
  NSMutableData *frames = [NSMutableData data];
  NSDictionary *completion = nil;
};

/// Copies and frees a native-owned response before leaving its callback.
static NSDictionary *ORTTestDecode(char *response) {
  if (!response) throw std::runtime_error("Missing ARM64 command response");
  NSData *data = [NSData dataWithBytes:response length:strlen(response)];
  ORT_TEST_FREE(response);
  return [NSJSONSerialization JSONObjectWithData:data options:0 error:nullptr];
}

/// Publishes a frame without invoking a borrowed test callback on the session queue.
static void ORTTestOutput(void *context, const uint8_t *bytes, size_t count) {
  auto &state = *static_cast<ORTTestCommand *>(context);
  std::lock_guard<std::mutex> lock(state.mutex);
  [state.frames appendBytes:bytes length:count];
  state.changed.notify_one();
}

/// Publishes the sole completion after native delivery of every output callback.
static void ORTTestComplete(void *context, char *response) {
  auto &state = *static_cast<ORTTestCommand *>(context);
  std::lock_guard<std::mutex> lock(state.mutex);
  state.completion = ORTTestDecode(response);
  state.changed.notify_one();
}

/// Observes the new asynchronous bridge with an event wait and absolute cancellation deadlines.
static NSDictionary *ORTTestExecute(NSDictionary *request, void *context = nullptr,
    bool (*stream)(void *, const uint8_t *, size_t) = nullptr, double cancelAt = 0) {
  ORTTestCommand state;
  NSData *json = [NSJSONSerialization dataWithJSONObject:request options:0 error:nullptr];
  NSString *encoded = [[NSString alloc] initWithData:json encoding:NSUTF8StringEncoding];
  NSDictionary *accepted = ORTTestDecode(ORT_TEST_START("terminalExecuteAsync", encoded.UTF8String,
      &state, ORTTestOutput, ORTTestComplete));
  if (accepted[@"error"]) return accepted;
  NSMutableData *output = [NSMutableData data];
  const auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(
      request[@"timeoutMs"] ? [request[@"timeoutMs"] unsignedLongLongValue] : 30000);
  bool canceled = false;
  bool timedOut = false;
  auto stopDeadline = deadline;
  for (;;) {
    NSData *frame;
    NSDictionary *completion;
    {
      std::unique_lock<std::mutex> lock(state.mutex);
      auto wakeAt = canceled || timedOut ? stopDeadline : deadline;
      if (cancelAt && !canceled) {
        const double remaining = std::max(0.0, cancelAt - NSProcessInfo.processInfo.systemUptime);
        wakeAt = std::min(wakeAt, std::chrono::steady_clock::now() +
            std::chrono::duration_cast<std::chrono::steady_clock::duration>(std::chrono::duration<double>(remaining)));
      }
      state.changed.wait_until(lock, wakeAt, [&]{ return state.frames.length || state.completion; });
      frame = [state.frames copy];
      [state.frames setLength:0];
      completion = state.completion;
    }
    [output appendData:frame];
    bool cancel = frame.length && stream && !stream(context, static_cast<const uint8_t *>(frame.bytes), frame.length);
    cancel = cancel || (cancelAt && NSProcessInfo.processInfo.systemUptime >= cancelAt);
    if (!completion && !canceled && !timedOut && (cancel || std::chrono::steady_clock::now() >= deadline)) {
      canceled = cancel;
      timedOut = !cancel;
      stopDeadline = std::chrono::steady_clock::now() + std::chrono::seconds(3);
      NSData *cancelJSON = [NSJSONSerialization dataWithJSONObject:@{ @"sessionId": request[@"sessionId"] } options:0 error:nullptr];
      NSString *text = [[NSString alloc] initWithData:cancelJSON encoding:NSUTF8StringEncoding];
      ORTTestDecode(ORT_TEST_CALL("terminalCancel", text.UTF8String));
    }
    if (completion) {
      if (canceled) return @{ @"error": @"Test command canceled after native readiness acknowledgement" };
      if (completion[@"error"]) return completion;
      NSMutableDictionary *receipt = [completion[@"result"] mutableCopy];
      NSMutableArray *bytes = [NSMutableArray arrayWithCapacity:output.length];
      const uint8_t *raw = static_cast<const uint8_t *>(output.bytes);
      for (NSUInteger index = 0; index < output.length; ++index) [bytes addObject:@(raw[index])];
      receipt[@"outputBytes"] = bytes;
      receipt[@"timedOut"] = @(timedOut);
      return @{ @"result": receipt };
    }
    // A broken native callback lifetime must fail the smoke process before releasing its context.
    if ((canceled || timedOut) && std::chrono::steady_clock::now() >= stopDeadline) std::abort();
  }
}
