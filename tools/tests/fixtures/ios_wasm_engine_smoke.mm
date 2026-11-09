#import <Foundation/Foundation.h>
extern "C" int operit_ios_wasm_engine_smoke(const char *path);
int main(int argc, char **argv) {
  @autoreleasepool {
    if (argc != 2) return 64;
    return operit_ios_wasm_engine_smoke(argv[1]);
  }
}
