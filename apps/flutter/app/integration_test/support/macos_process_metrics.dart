import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

class MacOSProcessUsage {
  const MacOSProcessUsage(this.cpuSeconds, this.wakeups, this.footprint);
  final double cpuSeconds;
  final int wakeups;
  final int footprint;
}

/// Reads process counters at stage boundaries without introducing a sampling timer.
class MacOSProcessMetrics {
  MacOSProcessMetrics() {
    final info = calloc<Uint32>(2);
    try {
      final timebase = DynamicLibrary.open('/usr/lib/libSystem.B.dylib')
          .lookupFunction<
            Int32 Function(Pointer<Uint32>),
            int Function(Pointer<Uint32>)
          >('mach_timebase_info');
      if (timebase(info) != 0) throw StateError('mach_timebase_info failed');
      _secondsPerTick = info[0] / info[1] / 1e9;
    } finally {
      calloc.free(info);
    }
  }
  final _buffer = calloc<Uint64>(
    20,
  ); // Darwin rusage_info_v2, including its 16-byte UUID.
  final _read = DynamicLibrary.open('/usr/lib/libproc.dylib')
      .lookupFunction<
        Int32 Function(Int32, Int32, Pointer<Void>),
        int Function(int, int, Pointer<Void>)
      >('proc_pid_rusage');
  late final double _secondsPerTick;
  MacOSProcessUsage read() {
    if (_read(pid, 2, _buffer.cast<Void>()) != 0) {
      throw StateError('proc_pid_rusage failed');
    }
    return MacOSProcessUsage(
      (_buffer[2] + _buffer[3]) * _secondsPerTick,
      _buffer[5],
      _buffer[9],
    );
  }

  void close() => calloc.free(_buffer);
}
