// Copyright 2026 The Operit Authors. All rights reserved.

#import <TargetConditionals.h>
#import <WebKit/WebKit.h>

#if TARGET_OS_IOS
NS_ASSUME_NONNULL_BEGIN

/// Contains invalid UIKit/WebKit geometry within the affected platform view.
/// Other WebViews and the Flutter application remain usable after a failure.
@interface OperitSafeWebView : WKWebView
@end

NS_ASSUME_NONNULL_END
#endif
