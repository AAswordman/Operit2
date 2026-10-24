"""Verify that embedded Linux formatters cannot replace the app's libc calls."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

REPO_ROOT = Path(__file__).resolve().parents[2]
PATCH = REPO_ROOT / "tools/ios-runtime/ish/patches/0005-namespace-darwin-kernel-formatters.patch"


class IshFormatterSymbolsTests(unittest.TestCase):
    def test_kernel_and_host_keep_separate_formatters(self):
        compiler = shutil.which("clang")
        if compiler is None:
            self.skipTest("clang is required for the formatter link regression")
        additions = "\n".join(line[1:] for line in PATCH.read_text().splitlines()
                              if line.startswith("+") and not line.startswith("+++"))
        with tempfile.TemporaryDirectory(prefix="operit ish symbols ") as temporary:
            root = Path(temporary)
            kernel = root / "kernel.c"
            kernel.write_text('''#include <stddef.h>
#include <stdarg.h>
#define CONFIG_HOST_DARWIN 1
''' + additions + '''
int vsnprintf(char *buffer, size_t size, const char *format, va_list args) {
    (void)buffer; (void)size; (void)format; (void)args;
    return -123;
}
int call_kernel_formatter(const char *format, ...) {
    va_list args;
    va_start(args, format);
    int result = vsnprintf(NULL, 0, format, args);
    va_end(args);
    return result;
}
''')
            host = root / "host.c"
            host.write_text('''#include <stdio.h>
#include <stdarg.h>
#include <string.h>
extern int call_kernel_formatter(const char *, ...);
static int format_host(char *buffer, size_t size, const char *format, ...) {
    va_list args;
    va_start(args, format);
    int result = vsnprintf(buffer, size, format, args);
    va_end(args);
    return result;
}
int main(void) {
    char buffer[64];
    if (format_host(buffer, sizeof(buffer), "%.2f %d", 1.25, 7) != 6) return 1;
    if (strcmp(buffer, "1.25 7") != 0) return 2;
    if (call_kernel_formatter("%p", (void *)0) != -123) return 3;
    return 0;
}
''')
            executable = root / "regression"
            subprocess.run([compiler, "-Wall", "-Wextra", "-Werror", "-fno-builtin",
                            str(kernel), str(host), "-o", str(executable)],
                           check=True, capture_output=True, text=True, timeout=60)
            subprocess.run([str(executable)], check=True, capture_output=True, timeout=15)


if __name__ == "__main__":
    unittest.main()
