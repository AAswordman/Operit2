// ignore_for_file: file_names

import '../proxy/generated/CoreProxyModels.g.dart' show PeerTransport;

/// Selects the carrier advertised by discovery; bare socket addresses are TCP.
PeerTransport peerEndpointTransport(String address) {
  final endpoint = address.trim();
  if (!endpoint.contains('://')) {
    if (RegExp(r'^(\[[^\]]+\]|[^:/\s]+):[0-9]+$').hasMatch(endpoint)) {
      return PeerTransport.tcp;
    }
    throw FormatException('Invalid peer endpoint', address);
  }
  return switch (Uri.parse(endpoint).scheme.toLowerCase()) {
    'http' || 'https' => PeerTransport.http,
    'ws' || 'wss' => PeerTransport.webSocket,
    'tcp' => PeerTransport.tcp,
    'serial' => PeerTransport.serial,
    'bluetooth' => PeerTransport.bluetooth,
    _ => throw FormatException('Unsupported peer transport', address),
  };
}
