// ignore_for_file: file_names

enum ClientLogLevel {
  verbose('V'),
  debug('D'),
  info('I'),
  warn('W'),
  error('E'),
  assert_('A');

  const ClientLogLevel(this.code);

  final String code;
}
