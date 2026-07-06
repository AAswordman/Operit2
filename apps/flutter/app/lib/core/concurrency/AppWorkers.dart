// ignore_for_file: file_names

import 'dart:async';
import 'dart:isolate';

typedef AppWorkerTask<R> = FutureOr<R> Function();

abstract final class AppWorkers {
  static Future<R> run<R>(AppWorkerTask<R> task, {String? debugName}) {
    return Isolate.run<R>(task, debugName: debugName ?? 'app-worker');
  }
}
