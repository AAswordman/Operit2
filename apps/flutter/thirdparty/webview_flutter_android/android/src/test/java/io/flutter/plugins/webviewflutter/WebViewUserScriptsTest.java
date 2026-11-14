package io.flutter.plugins.webviewflutter;

import static org.junit.Assert.assertNotEquals;
import static org.junit.Assert.assertTrue;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.ArgumentMatchers.isNull;
import static org.mockito.Mockito.clearInvocations;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.mockStatic;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import android.webkit.WebView;
import androidx.webkit.ScriptHandler;
import androidx.webkit.WebViewCompat;
import androidx.webkit.WebViewFeature;
import io.flutter.plugin.common.MethodCall;
import io.flutter.plugin.common.MethodChannel;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
import org.junit.Test;
import org.mockito.ArgumentCaptor;
import org.mockito.MockedStatic;

/** Exercises provider selection, lifecycle delivery and script ownership. */
public class WebViewUserScriptsTest {
  /** Sends one script command through the production Flutter method handler. */
  private static MethodChannel.Result call(
      WebViewUserScripts scripts, String method, String key, String value) {
    Map<String, Object> arguments = new HashMap<>();
    arguments.put("viewIdentifier", 1L);
    arguments.put(key, value);
    MethodChannel.Result result = mock(MethodChannel.Result.class);
    scripts.onMethodCall(new MethodCall(method, arguments), result);
    return result;
  }

  /** Preserves native pre-page execution on providers that advertise support. */
  @Test
  public void supportedProviderUsesNativeRegistration() {
    try (MockedStatic<WebViewFeature> features = mockStatic(WebViewFeature.class);
        MockedStatic<WebViewCompat> compat = mockStatic(WebViewCompat.class)) {
      features.when(() -> WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT))
          .thenReturn(true);
      TestProxyApiRegistrar registrar = new TestProxyApiRegistrar();
      WebView view = mock(WebView.class);
      registrar.getInstanceManager().addDartCreatedInstance(view, 1L);
      ScriptHandler handler = mock(ScriptHandler.class);
      compat.when(() -> WebViewCompat.addDocumentStartJavaScript(
          view, "window.nativeReady=true;", Collections.singleton("*"))).thenReturn(handler);

      MethodChannel.Result result = call(registrar.userScripts, "add", "source", "window.nativeReady=true;");
      verify(result).success("operit-user-script-1");
      registrar.userScripts.injectLegacyScripts(view);
      verify(view, never()).evaluateJavascript(anyString(), isNull());
      call(registrar.userScripts, "remove", "identifier", "operit-user-script-1");
      verify(handler).remove();
      registrar.userScripts.dispose();
    }
  }

  /** Accepts unsupported providers, retains order and removes scripts independently. */
  @Test
  public void unsupportedProviderInjectsInOrderAndHonorsRemoval() {
    try (MockedStatic<WebViewFeature> features = mockStatic(WebViewFeature.class);
        MockedStatic<WebViewCompat> compat = mockStatic(WebViewCompat.class)) {
      TestProxyApiRegistrar registrar = new TestProxyApiRegistrar();
      WebView view = mock(WebView.class);
      registrar.getInstanceManager().addDartCreatedInstance(view, 1L);
      verify(call(registrar.userScripts, "add", "source", "window.first=true;"))
          .success("operit-user-script-1");
      verify(call(registrar.userScripts, "add", "source", "window.second=true;"))
          .success("operit-user-script-2");
      compat.verifyNoInteractions();
      verify(view, never()).evaluateJavascript(anyString(), isNull());

      registrar.userScripts.injectLegacyScripts(view);
      ArgumentCaptor<String> captured = ArgumentCaptor.forClass(String.class);
      verify(view, times(2)).evaluateJavascript(captured.capture(), isNull());
      assertTrue(captured.getAllValues().get(0).contains("window.first=true;"));
      assertTrue(captured.getAllValues().get(1).contains("window.second=true;"));
      assertNotEquals(captured.getAllValues().get(0), captured.getAllValues().get(1));
      clearInvocations(view);
      call(registrar.userScripts, "remove", "identifier", "operit-user-script-1");
      registrar.userScripts.injectLegacyScripts(view);
      verify(view).evaluateJavascript(eq(captured.getAllValues().get(1)), isNull());
      clearInvocations(view);
      call(registrar.userScripts, "removeAll", "unused", "");
      registrar.userScripts.injectLegacyScripts(view);
      verify(view, never()).evaluateJavascript(anyString(), isNull());
      registrar.userScripts.dispose();
    }
  }

  /** Delivers scripts from every navigation hook while keeping view ownership isolated. */
  @Test
  public void lifecycleCallbacksInjectOnlyIntoTheirOwnerAndDestroyClearsRegistration() {
    try (MockedStatic<WebViewFeature> features = mockStatic(WebViewFeature.class)) {
      TestProxyApiRegistrar registrar = new TestProxyApiRegistrar();
      WebView view = mock(WebView.class);
      WebView other = mock(WebView.class);
      registrar.getInstanceManager().addDartCreatedInstance(view, 1L);
      call(registrar.userScripts, "add", "source", "window.ready=true;");
      WebViewClientProxyApi api = mock(WebViewClientProxyApi.class);
      when(api.getPigeonRegistrar()).thenReturn(registrar);
      WebViewClientProxyApi.WebViewClientImpl client = new WebViewClientProxyApi.WebViewClientImpl(api);
      client.onPageStarted(view, "https://example.com", null);
      client.onPageCommitVisible(view, "https://example.com");
      client.onPageFinished(view, "https://example.com");
      verify(view, times(3)).evaluateJavascript(anyString(), isNull());
      client.onPageStarted(other, "https://example.com", null);
      verify(other, never()).evaluateJavascript(anyString(), isNull());
      clearInvocations(view);
      new WebViewProxyApi(registrar).destroy(view);
      registrar.userScripts.injectLegacyScripts(view);
      verify(view).destroy();
      verify(view, never()).evaluateJavascript(anyString(), isNull());
      registrar.userScripts.dispose();
    }
  }

  /** Stops further lifecycle injection when the Flutter engine detaches. */
  @Test
  public void disposeClearsLegacyScripts() {
    try (MockedStatic<WebViewFeature> features = mockStatic(WebViewFeature.class)) {
      TestProxyApiRegistrar registrar = new TestProxyApiRegistrar();
      WebView view = mock(WebView.class);
      registrar.getInstanceManager().addDartCreatedInstance(view, 1L);
      call(registrar.userScripts, "add", "source", "window.ready=true;");
      registrar.userScripts.dispose();
      registrar.userScripts.injectLegacyScripts(view);
      verify(view, never()).evaluateJavascript(anyString(), isNull());
    }
  }
}
