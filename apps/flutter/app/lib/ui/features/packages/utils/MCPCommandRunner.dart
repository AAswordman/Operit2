// ignore_for_file: file_names

import '../../../../core/link/CoreLinkProtocol.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';

Future<String> runMcpCoreCommand({
  required GeneratedCoreProxyClients clients,
  required List<String> args,
}) async {
  final value = await clients.bridge.call(
    CoreCallRequest(
      requestId: 'flutter-mcp-${DateTime.now().microsecondsSinceEpoch}',
      targetPath: CoreObjectPath.parse('application'),
      methodName: 'runCoreCommand',
      args: <String, Object?>{
        'args': <String>['mcp', ...args],
      },
    ),
  );
  if (value is! Map<Object?, Object?>) {
    throw StateError('Invalid core command output');
  }
  final stderr = value['stderr']?.toString().trim() ?? '';
  if (stderr.isNotEmpty) {
    throw StateError(stderr);
  }
  return value['stdout']?.toString().trim() ?? '';
}

Future<String> startMcpServer({
  required GeneratedCoreProxyClients clients,
  required String serverId,
}) {
  return runMcpCoreCommand(clients: clients, args: <String>['start', serverId]);
}

Future<String> killMcpServer({
  required GeneratedCoreProxyClients clients,
  required String serverId,
}) {
  return runMcpCoreCommand(clients: clients, args: <String>['kill', serverId]);
}

Future<String> applyMcpServerLifecycle({
  required GeneratedCoreProxyClients clients,
  required String serverId,
  required bool enabled,
}) async {
  if (enabled) {
    return startMcpServer(clients: clients, serverId: serverId);
  }
  return killMcpServer(clients: clients, serverId: serverId);
}

Future<String> restartMcpServer({
  required GeneratedCoreProxyClients clients,
  required String serverId,
}) async {
  await killMcpServer(clients: clients, serverId: serverId);
  return startMcpServer(clients: clients, serverId: serverId);
}
