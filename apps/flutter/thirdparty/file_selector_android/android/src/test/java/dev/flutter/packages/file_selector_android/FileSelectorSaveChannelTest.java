// Copyright 2026 The Operit Authors.
// Use of this source code is governed by a BSD-style license that can be found in the LICENSE file.
package dev.flutter.packages.file_selector_android;

import static org.junit.Assert.*;
import static org.mockito.Mockito.*;
import static org.robolectric.Shadows.shadowOf;

import android.app.Activity;
import android.content.Context;
import android.content.Intent;
import android.net.Uri;
import android.os.Looper;
import androidx.test.core.app.ApplicationProvider;
import io.flutter.embedding.engine.plugins.activity.ActivityPluginBinding;
import io.flutter.plugin.common.BinaryMessenger;
import io.flutter.plugin.common.MethodCall;
import io.flutter.plugin.common.MethodChannel;
import java.io.File;
import java.nio.file.Files;
import java.util.List;
import java.util.Map;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import org.junit.After;
import org.junit.Before;
import org.junit.Rule;
import org.junit.Test;
import org.junit.rules.TemporaryFolder;
import org.mockito.ArgumentCaptor;
import org.robolectric.RobolectricTestRunner;
import org.robolectric.annotation.Config;
import org.junit.runner.RunWith;

/** Exercises document staging and deletion through the production Android channel. */
@RunWith(RobolectricTestRunner.class)
@Config(manifest = Config.NONE, sdk = 28)
public class FileSelectorSaveChannelTest {
  @Rule public TemporaryFolder directory = new TemporaryFolder();
  private FileSelectorSaveChannel channel;
  private Activity activity;

  /** Creates the real channel with a filesystem-capable application context. */
  @Before
  public void setUp() {
    Context context = ApplicationProvider.getApplicationContext();
    activity = mock(Activity.class);
    when(activity.getApplicationContext()).thenReturn(context);
    ActivityPluginBinding binding = mock(ActivityPluginBinding.class);
    when(binding.getActivity()).thenReturn(activity);
    channel = new FileSelectorSaveChannel(mock(BinaryMessenger.class));
    channel.setActivityPluginBinding(binding);
  }

  /** Releases every outstanding copy when a test leaves the plugin attached. */
  @After
  public void tearDown() {
    if (channel != null) channel.close();
  }

  /** Deletes the picker-owned copy while preserving the selected original exactly. */
  @Test
  public void releaseDeletesOnlyOwnedCopy() throws Exception {
    File original = directory.newFile("backup.zip");
    byte[] content = new byte[] {1, 4, 9, 16};
    Files.write(original.toPath(), content);
    File selected = select(original);
    assertArrayEquals(content, Files.readAllBytes(selected.toPath()));
    Response released = release(selected);
    assertNull(released.errorCode);
    assertFalse(selected.exists());
    assertArrayEquals(content, Files.readAllBytes(original.toPath()));
    assertEquals("UNKNOWN_SOURCE", release(selected).errorCode);
  }

  /** Rejects arbitrary paths without deleting either the original or the owned copy. */
  @Test
  public void releaseRejectsUnownedOriginal() throws Exception {
    File original = directory.newFile("backup.zip");
    Files.write(original.toPath(), new byte[] {3, 2, 1});
    File selected = select(original);
    assertEquals("UNKNOWN_SOURCE", release(original).errorCode);
    assertTrue(original.isFile());
    assertTrue(selected.isFile());
    assertNull(release(selected).errorCode);
  }

  /** Deletes unclaimed source staging after the plugin detaches. */
  @Test
  public void detachReleasesUnclaimedCopy() throws Exception {
    File original = directory.newFile("backup.zip");
    File selected = select(original);
    channel.close();
    channel = null;
    long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(3);
    while (selected.exists() && System.nanoTime() < deadline) Thread.sleep(5);
    assertFalse(selected.exists());
    assertTrue(original.isFile());
  }

  /** Stages one source through activity selection and waits for its real background copy. */
  private File select(File original) throws Exception {
    Response response = new Response();
    channel.onMethodCall(new MethodCall("openFileWithProgress", Map.of("mimeTypes", List.of("application/zip"))), response);
    ArgumentCaptor<Integer> code = ArgumentCaptor.forClass(Integer.class);
    verify(activity).startActivityForResult(any(Intent.class), code.capture());
    channel.onActivityResult(code.getValue(), Activity.RESULT_OK, new Intent().setData(Uri.fromFile(original)));
    response.await();
    assertNull(response.errorCode);
    return new File((String) ((Map<?, ?>) response.value).get("path"));
  }

  /** Requests deletion through the channel rather than directly changing the filesystem. */
  private Response release(File file) throws Exception {
    Response response = new Response();
    channel.onMethodCall(new MethodCall("releaseSelectedFile", Map.of("path", file.getAbsolutePath())), response);
    response.await();
    return response;
  }

  /** Captures one result while the actual worker and main-looper callbacks execute. */
  private static class Response implements MethodChannel.Result {
    final CountDownLatch done = new CountDownLatch(1);
    Object value;
    String errorCode;

    /** Records completion of a production channel operation. */
    @Override
    public void success(Object value) { this.value = value; done.countDown(); }

    /** Records the native diagnostic returned by a rejected operation. */
    @Override
    public void error(String code, String message, Object details) { errorCode = code; done.countDown(); }

    /** Records unsupported operations as failures visible to the test. */
    @Override
    public void notImplemented() { errorCode = "NOT_IMPLEMENTED"; done.countDown(); }

    /** Pumps main-thread delivery while waiting for the bounded background operation. */
    void await() throws Exception {
      long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(3);
      while (done.getCount() != 0 && System.nanoTime() < deadline) {
        shadowOf(Looper.getMainLooper()).idle();
        done.await(5, TimeUnit.MILLISECONDS);
      }
      assertEquals("The native operation did not complete", 0, done.getCount());
    }
  }
}
