package io.flutter.plugins.webviewflutter;

import static org.junit.Assert.assertSame;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.assertFalse;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.mockStatic;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import android.content.Intent;
import android.net.Uri;
import org.junit.Test;
import org.mockito.MockedStatic;

/** Verifies native Intent parsing and the application-link boundary. */
public class WebViewExternalNavigationTest {
  /** Keeps the parsed target package and extras without allowing explicit components or grants. */
  @Test
  public void parsesIntentAndRestrictsItsLaunchContract() throws Exception {
    String url = "intent://scan/#Intent;scheme=zxing;package=com.example.scanner;end";
    Uri original = mock(Uri.class);
    Uri data = mock(Uri.class);
    Intent parsed = mock(Intent.class);
    when(original.getScheme()).thenReturn("intent");
    when(data.getScheme()).thenReturn("zxing");
    when(parsed.getData()).thenReturn(data);
    try (MockedStatic<Uri> uris = mockStatic(Uri.class);
         MockedStatic<Intent> intents = mockStatic(Intent.class)) {
      uris.when(() -> Uri.parse(url)).thenReturn(original);
      intents.when(() -> Intent.parseUri(url, Intent.URI_INTENT_SCHEME)).thenReturn(parsed);
      assertSame(parsed, WebViewExternalNavigation.createIntent(url));
      intents.verify(() -> Intent.parseUri(url, Intent.URI_INTENT_SCHEME));
      verify(parsed).setAction(Intent.ACTION_VIEW);
      verify(parsed).addCategory(Intent.CATEGORY_BROWSABLE);
      verify(parsed).setComponent(null);
      verify(parsed).setSelector(null);
      verify(parsed).setFlags(0);
      verify(parsed).removeExtra("browser_fallback_url");
      verify(parsed, never()).setPackage(anyString());
    }
  }

  /** Rejects a serialized Intent that attempts to expose an application-private resource. */
  @Test
  public void rejectsPrivateResourceIntents() throws Exception {
    String url = "intent://private/#Intent;scheme=content;end";
    Uri original = mock(Uri.class);
    Uri data = mock(Uri.class);
    Intent parsed = mock(Intent.class);
    when(original.getScheme()).thenReturn("intent");
    when(data.getScheme()).thenReturn("content");
    when(parsed.getData()).thenReturn(data);
    try (MockedStatic<Uri> uris = mockStatic(Uri.class);
         MockedStatic<Intent> intents = mockStatic(Intent.class)) {
      uris.when(() -> Uri.parse(url)).thenReturn(original);
      intents.when(() -> Intent.parseUri(url, Intent.URI_INTENT_SCHEME)).thenReturn(parsed);
      assertThrows(IllegalArgumentException.class, () -> WebViewExternalNavigation.createIntent(url));
    }
  }

  /** Allows normal frame navigation while capturing custom application protocols. */
  @Test
  public void classifiesSubframeApplicationLinks() {
    Uri uri = mock(Uri.class);
    when(uri.getScheme()).thenReturn("https");
    assertFalse(WebViewExternalNavigation.isApplicationUri(uri));
    when(uri.getScheme()).thenReturn("WEIXIN");
    assertTrue(WebViewExternalNavigation.isApplicationUri(uri));
    when(uri.getScheme()).thenReturn("intent");
    assertTrue(WebViewExternalNavigation.isApplicationUri(uri));
  }
}
