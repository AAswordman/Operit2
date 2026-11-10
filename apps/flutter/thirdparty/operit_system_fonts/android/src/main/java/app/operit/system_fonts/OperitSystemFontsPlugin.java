package app.operit.system_fonts;

import io.flutter.embedding.engine.plugins.FlutterPlugin;
import io.flutter.plugin.common.MethodCall;
import io.flutter.plugin.common.MethodChannel;
import java.io.File;
import java.io.RandomAccessFile;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.Map;
import java.util.TreeMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import android.os.Handler;
import android.os.Looper;

/// Enumerates public Android font resources without private APIs or storage permissions.
public final class OperitSystemFontsPlugin implements FlutterPlugin, MethodChannel.MethodCallHandler {
  private final Map<String, Entry> fonts = new TreeMap<>();
  private final ExecutorService worker = Executors.newSingleThreadExecutor();
  private final Handler main = new Handler(Looper.getMainLooper());
  private MethodChannel channel;

  /// Retains a family, its physical face index, and its regular-style distance.
  private static final class Entry {
    final File file; final int index; final int score;
    Entry(File file, int index, int score) { this.file = file; this.index = index; this.score = score; }
  }

  /// Attaches this client-local font channel to its engine.
  @Override public void onAttachedToEngine(FlutterPluginBinding binding) {
    channel = new MethodChannel(binding.getBinaryMessenger(), "operit/system_fonts");
    channel.setMethodCallHandler(this);
  }

  /// Releases the worker and channel when the engine is detached.
  @Override public void onDetachedFromEngine(FlutterPluginBinding binding) {
    channel.setMethodCallHandler(null);
    worker.shutdown();
  }

  /// Keeps font discovery and file reads off Android's main thread.
  @Override public void onMethodCall(MethodCall call, MethodChannel.Result result) {
    worker.execute(() -> {
      try {
        Object value;
        switch (call.method) {
          case "listFamilies":
            fonts.clear();
            for (String root : new String[]{"/system/fonts", "/product/fonts", "/system_ext/fonts", "/vendor/fonts", "/data/fonts/files"}) {
              File directory = new File(root);
              if (!directory.isDirectory()) continue;
              File[] files = directory.listFiles();
              if (files == null) continue; // Only readable system resources belong to the catalog.
              for (File file : files) {
                String name = file.getName().toLowerCase(java.util.Locale.ROOT);
                if (name.endsWith(".ttf") || name.endsWith(".otf") || name.endsWith(".ttc") || name.endsWith(".otc")) catalog(file);
              }
            }
            value = new ArrayList<>(fonts.keySet());
            break;
          case "readFamily":
            Entry entry = fonts.get((String) call.arguments);
            if (entry == null) throw new IllegalArgumentException("Unknown installed font");
            if (entry.file.length() <= 0 || entry.file.length() > 256L * 1024 * 1024) throw new IllegalArgumentException("Invalid installed font size");
            Map<String, Object> payload = new HashMap<>();
            try (RandomAccessFile input = new RandomAccessFile(entry.file, "r")) {
              byte[] bytes = new byte[(int) input.length()];
              input.readFully(bytes);
              payload.put("bytes", bytes);
            }
            payload.put("index", entry.index);
            value = payload;
            break;
          default:
            main.post(result::notImplemented);
            return;
        }
        main.post(() -> result.success(value));
      } catch (Exception error) {
        main.post(() -> result.error("device_font_error", error.toString(), null));
      }
    });
  }

  /// Reads SFNT metadata without loading every installed font into Dart memory.
  private void catalog(File file) throws Exception {
    try (RandomAccessFile input = new RandomAccessFile(file, "r")) {
      int signature = input.readInt();
      long[] offsets;
      if (signature == 0x74746366) {
        input.readInt();
        int count = input.readInt();
        if (count <= 0 || count > 4096) throw new IllegalArgumentException("Invalid font collection");
        offsets = new long[count];
        for (int i = 0; i < count; i++) offsets[i] = Integer.toUnsignedLong(input.readInt());
      } else {
        offsets = new long[]{0};
      }
      for (int index = 0; index < offsets.length; index++) {
        long face = offsets[index];
        input.seek(face + 4);
        int count = input.readUnsignedShort();
        long nameTable = -1, os2 = -1;
        for (int i = 0; i < count; i++) {
          input.seek(face + 12 + i * 16L);
          int tag = input.readInt();
          input.readInt();
          long offset = Integer.toUnsignedLong(input.readInt());
          if (tag == 0x6e616d65) nameTable = offset;
          if (tag == 0x4f532f32) os2 = offset;
        }
        if (nameTable < 0) continue;
        input.seek(nameTable + 2);
        int records = input.readUnsignedShort();
        long strings = nameTable + input.readUnsignedShort();
        String family = null;
        int best = -1;
        for (int i = 0; i < records; i++) {
          input.seek(nameTable + 6 + i * 12L);
          int platform = input.readUnsignedShort();
          input.readUnsignedShort();
          int language = input.readUnsignedShort();
          int nameId = input.readUnsignedShort();
          int length = input.readUnsignedShort();
          long start = strings + input.readUnsignedShort();
          if ((nameId != 1 && nameId != 16) || (platform != 0 && platform != 3)) continue;
          int rank = (nameId == 16 ? 4 : 0) + (language == 0x409 ? 2 : 0) + (platform == 3 ? 1 : 0);
          if (rank <= best) continue;
          byte[] name = new byte[length];
          input.seek(start);
          input.readFully(name);
          family = new String(name, StandardCharsets.UTF_16BE);
          best = rank;
        }
        if (family == null || family.isEmpty()) continue;
        int score = 0;
        if (os2 >= 0) {
          input.seek(os2 + 4);
          score = Math.abs(input.readUnsignedShort() - 400);
          input.seek(os2 + 62);
          if ((input.readUnsignedShort() & 1) != 0) score += 1000;
        }
        Entry current = fonts.get(family);
        if (current == null || score < current.score) fonts.put(family, new Entry(file, index, score));
      }
    }
  }
}
