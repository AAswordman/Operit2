import 'dart:collection';
import 'dart:typed_data';

/// Retains indexed program data directly in its immutable transport allocation.
class ComposeProgramBuffer {
  /// Validates the fixed envelope before exposing any indexed data.
  ComposeProgramBuffer(this.bytes) : _view = ByteData.sublistView(bytes) {
    if (bytes.length < 32 ||
        word(0) != 0x31425043 ||
        word(4) != 1 ||
        word(8) != bytes.length ||
        word(12) > 1) {
      throw const FormatException('Invalid Compose buffer header');
    }
  }
  final Uint8List bytes;
  final ByteData _view;
  final Map<int, String> _strings = {};

  /// Reads metadata only after checking its complete range.
  int word(int offset) {
    if (offset < 0 || offset + 4 > bytes.length) {
      throw const FormatException('Compose address exceeds buffer');
    }
    return _view.getUint32(offset, Endian.little);
  }

  /// Checks a complete indexed range without traversing its contents.
  void range(int offset, int count, int stride) {
    if (offset < 32 || count < 0 || offset + count * stride > bytes.length) {
      throw const FormatException('Compose range exceeds buffer');
    }
  }

  /// Reads one parameter while leaving nested containers in the same backing storage.
  Object? value(int offset, [int depth = 0]) {
    if (depth > 128 ||
        offset < 32 ||
        offset % 8 != 0 ||
        offset + 16 > bytes.length) {
      throw const FormatException('Invalid Compose value address');
    }
    final tag = word(offset), count = word(offset + 4), data = word(offset + 8);
    switch (tag) {
      case 0:
        return null;
      case 1:
        return false;
      case 2:
        return true;
      case 3:
        final number = _view.getFloat64(offset + 8, Endian.little);
        if (!number.isFinite) {
          throw const FormatException('Nonfinite Compose number');
        }
        return number >= -9223372036854775808.0 &&
                number < 9223372036854775808.0 &&
                number == number.truncateToDouble()
            ? number.toInt()
            : number;
      case 4:
        range(data, count, 2);
        return _strings.putIfAbsent(
          offset,
          () => String.fromCharCodes(
            List<int>.generate(
              count,
              (index) => _view.getUint16(data + index * 2, Endian.little),
              growable: false,
            ),
          ),
        );
      case 5:
        if (count > 1000000) {
          throw const FormatException('Too many Compose array members');
        }
        range(data, count, 4);
        return _BufferList(this, data, count, depth);
      case 6:
        if (count > 1000000) {
          throw const FormatException('Too many Compose object fields');
        }
        range(data, count, 8);
        return _BufferMap(this, data, count, depth);
      case 7:
        return const <String, Object?>{'__composeUndefined': true};
      case 8:
        return <String, Object?>{
          '__actionId': value(data, depth + 1) as String,
        };
      case 9:
        return <String, Object?>{
          '__composeRef': [value(data, depth + 1) as String, count],
        };
      default:
        throw const FormatException('Unknown Compose value tag');
    }
  }

  /// Returns a source transaction whose containers remain indexed views.
  ComposeProgramTransaction get transaction {
    final fields = value(word(16)) as List;
    if (fields.length != 7) {
      throw const FormatException('Invalid Compose transaction record');
    }
    return ComposeProgramTransaction(
      reset: fields[0] as bool,
      revision: fields[1] as int,
      root: ToolPkgComposeDslReference(
        instanceId: fields[2] as String,
        node: fields[3] as int,
      ),
      programs: _MappedList(
        fields[4] as List,
        (value) => ToolPkgComposeDslProgram.fromValue(value as Map),
      ),
      instances: _MappedList(
        fields[5] as List,
        (value) => ToolPkgComposeDslInstance.fromValue(value as List),
      ),
      removed: (fields[6] as List).cast<String>(),
    );
  }
}

/// Reads array elements by address rather than allocating a second element tree.
class _BufferList extends ListBase<Object?> {
  /// Binds a checked index table to its immutable arena.
  _BufferList(this.arena, this.offset, this.length, this.depth);
  final ComposeProgramBuffer arena;
  final int offset, depth;
  @override
  final int length;

  /// Rejects mutations of published storage.
  @override
  set length(int value) =>
      throw UnsupportedError('Compose buffers are immutable');

  /// Resolves only the addressed element.
  @override
  Object? operator [](int index) {
    RangeError.checkValidIndex(index, this);
    return arena.value(arena.word(offset + index * 4), depth + 1);
  }

  /// Rejects writes through a published array view.
  @override
  void operator []=(int index, Object? value) =>
      throw UnsupportedError('Compose buffers are immutable');
}

/// Reads object fields from their immutable indexed table.
class _BufferMap extends MapBase<String, Object?> {
  /// Retains a checked table without copying its values.
  _BufferMap(this.arena, this.offset, this.count, this.depth);
  final ComposeProgramBuffer arena;
  final int offset, count, depth;

  /// Reads the original field names in their indexed order.
  @override
  Iterable<String> get keys sync* {
    for (var index = 0; index < count; index++) {
      yield arena.value(arena.word(offset + index * 8), depth + 1) as String;
    }
  }

  /// Iterates each field once without MapBase repeating a linear lookup.
  @override
  Iterable<MapEntry<String, Object?>> get entries sync* {
    for (var index = 0; index < count; index++) {
      yield MapEntry(
        arena.value(arena.word(offset + index * 8), depth + 1) as String,
        arena.value(arena.word(offset + index * 8 + 4), depth + 1),
      );
    }
  }

