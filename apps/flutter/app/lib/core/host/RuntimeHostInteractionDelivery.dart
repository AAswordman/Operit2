// ignore_for_file: file_names

import 'dart:async';

import '../link/CoreLinkProtocol.dart';
import '../proxy/generated/CoreProxyModels.g.dart';

/// Retains an operation's result until Core accepts it across owner reconnects.
class RuntimeHostInteractionDelivery {
  /// Connects native execution and response delivery without binding their lifetimes.
  RuntimeHostInteractionDelivery({
    required this.dispatch,
    required this.respond,
    required this.errorResponse,
    required this.onDispatchError,
  });

  final Future<RuntimeHostInteractionResponse> Function(
    RuntimeHostInteractionRequest request,
  )
  dispatch;
  final Future<void> Function(
    String requestId,
    RuntimeHostInteractionResponse response,
  )
  respond;
  final RuntimeHostInteractionResponse Function(Object error) errorResponse;
  final void Function(Object error, StackTrace stackTrace) onDispatchError;
  final _pending = <String, _Delivery>{};

  /// Includes completed operations awaiting delivery, even if Core has expired them.
  List<RuntimeHostInteractionRequest> get pending =>
      _pending.values.map((delivery) => delivery.request).toList();

  /// Coalesces events and returns response transport time, or null after ownership ends.
  Future<Duration?> deliver(RuntimeHostInteractionRequest request) {
    final delivery = _pending.putIfAbsent(
      request.requestId,
      () => _Delivery(request, _dispatch(request)),
    );
    return delivery.sending ??= _respond(delivery);
  }

  /// Ends ownership so an operation still running cannot answer for a later session.
  void clear() => _pending.clear();

  /// Distinguishes broker rejections from transport and routing failures.
  static bool isCoreRejection(Object error) =>
      error is CoreLinkError && error.code == 'COMMAND_ERROR';

  /// Converts native failures into the same structured response as successful operations.
  Future<RuntimeHostInteractionResponse> _dispatch(
    RuntimeHostInteractionRequest request,
  ) async {
    try {
      return await dispatch(request);
    } catch (error, stackTrace) {
      onDispatchError(error, stackTrace);
      return errorResponse(error);
    }
  }

  /// Retains transport failures for the next connection and retires Core rejections.
  Future<Duration?> _respond(_Delivery delivery) async {
    final id = delivery.request.requestId;
    try {
      final response = await delivery.response;
      if (!identical(_pending[id], delivery)) return null;
      final stopwatch = Stopwatch()..start();
      await respond(id, response);
      if (identical(_pending[id], delivery)) _pending.remove(id);
      return stopwatch.elapsed;
    } catch (error) {
      if (isCoreRejection(error) && identical(_pending[id], delivery)) {
        _pending.remove(id);
      }
      rethrow;
    } finally {
      delivery.sending = null;
    }
  }
}

/// Owns one dispatch future independently of its response transport connection.
class _Delivery {
  /// Retains the native response independently of any response transport attempt.
  _Delivery(this.request, this.response);

  final RuntimeHostInteractionRequest request;
  final Future<RuntimeHostInteractionResponse> response;
  Future<Duration?>? sending;
}
