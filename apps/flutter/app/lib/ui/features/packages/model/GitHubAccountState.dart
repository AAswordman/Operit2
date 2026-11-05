// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';

import '../../../../core/logging/ClientLogger.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

const String _logTag = 'GitHubAccount';

/// The signed-in GitHub account that publishing and the creator center need.
///
/// One instance serves each Core connection, so the market's creator entry and
/// the creator center show the same account right after either of them signs
/// in or out.
class GitHubAccountState extends ChangeNotifier {
  GitHubAccountState._(this._clients);

  static final Expando<GitHubAccountState> _states =
      Expando<GitHubAccountState>();

  static GitHubAccountState of(GeneratedCoreProxyClients clients) =>
      _states[clients.bridge] ??= GitHubAccountState._(clients);

  final GeneratedCoreProxyClients _clients;

  bool _loaded = false;
  bool _loggedIn = false;
  core_proxy.CoreDataPreferencesGitHubAuthPreferencesGitHubUser? _user;
  Object? _error;
  Future<void>? _reading;

  /// Whether the saved login has been read at least once.
  bool get loaded => _loaded;

  bool get loggedIn => _loggedIn;

  core_proxy.CoreDataPreferencesGitHubAuthPreferencesGitHubUser? get user =>
      _user;

  /// Why the saved login could not be read, if it could not.
  Object? get error => _error;

  GeneratedPreferencesGitHubAuthPreferencesCoreProxy get _auth =>
      _clients.preferencesGitHubAuthPreferences;

  /// Reads the saved login the first time it is needed.
  Future<void> ensureLoaded() => _loaded ? Future<void>.value() : refresh();

  /// Reads the saved login again; concurrent calls share one read.
  Future<void> refresh() =>
      _reading ??= _read().whenComplete(() => _reading = null);

  Future<void> _read() async {
    _error = null;
    try {
      final loggedIn = await _auth.isLoggedIn();
      final user = loggedIn ? await _auth.getCurrentUserInfo() : null;
      _user = user;
      _loggedIn = user != null;
    } catch (error, stackTrace) {
      ClientLogger.e(
        'read GitHub login failed',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
      _user = null;
      _loggedIn = false;
      _error = error;
    } finally {
      _loaded = true;
      notifyListeners();
    }
  }

  /// Signs out of GitHub and reads the resulting state.
  Future<void> logout() async {
    await _auth.logout();
    await refresh();
  }
}
