# file\_selector\_ohos

The Ohos implementation of [`file_selector`][1].

## Usage

This package is [endorsed][2], which means you can simply use `file_selector`
normally. This package will be automatically included in your app when you do,
so you do not need to add it to your `pubspec.yaml`.

However, if you `import` this package to use any of its APIs directly, you
should add it to your `pubspec.yaml` as usual.

[1]: https://pub.dev/packages/file_selector
[2]: https://flutter.dev/docs/development/packages-and-plugins/developing-packages#endorsed-federated-plugin


## Local OpenHarmony API 18 compatibility

Native changes are maintained in this vendored source and copied by the HAP
builder. ArkTS fields are declared outside constructor parameters, channel maps
use explicit types, and MethodCall payloads use the pinned embedding's `args`
property. File-selection cancellation retains its nullable response contract.
