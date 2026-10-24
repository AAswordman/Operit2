/// Replaces unpaired UTF-16 surrogates before text reaches Flutter paragraph layout.
String sanitizeUtf16(String value) {
  final units = value.codeUnits;
  final buffer = StringBuffer();
  for (var index = 0; index < units.length; index++) {
    final unit = units[index];
    if (unit >= 0xD800 && unit <= 0xDBFF) {
      if (index + 1 < units.length &&
          units[index + 1] >= 0xDC00 &&
          units[index + 1] <= 0xDFFF) {
        buffer.writeCharCode(unit);
        buffer.writeCharCode(units[++index]);
      } else {
        buffer.write('\uFFFD');
      }
    } else if (unit >= 0xDC00 && unit <= 0xDFFF) {
      buffer.write('\uFFFD');
    } else {
      buffer.writeCharCode(unit);
    }
  }
  return buffer.toString();
}
