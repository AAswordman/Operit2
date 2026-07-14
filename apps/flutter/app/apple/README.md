# Apple local inference engine

Operit links the Sherpa ONNX engine into iOS builds because iOS applications
cannot execute downloaded native code. STT and TTS model files remain optional
runtime downloads managed by `operit-local-models`.

Run the following command before opening or building the iOS Xcode project:

```text
python tools/build_scripts/prepare_apple_sherpa.py
```

The command downloads Sherpa ONNX 1.13.2 from the official k2-fsa release,
verifies its exact size and SHA-256 digest, and installs the Sherpa ONNX and
ONNX Runtime XCFrameworks under `apps/flutter/app/apple/Frameworks`.

The generated framework directory and download cache are intentionally not
tracked by Git.
