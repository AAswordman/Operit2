import fs from '@ohos.file.fs';
import image from '@ohos.multimedia.image';
import common from '@ohos.app.ability.common';
import { FlutterEngine } from '@ohos/flutter_ohos';
import MethodCall from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodCall';
import MethodChannel, { MethodResult } from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodChannel';

interface PdfRenderRequest {
  input: string;
  outputDirectory: string;
  pages: number[] | null;
  scale: number;
}

interface PdfRenderResponse {
  ok: boolean;
  pages?: string[];
  error?: string;
}

/** Implements printing's raster protocol using the same offline terminal runtime as Core. */
export class OperitPdfPreview {
  private channel: MethodChannel;
  private context: common.UIAbilityContext;
  private render: (request: string) => Promise<string>;
  private jobs: Set<number> = new Set();
  private active: boolean = true;

  /** Registers PDF preview without adding platform branches to shared Flutter widgets. */
  constructor(engine: FlutterEngine, context: common.UIAbilityContext, render: (request: string) => Promise<string>) {
    this.context = context;
    this.render = render;
    this.channel = new MethodChannel(engine.getDartExecutor(), 'net.nfet.printing');
    this.channel.setMethodCallHandler({ onMethodCall: (call, result): void => this.handle(call, result) });
  }

  /** Starts asynchronous page rendering and exposes only implemented printing capabilities. */
  private handle(call: MethodCall, result: MethodResult): void {
    switch (call.method) {
      case 'printingInfo': {
        const capabilities = new Map<string, boolean>();
        capabilities.set('canRaster', true);
        capabilities.set('canPrint', false);
        capabilities.set('canShare', false);
        capabilities.set('directPrint', false);
        capabilities.set('dynamicLayout', false);
        capabilities.set('canConvertHtml', false);
        result.success(capabilities);
        return;
      }
      case 'rasterPdf': {
        const job = call.argument('job') as number;
        if (this.jobs.has(job)) { result.error('PDF_JOB_EXISTS', 'PDF job already exists', null); return; }
        this.jobs.add(job);
        result.success(null);
        this.raster(call, job).catch((error: Error): void => {
          console.error(`PDF cleanup failed: ${error.message}`);
        });
        return;
      }
      case 'cancelJob': this.jobs.delete(call.argument('job') as number); result.success(null); return;
      default: result.notImplemented();
    }
  }

  /** Publishes one printing callback while the Flutter engine still owns the channel. */
  private emit(method: string, payload: Map<string, Object>): void {
    if (this.active) this.channel.invokeMethod(method, payload);
  }

  /** Converts ordered PNG pages to exact RGBA buffers and always removes temporary files. */
  private async raster(call: MethodCall, job: number): Promise<void> {
    const directory = `${this.context.tempDir}/operit-pdf-${job}-${Date.now()}`;
    try {
      fs.mkdirSync(directory);
      const bytes = call.argument('doc') as Uint8Array;
      const input = `${directory}/source.pdf`;
      const file = fs.openSync(input, fs.OpenMode.CREATE | fs.OpenMode.TRUNC | fs.OpenMode.WRITE_ONLY);
      try {
        let offset = 0;
        while (offset < bytes.byteLength) {
          const count = fs.writeSync(file.fd, bytes.buffer.slice(bytes.byteOffset + offset, bytes.byteOffset + bytes.byteLength));
          if (count <= 0) throw new Error('Incomplete PDF source write');
          offset += count;
        }
      } finally { fs.closeSync(file); }
      const request: PdfRenderRequest = {
        input: input, outputDirectory: directory,
        pages: call.argument('pages') as number[] | null, scale: call.argument('scale') as number
      };
      const response = JSON.parse(await this.render(JSON.stringify(request))) as PdfRenderResponse;
      if (!response.ok || response.pages === undefined) throw new Error(`${response.error}`);
      for (const name of response.pages) {
        if (!this.active || !this.jobs.has(job)) break;
        const source = image.createImageSource(`${directory}/${name}`);
        try {
          const pixels = await source.createPixelMap({ desiredPixelFormat: image.PixelMapFormat.RGBA_8888 });
          try {
            const info = await pixels.getImageInfo();
            const buffer = new ArrayBuffer(pixels.getPixelBytesNumber());
            await pixels.readPixelsToBuffer(buffer);
            const payload = new Map<string, Object>();
            payload.set('job', job);
            payload.set('width', info.size.width);
            payload.set('height', info.size.height);
            payload.set('image', new Uint8Array(buffer));
            this.emit('onPageRasterized', payload);
          } finally { await pixels.release(); }
        } finally { source.release(); }
      }
      const completed = new Map<string, Object>();
      completed.set('job', job);
      this.emit('onPageRasterEnd', completed);
    } catch (error) {
      const failed = new Map<string, Object>();
      failed.set('job', job);
      failed.set('error', `${error}`);
      this.emit('onPageRasterEnd', failed);
    } finally {
      this.jobs.delete(job);
      if (fs.accessSync(directory)) fs.rmdirSync(directory);
    }
  }

  /** Stops callbacks and detaches the protocol when its owner is destroyed. */
  dispose(): void {
    this.active = false;
    this.jobs.clear();
    this.channel.setMethodCallHandler(null);
  }
}
