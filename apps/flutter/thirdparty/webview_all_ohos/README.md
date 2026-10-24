# webview_all_ohos

This package is part of [`webview_all`](https://github.com/abandoft/webview_all).

Use [`webview_all`](https://pub.dev/packages/webview_all) directly in your application. Do not depend on this platform implementation package directly.


## Local OpenHarmony API 18 compatibility

Native changes are maintained in this vendored source and copied by the HAP
builder. ArkTS fields are declared outside constructor parameters, channel maps
use explicit types, and MethodCall payloads use the pinned embedding's `args`
property. File-selection cancellation retains its nullable response contract.
