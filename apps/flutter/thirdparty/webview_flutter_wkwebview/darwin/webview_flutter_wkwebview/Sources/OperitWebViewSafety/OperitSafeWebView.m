// Copyright 2026 The Operit Authors. All rights reserved.

#import "OperitSafeWebView.h"

#if TARGET_OS_IOS
#import <math.h>

static BOOL OperitValidRect(CGRect rect) {
  return isfinite(rect.origin.x) && isfinite(rect.origin.y) &&
         isfinite(rect.size.width) && isfinite(rect.size.height) &&
         rect.size.width >= 0 && rect.size.height >= 0 &&
         isfinite(rect.origin.x + rect.size.width) &&
         isfinite(rect.origin.y + rect.size.height);
}

static BOOL OperitFiniteInsets(UIEdgeInsets insets) {
  return isfinite(insets.top) && isfinite(insets.left) &&
         isfinite(insets.bottom) && isfinite(insets.right);
}

@implementation OperitSafeWebView {
  BOOL _operitGeometryFailed;
}

- (instancetype)initWithFrame:(CGRect)frame configuration:(WKWebViewConfiguration *)configuration {
  // WKWebView is normally constructed at CGRectZero, before Flutter measures it.
  self = [super initWithFrame:OperitValidRect(frame) ? frame : CGRectZero
               configuration:configuration];
  if (self) {
    @try {
      [self operitPrepareScrollView];
    } @catch (NSException *exception) {
      [self operitHandleGeometryException:exception operation:@"initialization"];
    }
    if (!OperitValidRect(frame)) {
      [self operitFailGeometry:@"Invalid initial WebView frame"];
    }
  }
  return self;
}

- (void)setFrame:(CGRect)frame {
  if (_operitGeometryFailed) return;
  if (!OperitValidRect(frame)) {
    [self operitFailGeometry:[@"Invalid WebView frame: " stringByAppendingString:NSStringFromCGRect(frame)]];
    return;
  }
  // Swift cannot catch NSException. Keep both the superclass call and inset
  // adjustment inside Objective-C, including WebKit's internal scroll updates.
  @try {
    [self operitPrepareScrollView];
    [super setFrame:frame];
    [self operitAdjustInsets];
  } @catch (NSException *exception) {
    [self operitHandleGeometryException:exception operation:@"setFrame"];
  }
}

- (void)setBounds:(CGRect)bounds {
  if (_operitGeometryFailed) return;
  if (!OperitValidRect(bounds)) {
    [self operitFailGeometry:[@"Invalid WebView bounds: " stringByAppendingString:NSStringFromCGRect(bounds)]];
    return;
  }
  @try {
    [super setBounds:bounds];
  } @catch (NSException *exception) {
    [self operitHandleGeometryException:exception operation:@"setBounds"];
  }
}

- (void)layoutSubviews {
  if (_operitGeometryFailed) return;
  @try {
    [self operitPrepareScrollView];
    [super layoutSubviews];
  } @catch (NSException *exception) {
    [self operitHandleGeometryException:exception operation:@"layoutSubviews"];
  }
}

- (void)safeAreaInsetsDidChange {
  if (_operitGeometryFailed) return;
  @try {
    [self operitPrepareScrollView];
    [super safeAreaInsetsDidChange];
    [self operitAdjustInsets];
  } @catch (NSException *exception) {
    [self operitHandleGeometryException:exception operation:@"safeAreaInsetsDidChange"];
  }
}

- (void)operitPrepareScrollView {
  UIScrollView *scrollView = self.scrollView;
  // Configure this before setFrame: can recalculate the scroll position. Doing
  // it afterwards is too late for the TestFlight CALayer.setBounds exception.
  scrollView.contentInsetAdjustmentBehavior = UIScrollViewContentInsetAdjustmentNever;
  scrollView.automaticallyAdjustsScrollIndicatorInsets = NO;
  CGPoint offset = scrollView.contentOffset;
  if (!isfinite(offset.x) || !isfinite(offset.y)) {
    scrollView.contentOffset = CGPointZero;
  }
  if (!OperitFiniteInsets(scrollView.contentInset)) {
    scrollView.contentInset = UIEdgeInsetsZero;
  }
}

- (void)operitAdjustInsets {
  UIScrollView *scrollView = self.scrollView;
  // Preserve Flutter's ownership of insets, including the existing compensation
  // for adjustedContentInset on UIKit versions that still supply system insets.
  if (!UIEdgeInsetsEqualToEdgeInsets(scrollView.contentInset, UIEdgeInsetsZero)) {
    scrollView.contentInset = UIEdgeInsetsZero;
  }
  UIEdgeInsets adjusted = scrollView.adjustedContentInset;
  if (!OperitFiniteInsets(adjusted)) {
    [self operitFailGeometry:@"Non-finite WebView adjustedContentInset"];
    return;
  }
  if (!UIEdgeInsetsEqualToEdgeInsets(adjusted, UIEdgeInsetsZero)) {
    scrollView.contentInset = UIEdgeInsetsMake(-adjusted.top, -adjusted.left,
                                              -adjusted.bottom, -adjusted.right);
  }
}

- (void)operitHandleGeometryException:(NSException *)exception operation:(NSString *)operation {
  // Contain only the geometry exception seen in TestFlight. Programming errors
  // and unrelated native exceptions must keep their original diagnostics.
  if (![exception.name isEqualToString:@"CALayerInvalidGeometry"]) {
    @throw exception;
  }
  [self operitFailGeometry:[NSString stringWithFormat:@"WebView %@: %@: %@",
                           operation, exception.name, exception.reason ?: @""]];
}

- (void)operitFailGeometry:(NSString *)reason {
  if (_operitGeometryFailed) return;
  _operitGeometryFailed = YES;
  // UIKit may have partially changed the view before raising. Do not reuse or
  // try to repair that native view; retain it inert until its owner disposes it.
  self.hidden = YES;
  NSLog(@"OperitWebViewGeometryError: %@; frame=%@ bounds=%@ offset=%@ insets=%@",
        reason, NSStringFromCGRect(self.frame), NSStringFromCGRect(self.bounds),
        NSStringFromCGPoint(self.scrollView.contentOffset),
        NSStringFromUIEdgeInsets(self.scrollView.contentInset));
  NSMutableDictionary *info = [@{NSLocalizedDescriptionKey : reason} mutableCopy];
  if (self.URL.absoluteString) {
    info[NSURLErrorFailingURLStringErrorKey] = self.URL.absoluteString;
  }
  NSError *error = [NSError errorWithDomain:@"OperitWebViewGeometryError" code:1 userInfo:info];
  // Leave the current UIKit composition before invoking plugin/Dart callbacks.
  // Reuse onWebResourceError so a plugin can show its existing load-error UI.
  __weak OperitSafeWebView *weakSelf = self;
  dispatch_async(dispatch_get_main_queue(), ^{
    OperitSafeWebView *view = weakSelf;
    if (!view) return;
    [view stopLoading];
    id<WKNavigationDelegate> delegate = view.navigationDelegate;
    if ([delegate respondsToSelector:@selector(webView:didFailNavigation:withError:)]) {
      [delegate webView:view didFailNavigation:nil withError:error];
    }
  });
}

@end
#endif
