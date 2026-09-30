import 'dart:async';
import 'dart:js_interop';

import 'package:flutter/services.dart';
import 'package:image_picker_platform_interface/image_picker_platform_interface.dart';
import 'package:web/web.dart' as web;

/// Uses an actual browser media stream instead of an advisory file-input capture attribute.
class BrowserCameraCapture {
  static bool _active = false;

  static Future<XFile?> capture({
    required bool video,
    required CameraDevice device,
    Duration? maxDuration,
  }) async {
    if (_active)
      throw PlatformException(
        code: 'camera_busy',
        message: 'Camera capture is already active',
      );
    if (!web.window.isSecureContext)
      throw PlatformException(
        code: 'camera_insecure_context',
        message: 'Camera access requires HTTPS or localhost',
      );
    if (maxDuration != null && maxDuration <= Duration.zero)
      throw ArgumentError.value(maxDuration, 'maxDuration');
    _active = true;
    final done = Completer<XFile?>();
    web.MediaStream? stream;
    web.MediaRecorder? recorder;
    Timer? timer;
    final chunks = <web.Blob>[];
    final panel = web.HTMLDialogElement();
    panel.style.cssText =
        'padding:16px;background:#202124;color:white;border:0;border-radius:12px;max-width:90vw;';
    panel.setAttribute('aria-label', video ? 'Record video' : 'Take photo');
    final preview = web.HTMLVideoElement()
      ..autoplay = true
      ..muted = true;
    preview.setAttribute('playsinline', '');
    preview.style.cssText =
        'display:block;width:640px;max-width:80vw;max-height:70vh;';
    final capture = web.HTMLButtonElement()
      ..textContent = video ? 'Record' : 'Take photo';
    capture.disabled = true;
    final cancel = web.HTMLButtonElement()..textContent = 'Cancel';
    panel.appendChild(preview);
    panel.appendChild(capture);
    panel.appendChild(cancel);
    void stopTracks(web.MediaStream value) {
      for (final track in value.getTracks().toDart) {
        track.stop();
      }
    }

    void fail(Object error, [StackTrace? stack]) {
      if (!done.isCompleted)
        done.completeError(
          error is PlatformException
              ? error
              : PlatformException(
                  code: 'camera_capture_failed',
                  message: error.toString(),
                ),
          stack,
        );
    }

    void cancelCapture() {
      if (!done.isCompleted) done.complete(null);
    }

    cancel.onclick = ((web.Event _) => cancelCapture()).toJS;
    panel.oncancel = ((web.Event event) {
      event.preventDefault();
      cancelCapture();
    }).toJS;
    panel.onclose = ((web.Event _) => cancelCapture()).toJS;
    final pageHide = ((web.Event _) => cancelCapture()).toJS;
    web.window.addEventListener('pagehide', pageHide);
    try {
      web.document.body!.appendChild(panel);
      panel.showModal();
      // Do not block cancellation while the browser permission prompt is pending.
      unawaited(() async {
        try {
          final media = await web.window.navigator.mediaDevices
              .getUserMedia(
                web.MediaStreamConstraints(
                  video: {
                    'facingMode': {
                      'ideal': device == CameraDevice.front
                          ? 'user'
                          : 'environment',
                    },
                  }.jsify()!,
                  audio: video.toJS,
                ),
              )
              .toDart;
          if (done.isCompleted) {
            stopTracks(media);
            return;
          }
          stream = media;
          preview.srcObject = media;
          await preview.play().toDart;
          if (!done.isCompleted) capture.disabled = false;
        } catch (error, stack) {
          fail(error, stack);
        }
      }());
      capture.onclick = ((web.Event _) {
        if (done.isCompleted || stream == null) return;
        if (video) {
          if (recorder != null) {
            capture.disabled = true;
            if (recorder!.state == 'recording') recorder!.stop();
            return;
          }
          try {
            final types = [
              'video/webm;codecs=vp8,opus',
              'video/webm',
              'video/mp4',
            ];
            final type = types.firstWhere(
              (t) => web.MediaRecorder.isTypeSupported(t),
              orElse: () => '',
            );
            final recording = type.isEmpty
                ? web.MediaRecorder(stream!)
                : web.MediaRecorder(
                    stream!,
                    web.MediaRecorderOptions(mimeType: type),
                  );
            recorder = recording;
            recording.ondataavailable = ((web.BlobEvent event) {
              if (event.data.size > 0) chunks.add(event.data);
            }).toJS;
            recording.onerror = ((web.Event event) => fail(
              PlatformException(
                code: 'video_recording_failed',
                message: 'Browser recording failed',
              ),
            )).toJS;
            recording.onstop = ((web.Event _) {
              if (done.isCompleted) return;
              unawaited(() async {
                try {
                  final mime = recording.mimeType.isEmpty
                      ? type
                      : recording.mimeType;
                  final blob = web.Blob(
                    chunks.toJS,
                    web.BlobPropertyBag(type: mime),
                  );
                  if (blob.size == 0) throw StateError('Recording is empty');
                  final bytes = (await blob.arrayBuffer().toDart).toDart
                      .asUint8List();
                  if (!done.isCompleted)
                    done.complete(
                      XFile.fromData(
                        bytes,
                        mimeType: mime,
                        name: mime.contains('mp4')
                            ? 'camera.mp4'
                            : 'camera.webm',
                      ),
                    );
                } catch (error, stack) {
                  fail(error, stack);
                }
              }());
            }).toJS;
            recording.start();
            capture.textContent = 'Finish';
            if (maxDuration != null)
              timer = Timer(maxDuration, () {
                capture.disabled = true;
                if (recording.state == 'recording') recording.stop();
              });
          } catch (error, stack) {
            fail(error, stack);
          }
        } else {
          capture.disabled = true;
          try {
            final canvas = web.HTMLCanvasElement()
              ..width = preview.videoWidth
              ..height = preview.videoHeight;
            if (canvas.width == 0 || canvas.height == 0)
              throw StateError('Camera preview is not ready');
            canvas.context2D.drawImage(preview, 0, 0);
            canvas.toBlob(
              ((web.Blob? blob) {
                unawaited(() async {
                  try {
                    if (blob == null)
                      throw StateError('Could not encode photo');
                    final bytes = (await blob.arrayBuffer().toDart).toDart
                        .asUint8List();
                    if (!done.isCompleted)
                      done.complete(
                        XFile.fromData(
                          bytes,
                          mimeType: 'image/jpeg',
                          name: 'camera.jpg',
                        ),
                      );
                  } catch (error, stack) {
                    fail(error, stack);
                  }
                }());
              }).toJS,
              'image/jpeg',
              1.toJS,
            );
          } catch (error, stack) {
            fail(error, stack);
          }
        }
      }).toJS;
      return await done.future;
    } finally {
      timer?.cancel();
      if (recorder?.state == 'recording') recorder!.stop();
      if (stream != null) stopTracks(stream!);
      web.window.removeEventListener('pagehide', pageHide);
      panel.close();
      panel.remove();
      _active = false;
    }
  }
}
