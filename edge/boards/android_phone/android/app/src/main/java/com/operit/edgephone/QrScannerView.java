package com.operit.edgephone;

import android.content.Context;
import android.graphics.Matrix;
import android.graphics.SurfaceTexture;
import android.hardware.Camera;
import android.view.TextureView;
import com.google.zxing.BinaryBitmap;
import com.google.zxing.PlanarYUVLuminanceSource;
import com.google.zxing.ReaderException;
import com.google.zxing.common.HybridBinarizer;
import com.google.zxing.qrcode.QRCodeReader;
import java.io.IOException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** Uses one camera frame at a time while the Space scanner is visible. */
@SuppressWarnings("deprecation")
final class QrScannerView extends TextureView implements TextureView.SurfaceTextureListener {
    interface Listener {
        /** Reports one decoded QR payload for validation. */
        void scanned(String code);

        /** Reports an unavailable camera session to the scan page. */
        void unavailable();
    }

    private final ExecutorService decoder = Executors.newSingleThreadExecutor();
    private final Listener listener;
    private Camera camera;
    private int orientation;
    private boolean active;

    /** Creates the camera preview without opening a camera before the page is visible. */
    QrScannerView(Context context, Listener listener) {
        super(context);
        this.listener = listener;
        setContentDescription("设备空间二维码扫描预览");
        setSurfaceTextureListener(this);
    }

    /** Starts and releases the camera with the Activity's resumed state. */
    void setScanning(boolean active) {
        this.active = active;
        if (active && isAvailable()) open(getSurfaceTexture());
        if (!active) closeCamera();
    }

    /** Starts preview when its surface becomes available. */
    @Override public void onSurfaceTextureAvailable(SurfaceTexture surface, int width, int height) { open(surface); }

    /** Keeps the preview undistorted across round and rectangular displays. */
    @Override public void onSurfaceTextureSizeChanged(SurfaceTexture surface, int width, int height) {
        if (camera != null) resize(camera.getParameters().getPreviewSize());
    }

    /** Releases the camera before its preview surface is destroyed. */
    @Override public boolean onSurfaceTextureDestroyed(SurfaceTexture surface) {
        closeCamera();
        return true;
    }

    /** Preview frames are decoded through the throttled Camera callback. */
    @Override public void onSurfaceTextureUpdated(SurfaceTexture surface) {}

    /** Disposes the camera and decoder when the scanner page closes. */
    @Override protected void onDetachedFromWindow() {
        closeCamera();
        decoder.shutdownNow();
        super.onDetachedFromWindow();
    }

    /** Uses the device's primary camera at a bounded preview resolution. */
    private void open(SurfaceTexture surface) {
        if (!active || camera != null || decoder.isShutdown()) return;
        try {
            camera = Camera.open(0);
            Camera.CameraInfo info = new Camera.CameraInfo();
            Camera.getCameraInfo(0, info);
            int rotation = getDisplay().getRotation() * 90;
            orientation = info.facing == Camera.CameraInfo.CAMERA_FACING_FRONT
                ? (720 - info.orientation - rotation) % 360
                : (info.orientation - rotation + 360) % 360;
            Camera.Parameters params = camera.getParameters();
            Camera.Size size = params.getPreviewSize();
            for (Camera.Size candidate : params.getSupportedPreviewSizes()) {
                if (previewDistance(candidate) < previewDistance(size)) size = candidate;
            }
            params.setPreviewSize(size.width, size.height);
            if (params.getSupportedFocusModes().contains(Camera.Parameters.FOCUS_MODE_CONTINUOUS_PICTURE)) {
                params.setFocusMode(Camera.Parameters.FOCUS_MODE_CONTINUOUS_PICTURE);
            }
            camera.setParameters(params);
            camera.setDisplayOrientation(orientation);
            camera.setPreviewTexture(surface);
            camera.startPreview();
            resize(size);
            requestFrame(camera, size);
        } catch (IOException | RuntimeException error) {
            closeCamera();
            listener.unavailable();
        }
    }

    /** Measures a preview size against a 640 by 480 decoding target. */
    private int previewDistance(Camera.Size size) {
        return Math.abs(size.width * size.height - 640 * 480);
    }

    /** Center-crops the preview without changing the native decoding dimensions. */
    private void resize(Camera.Size size) {
        boolean portrait = orientation % 180 != 0;
        float previewRatio = portrait ? (float) size.height / size.width : (float) size.width / size.height;
        float viewRatio = (float) getWidth() / getHeight();
        Matrix transform = new Matrix();
        transform.setScale(Math.max(1, previewRatio / viewRatio), Math.max(1, viewRatio / previewRatio),
            getWidth() * .5f, getHeight() * .5f);
        setTransform(transform);
    }

    /** Decodes a single luminance frame off the UI thread. */
    private void requestFrame(Camera source, Camera.Size size) {
        source.setOneShotPreviewCallback((bytes, owner) -> {
            if (camera != owner) return;
            decoder.execute(() -> {
                String code = decode(bytes, size);
                post(() -> {
                    if (camera != owner) return;
                    if (code == null) requestFrame(owner, size);
                    else {
                        closeCamera();
                        listener.scanned(code);
                    }
                });
            });
        });
    }

    /** Reads QR only and treats frames without a readable symbol as camera input. */
    private String decode(byte[] bytes, Camera.Size size) {
        PlanarYUVLuminanceSource luminance = new PlanarYUVLuminanceSource(
            bytes, size.width, size.height, 0, 0, size.width, size.height, false);
        try {
            return new QRCodeReader().decode(new BinaryBitmap(new HybridBinarizer(luminance))).getText();
        } catch (ReaderException error) {
            return null;
        }
    }

    /** Invalidates pending frames before returning the camera to the OS. */
    private void closeCamera() {
        Camera previous = camera;
        camera = null;
        if (previous != null) previous.release();
    }
}
