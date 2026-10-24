// Copyright 2026 The Operit Authors. All rights reserved.

#import <TargetConditionals.h>
#if TARGET_OS_IOS
#import <XCTest/XCTest.h>
#import <objc/runtime.h>
#import "OperitSafeWebView.h"

// Inject at WKWebView's superclass boundary: the exact call that threw in
// TestFlight. A Swift do/catch around setFrame would not pass these tests.
static __weak WKWebView *OperitFailingView;
static NSString *OperitInjectedException;
static BOOL OperitInjectInvalidLayerBounds;

@interface WKWebView (OperitGeometryTest)
- (void)operitTestSetFrame:(CGRect)frame;
@end

@implementation WKWebView (OperitGeometryTest)
- (void)operitTestSetFrame:(CGRect)frame {
  if (self == OperitFailingView && OperitInjectInvalidLayerBounds) {
    // Let QuartzCore generate the real exception, rather than only emulating it.
    self.scrollView.layer.bounds = CGRectMake(NAN, 0, 320, 600);
  }
  if (self == OperitFailingView && OperitInjectedException) {
    [NSException raise:OperitInjectedException format:@"Injected WebKit geometry failure"];
  }
  [self operitTestSetFrame:frame];
}
@end

@interface OperitGeometryNavigationDelegate : NSObject <WKNavigationDelegate>
@property(nonatomic, strong) NSError *error;
@property(nonatomic) NSUInteger errorCount;
@property(nonatomic, strong) XCTestExpectation *failure;
@end
@implementation OperitGeometryNavigationDelegate
- (void)webView:(WKWebView *)webView didFailNavigation:(WKNavigation *)navigation
     withError:(NSError *)error {
  self.error = error;
  self.errorCount++;
  [self.failure fulfill];
}
@end

@interface WebViewGeometryTests : XCTestCase
@end

@implementation WebViewGeometryTests
- (void)setUp {
  [super setUp];
  Method original = class_getInstanceMethod(WKWebView.class, @selector(setFrame:));
  Method replacement = class_getInstanceMethod(WKWebView.class, @selector(operitTestSetFrame:));
  method_exchangeImplementations(original, replacement);
}

- (void)tearDown {
  OperitFailingView = nil;
  OperitInjectedException = nil;
  OperitInjectInvalidLayerBounds = NO;
  Method original = class_getInstanceMethod(WKWebView.class, @selector(setFrame:));
  Method replacement = class_getInstanceMethod(WKWebView.class, @selector(operitTestSetFrame:));
  method_exchangeImplementations(original, replacement);
  [super tearDown];
}

- (OperitSafeWebView *)makeView {
  return [[OperitSafeWebView alloc] initWithFrame:CGRectZero
                                  configuration:[[WKWebViewConfiguration alloc] init]];
}

- (void)testNormalZeroResizeAndRecreation {
  UIView *container = [[UIView alloc] initWithFrame:CGRectMake(0, 0, 400, 800)];
  for (NSUInteger index = 0; index < 8; index++) {
    @autoreleasepool {
      OperitSafeWebView *view = [self makeView];
      view.autoresizingMask = UIViewAutoresizingFlexibleWidth | UIViewAutoresizingFlexibleHeight;
      [container addSubview:view];
      for (NSValue *value in @[[NSValue valueWithCGRect:CGRectZero],
                              [NSValue valueWithCGRect:CGRectMake(0, 0, 320, 600)],
                              [NSValue valueWithCGRect:CGRectMake(10, 20, 600, 320)]]) {
        CGRect frame = value.CGRectValue;
        view.frame = frame;
        [view setNeedsLayout];
        [view layoutIfNeeded];
        XCTAssertTrue(CGRectEqualToRect(view.frame, frame));
        XCTAssertFalse(view.hidden);
        XCTAssertEqual(view.scrollView.contentInsetAdjustmentBehavior,
                       UIScrollViewContentInsetAdjustmentNever);
        XCTAssertFalse(view.scrollView.automaticallyAdjustsScrollIndicatorInsets);
        XCTAssertTrue(UIEdgeInsetsEqualToEdgeInsets(view.scrollView.adjustedContentInset,
                                                  UIEdgeInsetsZero));
        XCTAssertTrue(isfinite(view.scrollView.contentOffset.x));
        XCTAssertTrue(isfinite(view.scrollView.contentOffset.y));
      }
      container.frame = CGRectMake(0, 0, 390, 844);
      [view removeFromSuperview];
    }
  }
}