  /// Resolves one matching field directly from the parameter storage.
  @override
  Object? operator [](Object? key) {
    for (var index = 0; index < count; index++) {
      if (arena.value(arena.word(offset + index * 8), depth + 1) == key) {
        return arena.value(arena.word(offset + index * 8 + 4), depth + 1);
      }
    }
    return null;
  }

  /// Rejects writes through a published object view.
  @override
  void operator []=(String key, Object? value) =>
      throw UnsupportedError('Compose buffers are immutable');

  /// Rejects removing values from immutable storage.
  @override
  Object? remove(Object? key) =>
      throw UnsupportedError('Compose buffers are immutable');

  /// Rejects clearing immutable storage.
  @override
  void clear() => throw UnsupportedError('Compose buffers are immutable');
}

/// Projects typed metadata without materializing nested parameter containers.
class _MappedList<T> extends ListBase<T> {
  /// Connects an indexed source to its typed projection.
  _MappedList(this.source, this.convert);
  final List source;
  final T Function(Object?) convert;

  /// Reads the checked table length.
  @override
  int get length => source.length;

  /// Rejects modifications to a typed arena view.
  @override
  set length(int value) =>
      throw UnsupportedError('Compose buffers are immutable');

  /// Projects one addressed record.
  @override
  T operator [](int index) => convert(source[index]);

  /// Rejects modifications to an indexed record.
  @override
  void operator []=(int index, T value) =>
      throw UnsupportedError('Compose buffers are immutable');
}

/// Holds a logical address independently of a mounted widget.
class ToolPkgComposeDslReference {
  /// Stores the owning instance and its local instruction.
  const ToolPkgComposeDslReference({
    required this.instanceId,
    required this.node,
  });
  final String instanceId;
  final int node;
}

/// Exposes immutable shared instruction metadata.
class ToolPkgComposeDslInstruction {
  /// Stores code operands independently of their source parameters.
  const ToolPkgComposeDslInstruction({
    required this.nodeType,
    required this.props,
    required this.children,
    required this.explicitChildren,
  });

  /// Projects one installed instruction from its arena object.
  factory ToolPkgComposeDslInstruction.fromValue(Map value) =>
      ToolPkgComposeDslInstruction(
        nodeType: value['type'] as String,
        props: value['props'],
        children: value['children'],
        explicitChildren: value['explicitChildren'] as bool,
      );
  final String nodeType;
  final Object? props, children;
  final bool explicitChildren;
}

/// Exposes one layout shared by all its source instances.
class ToolPkgComposeDslProgram {
  /// Stores the source program identity and its instruction table.
  const ToolPkgComposeDslProgram({
    required this.id,
    required this.root,
    required this.nodes,
  });

  /// Projects an installed definition without expanding its operands.
  factory ToolPkgComposeDslProgram.fromValue(Map value) =>
      ToolPkgComposeDslProgram(
        id: value['id'] as String,
        root: value['root'] as int,
        nodes: _MappedList(
          value['nodes'] as List,
          (value) => ToolPkgComposeDslInstruction.fromValue(value as Map),
        ),
      );
  final String id;
  final int root;
  final List<ToolPkgComposeDslInstruction> nodes;
}

/// Exposes one explicit public-node edit.
class ToolPkgComposeDslPatch {
  /// Stores the local instruction and edited field.
  const ToolPkgComposeDslPatch({
    required this.node,
    required this.field,
    required this.value,
  });

  /// Reads a checked fixed patch record.
  factory ToolPkgComposeDslPatch.fromValue(List value) {
    if (value.length != 3) {
      throw const FormatException('Invalid Compose patch record');
    }
    return ToolPkgComposeDslPatch(
      node: value[0] as int,
      field: value[1] as String,
      value: value[2],
    );
  }
  final int node;
  final String field;
  final Object? value;
}

/// Retains parameter views backed by the producer's allocation.
class ToolPkgComposeDslInstance {
  /// Stores source identity and its indexed values.
  const ToolPkgComposeDslInstance({
    required this.id,
    required this.programId,
    required this.values,
    required this.patches,
  });

  /// Reads a fixed instance record without copying its parameters.
  factory ToolPkgComposeDslInstance.fromValue(List value) {
    if (value.length != 4) {
      throw const FormatException('Invalid Compose instance record');
    }
    return ToolPkgComposeDslInstance(
      id: value[0] as String,
      programId: value[1] as String,
      values: (value[2] as List).cast<Object?>(),
      patches: _MappedList(
        value[3] as List,
        (value) => ToolPkgComposeDslPatch.fromValue(value as List),
      ),
    );
  }
  final String id, programId;
  final List<Object?> values;
  final List<ToolPkgComposeDslPatch> patches;
}

/// Carries transaction indexes while every parameter retains the same arena ownership.
class ComposeProgramTransaction {
  /// Stores one checked revision and its indexed record tables.
  const ComposeProgramTransaction({
    required this.reset,
    required this.revision,
    required this.root,
    required this.programs,
    required this.instances,
    required this.removed,
  });
  final bool reset;
  final int revision;
  final ToolPkgComposeDslReference root;
  final List<ToolPkgComposeDslProgram> programs;
  final List<ToolPkgComposeDslInstance> instances;
  final List<String> removed;
}
