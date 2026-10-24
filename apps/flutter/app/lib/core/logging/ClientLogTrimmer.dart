// ignore_for_file: file_names

import 'dart:convert';

/// Size cap for the persistent client log file.
const int clientLogMaxBytes = 2 * 1024 * 1024;

/// Newest bytes kept after one client log eviction pass.
const int clientLogKeepBytes = 1024 * 1024;

/// Drops the oldest whole records when the log text exceeds its byte cap and
/// returns the text unchanged when it fits. The kept tail starts at a record
/// boundary, and a marker line records the eviction.
String trimClientLogText(String content) {
  final bytes = utf8.encode(content);
  if (bytes.length <= clientLogMaxBytes) {
    return content;
  }
  final kept = keepNewestLogBytes(bytes, clientLogKeepBytes);
  const marker = '... client log trimmed; oldest records evicted\n';
  return '$marker${utf8.decode(kept, allowMalformed: true)}';
}

/// Returns the newest whole records of one UTF-8 log, keeping at most
/// [keepBytes] bytes and starting at a record boundary; empty when nothing
/// can be kept.
List<int> keepNewestLogBytes(List<int> bytes, int keepBytes) {
  if (bytes.length <= keepBytes) {
    return bytes;
  }
  var cut = bytes.length - keepBytes;
  while (cut > 0 && cut < bytes.length && (bytes[cut] & 0xC0) == 0x80) {
    cut += 1;
  }
  while (cut < bytes.length && bytes[cut] != 0x0A) {
    cut += 1;
  }
  return cut < bytes.length ? bytes.sublist(cut + 1) : <int>[];
}
