import common from '@ohos.app.ability.common';
import fs from '@ohos.file.fs';
import webview from '@ohos.web.webview';
import { BinaryMessenger } from '@ohos/flutter_ohos/src/main/ets/plugin/common/BinaryMessenger';
import MethodChannel, { MethodCallHandler, MethodResult } from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodChannel';
import MethodCall from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodCall';
import { InstanceManager } from './InstanceManager';
import { WebViewPlatformView } from './WebViewPlatformView';

/** Retains the original ArkWeb operation and its private staging destination. */
class DownloadRecord {
  item: webview.WebDownloadItem;
  readonly identifier: number;
  readonly directory: string;
  readonly path: string;
  readonly name: string;
  completed: boolean = false;
  cancelled: boolean = false;
  lastProgress: number = 0;

  /** Assigns a unique directory while preventing server filenames escaping it. */
  constructor(item: webview.WebDownloadItem, identifier: number, cache: string) {
    this.item = item;
    this.identifier = identifier;
    this.directory = `${cache}/operit-browser-${item.getGuid()}`;
    const name = item.getSuggestedFileName().replace(/[\x00-\x1f\\/:*?"<>|]/g, '_').replace(/^[. ]+|[. ]+$/g, '');
    this.name = name.length === 0 ? 'download' : name;
    this.path = `${this.directory}/${this.name}`;
  }
}

/** Routes original ArkWeb responses and native progress to their Flutter owner. */
export class WebViewDownloads implements MethodCallHandler {
  private channel: MethodChannel;
  private instanceManager: InstanceManager;
  private cache: string;
  private records: Map<string, DownloadRecord> = new Map();

  /** Registers one engine-scoped command and event channel. */
  constructor(messenger: BinaryMessenger, manager: InstanceManager, context: common.Context) {
    this.instanceManager = manager;
    this.cache = context.cacheDir;
    this.channel = new MethodChannel(messenger, 'operit/webview_downloads');
    this.channel.setMethodCallHandler(this);
  }

  /** Handles exact transfer operations without recreating network requests. */
  onMethodCall(call: MethodCall, result: MethodResult): void {
    try {
      if (call.method === 'configure') {
        this.configure(call.argument('identifier') as number);
        result.success(null);
        return;
      }
      if (call.method === 'dispose') {
        this.closeView(call.argument('identifier') as number);
        result.success(null);
        return;
      }
      const id = call.argument('id') as string;
      const record = this.records.get(id);
      if (call.method === 'cancel' && record === undefined) {
        result.success(null);
        return;
      }
      if (record === undefined) throw new Error('The engine download has ended');
      if (call.method === 'cancel') {
        record.cancelled = true;
        record.item.cancel();
        this.publish(id, record, 'cancelled');
        this.release(id);
        result.success(null);
      } else if (call.method === 'read') {
        if (!record.completed) throw new Error('The engine file is not complete');
        const file = fs.openSync(record.path, fs.OpenMode.READ_ONLY);
        try {
          const size = fs.statSync(file.fd).size;
          const buffer = new ArrayBuffer(size);
          if (fs.readSync(file.fd, buffer) !== size) throw new Error('Incomplete engine file read');
          result.success(new Uint8Array(buffer));
        } finally {
          fs.closeSync(file);
          this.release(id);
        }
      } else {
        result.notImplemented();
      }
    } catch (error) {
      result.error('browser_download_failed', String(error), null);
    }
  }

  /** Installs the per-view delegate before the next navigation begins. */
  private configure(identifier: number): void {
    const view = this.instanceManager.getInstance<WebViewPlatformView>(identifier);
    if (view === undefined || view === null) throw new Error('Unknown WebView');
    const delegate = new webview.WebDownloadDelegate();
    delegate.onBeforeDownload((item: webview.WebDownloadItem): void => {
      const id = item.getGuid();
      const record = new DownloadRecord(item, identifier, this.cache);
      this.records.set(id, record);
      try {
        fs.mkdirSync(record.directory);
        this.publish(id, record, 'running');
        item.start(record.path);
      } catch (error) {
        this.publish(id, record, 'failed', String(error));
        item.cancel();
        this.release(id);
      }
    });
    delegate.onDownloadUpdated((item: webview.WebDownloadItem): void => {
      const record = this.records.get(item.getGuid());
      if (record === undefined || record.completed) return;
      record.item = item;
      const now = Date.now();
      if (now - record.lastProgress < 100) return;
      record.lastProgress = now;
      this.publish(item.getGuid(), record, 'running');
    });
    delegate.onDownloadFailed((item: webview.WebDownloadItem): void => {
      const record = this.records.get(item.getGuid());
      if (record === undefined) return;
      record.item = item;
      this.publish(item.getGuid(), record, record.cancelled ? 'cancelled' : 'failed', 'ArkWeb download interrupted');
      this.release(item.getGuid());
    });
    delegate.onDownloadFinish((item: webview.WebDownloadItem): void => {
      const record = this.records.get(item.getGuid());
      if (record === undefined) return;
      record.item = item;
      record.completed = true;
      this.publish(item.getGuid(), record, 'completed');
    });
    view.setNativeDownloadDelegate(delegate, (): void => this.closeView(identifier));
  }

  /** Emits byte counts and file metadata from the same original engine operation. */
  private publish(id: string, record: DownloadRecord, state: string, error: string = ''): void {
    const event = new Map<string, ESObject>()
      .set('identifier', record.identifier).set('id', id)
      .set('url', record.item.getUrl()).set('fileName', record.name)
      .set('state', state).set('receivedBytes', record.item.getReceivedBytes())
      .set('totalBytes', record.item.getTotalBytes()).set('error', error);
    this.channel.invokeMethod('download', event);
  }

  /** Removes only the staging files retained in this engine's transfer table. */
  private release(id: string): void {
    const record = this.records.get(id);
    if (record === undefined) return;
    this.records.delete(id);
    if (fs.accessSync(record.path)) fs.unlinkSync(record.path);
    if (fs.accessSync(record.directory)) fs.rmdirSync(record.directory);
  }

  /** Stops transfers belonging to a view before its controller is destroyed. */
  private closeView(identifier: number): void {
    const ids: Array<string> = [];
    this.records.forEach((record: DownloadRecord, id: string): void => {
      if (record.identifier === identifier) ids.push(id);
    });
    ids.forEach((id: string): void => {
      const record = this.records.get(id);
      if (record !== undefined && !record.completed) record.item.cancel();
      this.release(id);
    });
  }

  /** Releases all engine-owned operations before detaching the Flutter plugin. */
  dispose(): void {
    const owners = new Set<number>();
    this.records.forEach((record: DownloadRecord): void => { owners.add(record.identifier); });
    owners.forEach((identifier: number): void => this.closeView(identifier));
    this.channel.setMethodCallHandler(null);
  }
}
