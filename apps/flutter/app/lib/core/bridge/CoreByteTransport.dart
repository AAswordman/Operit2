// ignore_for_file: file_names
import 'dart:typed_data';
import '../link/CoreLinkCodec.dart';

/// Defines the operation numbers carried by the ABI v3 Core transport.
enum CoreTransportOperation {
  call,
  pushOpen,
  pushItem,
  pushClose,
  watchSnapshot,
  watchStream,
  closeWatchStream,
}

abstract interface class CoreByteTransport {
  /// Sends an encoded operation and returns its protocol response.
  Future<Uint8List> request(
    CoreTransportOperation operation,
    CoreLinkMessage payload,
  );

  /// Opens one independently cancellable watch and returns its encoded frames.
  Stream<Uint8List> watch(String subscriptionId, CoreLinkMessage payload);
}
