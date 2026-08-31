// ignore_for_file: file_names

enum ClientLogLevel {
  verbose(2, 'V'),
  debug(3, 'D'),
  info(4, 'I'),
  warn(5, 'W'),
  error(6, 'E'),
  assert_(7, 'A');

  const ClientLogLevel(this.value, this.code);

  final int value;
  final String code;
}

enum VerboseLevel {
  level1(1),
  level2(2),
  level3(3),
  level4(4),
  level5(5),
  level6(6);

  const VerboseLevel(this.value);

  final int value;
}