- (void)testInvalidFramesAreContainedBeforeReachingWebKit {
  NSArray<NSValue *> *invalid = @[
    [NSValue valueWithCGRect:CGRectMake(NAN, 0, 320, 600)],
    [NSValue valueWithCGRect:CGRectMake(0, INFINITY, 320, 600)],
    [NSValue valueWithCGRect:CGRectMake(0, 0, INFINITY, 600)],
    [NSValue valueWithCGRect:CGRectMake(0, 0, 320, NAN)],
    [NSValue valueWithCGRect:CGRectMake(0, 0, -1, 600)],
    [NSValue valueWithCGRect:CGRectMake(CGFLOAT_MAX, 0, CGFLOAT_MAX, 600)],
  ];
  for (NSValue *value in invalid) {
    OperitSafeWebView *view = [self makeView];
    view.frame = CGRectMake(0, 0, 320, 600);
    view.frame = value.CGRectValue;
    XCTAssertTrue(view.hidden);
    XCTAssertTrue(CGRectEqualToRect(view.frame, CGRectMake(0, 0, 320, 600)));
  }
}

- (void)testInvalidBoundsAndInitialFrameAreContained {
  OperitSafeWebView *view = [self makeView];
  view.bounds = CGRectMake(0, 0, 320, NAN);
  XCTAssertTrue(view.hidden);
  XCTAssertTrue(CGRectEqualToRect(view.bounds, CGRectZero));
  OperitSafeWebView *initial = [[OperitSafeWebView alloc]
    initWithFrame:CGRectMake(0, 0, INFINITY, 600)
    configuration:[[WKWebViewConfiguration alloc] init]];
  XCTAssertTrue(initial.hidden);
  XCTAssertTrue(CGRectEqualToRect(initial.frame, CGRectZero));
}

- (void)testWebKitGeometryExceptionOnlyDisablesFailingViewAndReportsOnce {
  OperitSafeWebView *view = [self makeView];
  OperitGeometryNavigationDelegate *delegate = [[OperitGeometryNavigationDelegate alloc] init];
  delegate.failure = [self expectationWithDescription:@"Geometry error reaches existing callback"];
  view.navigationDelegate = delegate;
  OperitFailingView = view;
  OperitInjectedException = @"CALayerInvalidGeometry";
  view.frame = CGRectMake(0, 0, 320, 600);
  XCTAssertTrue(view.hidden);
  // Recomposition must not re-enter a WKWebView whose update was interrupted.
  view.frame = CGRectMake(0, 0, 600, 320);
  view.bounds = CGRectMake(0, 0, 600, 320);
  [view layoutSubviews];
  [view safeAreaInsetsDidChange];
  [self waitForExpectationsWithTimeout:5 handler:nil];
  XCTAssertEqual(delegate.errorCount, 1u);
  XCTAssertEqualObjects(delegate.error.domain, @"OperitWebViewGeometryError");
  XCTAssertTrue([delegate.error.localizedDescription containsString:@"CALayerInvalidGeometry"]);
  // Another plugin/native view remains usable in the same process.
  OperitSafeWebView *healthy = [self makeView];
  healthy.frame = CGRectMake(0, 0, 390, 844);
  XCTAssertFalse(healthy.hidden);
  XCTAssertEqual(healthy.frame.size.height, 844);
}

- (void)testRealQuartzCoreInvalidScrollBoundsDoesNotTerminateProcess {
  OperitSafeWebView *view = [self makeView];
  OperitGeometryNavigationDelegate *delegate = [[OperitGeometryNavigationDelegate alloc] init];
  delegate.failure = [self expectationWithDescription:@"Real CALayer exception is reported"];
  view.navigationDelegate = delegate;
  OperitFailingView = view;
  OperitInjectInvalidLayerBounds = YES;
  view.frame = CGRectMake(0, 0, 320, 600);
  [self waitForExpectationsWithTimeout:5 handler:nil];
  XCTAssertTrue(view.hidden);
  XCTAssertEqual(delegate.errorCount, 1u);
  XCTAssertTrue([delegate.error.localizedDescription containsString:@"CALayerInvalidGeometry"]);
  OperitSafeWebView *replacement = [self makeView];
  replacement.frame = CGRectMake(0, 0, 320, 600);
  XCTAssertFalse(replacement.hidden);
}

- (void)testUnrelatedExceptionsKeepTheirOriginalFailure {
  OperitSafeWebView *view = [self makeView];
  OperitFailingView = view;
  OperitInjectedException = NSInternalInconsistencyException;
  XCTAssertThrowsSpecificNamed(view.frame = CGRectMake(0, 0, 320, 600),
                               NSException, NSInternalInconsistencyException);
  XCTAssertFalse(view.hidden);
}
@end
#endif
