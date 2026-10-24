// Copyright (c) 2023 Hunan OpenValley Digital Industry Development Co., Ltd.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE_ODID file.
// Based on FileSelectorApiImpl.java originally written by
// Copyright 2013 The Flutter Authors.

import fs from '@ohos.file.fs';
import picker from '@ohos.file.picker';
import Log from '@ohos/flutter_ohos/src/main/ets/util/Log';
import { Result, FileTypes, FileResponse, FileSelectorApiCodec, wrapError } from './GeneratedFileSelectorApi'
import { BinaryMessenger } from '@ohos/flutter_ohos/src/main/ets/plugin/common/BinaryMessenger';
import MessageCodec from '@ohos/flutter_ohos/src/main/ets/plugin/common/MessageCodec';
import BasicMessageChannel, { Reply } from '@ohos/flutter_ohos/src/main/ets/plugin/common/BasicMessageChannel';
import MethodChannel, { MethodResult } from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodChannel';
import MethodCall from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodCall';
import { AbilityPluginBinding } from '@ohos/flutter_ohos/src/main/ets/embedding/engine/plugins/ability/AbilityPluginBinding';
import common from '@ohos.app.ability.common';
import { photoPickerSelect, documentPickerSelect, filePicker } from './FIleSelectorUtil'

const TAG = "FileSelectorApiImpl";
const DOCUMENT = "Document";
const TEXT_MIME_TYPE = "text/*";
const MIME_TYPE = "*/*";
const PICTURE = "Picture";
const TEXT = "Text";
export class FileSelectorApiImpl {

  binding: AbilityPluginBinding;
  private saveChannel: MethodChannel | null = null;
  private inputChannel: MethodChannel | null = null;
  private inputs: Map<string, DocumentInput> = new Map<string, DocumentInput>();
  private nextInputId: number = 0;
  private inputOperations: Promise<void> = Promise.resolve();
  private detached: boolean = false;

  constructor(binding: AbilityPluginBinding) {
    this.binding = binding;
  }

  openFileWithExtensions(initialDirectory: string, allowedTypes: FileTypes, result: Result<FileResponse | null>): void {
    Log.d(TAG, 'openFile types:' + allowedTypes.extensions + ' dir:' + initialDirectory)
    filePicker(initialDirectory, allowedTypes, 1).then(async (filePickerResult: ESObject) => {
      if (filePickerResult != undefined) {
        let fileUris = filePickerResult as Array<string>;
        if (fileUris.length == 0) {
          result.success(null);
          return;
        }
        let uri: string = fileUris[0];
        Log.i(TAG,'documentPickerSelect select file successfully, documentPicker uri: ' + uri);
        FileSelectorApiImpl.toFileResponse(uri, MIME_TYPE, uri.substring(uri.lastIndexOf("/") + 1))
          .then((file) => {
            result.success(file);
          }).catch((err: ESObject) => {
          Log.e(TAG, 'documentPickerSelect select file failed with err: ' + err);
          result.error(err);
        });
      } else {
        Log.e(TAG, 'documentPickerSelect select file failed with errCode: ' + filePickerResult);
        result.error(new Error("Failed to select file with errCode: " + filePickerResult))
      }
    });
  }

  openFilesWithExtensions(initialDirectory: string, allowedTypes: FileTypes, result: Result<Array<FileResponse>>): void {
    Log.d(TAG, 'openFiles types:' + allowedTypes.extensions + ' dir:' + initialDirectory)
    filePicker(initialDirectory, allowedTypes).then((filePickerResult: ESObject) => {
      if (filePickerResult != undefined) {
        Log.i(TAG,'documentPickerSelect select files successfully, documentPicker uris: ' + filePickerResult);
        FileSelectorApiImpl.toFileListResponse(filePickerResult, MIME_TYPE)
          .then((file) => {
            result.success(file);
          }).catch((err: ESObject) => {
          Log.e(TAG, 'documentPickerSelect select files failed with err: ' + err);
          result.error(err);
        });
      } else {
        Log.e(TAG, 'documentPickerSelect select files failed with errCode: ' + filePickerResult);
        result.error(new Error("Failed to select files with errCode: " + filePickerResult))
      }
    });
  }

  /// Opens the system save dialog and returns the selected writable document URI.
  async getSavePath(
    initialDirectory: string,
    suggestedName: string,
    allowedTypes: FileTypes,
    result: Result<ESObject>,
  ): Promise<void> {
    const options = new picker.DocumentSaveOptions();
    options.defaultFilePathUri = initialDirectory;
    options.newFileNames = suggestedName == null ? [] : [suggestedName];
    options.fileSuffixChoices = allowedTypes
      .getExtensions()
      .map((extension: string): string => `.${extension}`);
    try {
      const pickerResult = await new picker.DocumentViewPicker().save(options);
      result.success(pickerResult.length == 0 ? null : pickerResult[0]);
    } catch (error) {
      result.error(error as Error);
    }
  }

  openFile(initialDirectory: string, allowedTypes: FileTypes, result: Result<FileResponse | null>): void {
    if (allowedTypes.getMimeTypes().length == 0) {
      allowedTypes.setMimeTypes(tryConvertExtensionsToMimetypes(allowedTypes.getExtensions()));
    }
    Log.d(TAG, 'openFile types:' + allowedTypes.mimeTypes + ' dir:' + initialDirectory)
    for (let type of allowedTypes.getMimeTypes()) {
      Log.d(TAG, 'openFile type:' + type)
      switch (type) {
        case picker.PhotoViewMIMETypes.IMAGE_TYPE:
        case picker.PhotoViewMIMETypes.VIDEO_TYPE:
        case picker.PhotoViewMIMETypes.IMAGE_VIDEO_TYPE:
          try {
            let PhotoSelectOptions = new picker.PhotoSelectOptions();
            PhotoSelectOptions.MIMEType = type;
            PhotoSelectOptions.maxSelectNumber = 1;
            photoPickerSelect(PhotoSelectOptions, this.getContext()).then((PhotoSelectResult: picker.PhotoSelectResult) => {
              console.info('PhotoViewPicker.select successfully, PhotoSelectResult uri: ' + JSON.stringify(PhotoSelectResult));
              let res: picker.PhotoSelectResult = PhotoSelectResult;
              FileSelectorApiImpl.toFileResponse(res.photoUris[0] , type, PICTURE)
                .then((file) => {
                  result.success(file);
              }).catch((err: ESObject) => {
                  Log.e(TAG, 'PhotoViewPicker.select failed with err: ' + err);
                  result.error(err);
              });
            }).catch((err: ESObject) => {
              Log.e(TAG, 'PhotoViewPicker.select failed with err: ' + err);
              result.error(new Error("Failed to read file, err: " + err));
            });
          } catch (err) {
            console.error('PhotoViewPicker failed with err: ' + err);
            result.error(new Error("Failed to read file: " + initialDirectory));
          }
          break;
        case DOCUMENT:
          try {
            let documentSelectOptions = new picker.DocumentSelectOptions();
            documentSelectOptions.maxSelectNumber = 1;
            documentPickerSelect(documentSelectOptions, this.getContext()).then((documentPickerResult: ESObject) => {
              if (Number.isNaN(Number(documentPickerResult))) {
                let uri: string = documentPickerResult[0];
                Log.i(TAG,'documentPickerSelect select successfully, documentPicker uri: ' + uri);
                FileSelectorApiImpl.toFileResponse(uri, TEXT_MIME_TYPE, TEXT)
                  .then((file) => {
                    result.success(file);
                  }).catch((err: ESObject) => {
                  Log.e(TAG, 'documentPickerSelect select failed with err: ' + err);
                  result.error(err);
                });
              } else {
                Log.e(TAG, 'documentPickerSelect select failed with errCode: ' + documentPickerResult);
                result.error(new Error("Failed to select file with errCode: " + documentPickerResult))
              }
            });
          } catch (err) {
            Log.e(TAG, 'documentPickerSelect select failed with err: ' + err);
            result.error(new Error("Failed to read file: " + initialDirectory));
          }
          break;
        default :
          break;
      }
    }
  }

  getContext(): common.UIAbilityContext {
    return this.binding.getAbility().context;
  }

  openFiles(initialDirectory: string, allowedTypes: FileTypes, result: Result<Array<FileResponse>>): void {
    if (allowedTypes.getMimeTypes().length == 0) {
      allowedTypes.setMimeTypes(tryConvertExtensionsToMimetypes(allowedTypes.getExtensions()));
    }
    for (let type of allowedTypes.getMimeTypes()) {
      Log.d(TAG, 'openFiles type:' + type)
      switch (type) {
        case picker.PhotoViewMIMETypes.IMAGE_TYPE:
        case picker.PhotoViewMIMETypes.VIDEO_TYPE:
        case picker.PhotoViewMIMETypes.IMAGE_VIDEO_TYPE:
          try {
            let PhotoSelectOptions = new picker.PhotoSelectOptions();
            PhotoSelectOptions.MIMEType = type;
            PhotoSelectOptions.maxSelectNumber = 5;
            photoPickerSelect(PhotoSelectOptions, this.getContext()).then((PhotoSelectResult: picker.PhotoSelectResult) => {
              Log.i(TAG, 'PhotoViewPicker.select successfully, PhotoSelectResult uri: ' + JSON.stringify(PhotoSelectResult));
              const res: picker.PhotoSelectResult = PhotoSelectResult; //<picker.PhotoSelectResult>(PhotoSelectResult);
              FileSelectorApiImpl.toFileListResponse(res.photoUris, type)
                .then((files) => {
                  Log.i(TAG, 'PhotoViewPicker.select successfully, uri: ' + JSON.stringify(PhotoSelectResult));
                  result.success(files);
                }).catch((err: ESObject) => {
                  Log.e(TAG, 'PhotoViewPicker.select failed with err: ' + err);
                  result.error(err);
              });
            }).catch((err: ESObject) => {
              Log.e(TAG, 'PhotoViewPicker.select failed with err: ' + err);
              result.error(new Error("Failed to read file, err: " + err));
            });
          } catch (err) {
            Log.e(TAG, 'PhotoViewPicker failed with err: ' + err);
            result.error(new Error("Failed to read file: " + initialDirectory));
          }
          break;
        case DOCUMENT:
          try {
            let documentSelectOptions = new picker.DocumentSelectOptions();
            documentSelectOptions.maxSelectNumber = 5;
            documentPickerSelect(documentSelectOptions, this.getContext()).then((documentPickerResult: ESObject) => {
              if (Number.isNaN(Number(documentPickerResult))) {
                let pickerResult = documentPickerResult as Array<string>;
                Log.i(TAG,'documentPickerSelect select successfully, documentPicker uris: ' + pickerResult.toString());
                FileSelectorApiImpl.toFileListResponse(pickerResult, TEXT_MIME_TYPE)
                  .then((files) => {
                    result.success(files);
                  }).catch((err: ESObject) => {
                  Log.e(TAG, 'documentPickerSelect select failed with err: ' + err);
                  result.error(err);
                });
              } else {
                Log.e(TAG, 'documentPickerSelect select failed with errCode: ' + documentPickerResult);
                result.error(new Error("Failed to select file with errCode: " + documentPickerResult))
              }
            });
          } catch (err) {
            Log.e(TAG, 'documentPickerSelect select failed with err: ' + err);
            result.error(new Error("Failed to read file: " + initialDirectory));
          }
          break;
        default :
          break;
      }
    }
  }

  getDirectoryPath(initialDirectory: string, result: Result<string>): void {
    throw new Error('Method not implemented.')
  }

  /// Releases both channels and every input descriptor when registration is detached.
  closeChannels(): void {
    this.saveChannel?.setMethodCallHandler(null);
    this.saveChannel = null;
    this.detached = true;
    this.inputChannel?.setMethodCallHandler(null);
    this.inputChannel = null;
    // Drain any read already in flight before closing its descriptor.
    this.inputOperations = this.inputOperations.then(async (): Promise<void> => {
      for (const input of this.inputs.values()) {
        try {
          await fs.close(input.file);
        } catch (error) {
          Log.e(TAG, 'Failed to close document input: ' + String(error));
        }
      }
      this.inputs.clear();
    });
  }

  /// Performs a native file copy with opened document handles and deterministic resource release.
  private async copyFileReference(call: MethodCall, result: MethodResult): Promise<void> {
    const argumentsMap = call.args as Map<string, string>;
    const sourcePath = argumentsMap.get('sourcePath');
    const destinationUri = argumentsMap.get('destinationUri');
    if (typeof sourcePath != 'string' || sourcePath.length == 0 ||
        typeof destinationUri != 'string' || destinationUri.length == 0) {
      result.error('INVALID_SAVE_ARGS', 'sourcePath and destinationUri are required', null);
      return;
    }
    try {
      const source = await fs.open(sourcePath, fs.OpenMode.READ_ONLY);
      try {
        const destination = await fs.open(destinationUri, fs.OpenMode.READ_WRITE | fs.OpenMode.CREATE | fs.OpenMode.TRUNC);
        try {
          if (call.method == 'copyFileFromPathWithProgress') {
            const total = (await fs.stat(source.fd)).size;
            let completed = 0;
            let lastReport = 0;
            this.reportSaveProgress(completed, total);
            const buffer = new ArrayBuffer(65536);
            while (completed < total) {
              const count = Math.min(buffer.byteLength, total - completed);
              const read = await fs.read(source.fd, buffer, { offset: completed, length: count });
              if (read != count) throw new Error('Truncated document copy source');
              const written = await fs.write(destination.fd, buffer, { offset: completed, length: count });
              if (written != count) throw new Error('Incomplete document destination write');
              completed += written;
              const now = Date.now();
              if (now - lastReport >= 50) { this.reportSaveProgress(completed, total); lastReport = now; }
            }
            await fs.fsync(destination.fd);
            this.reportSaveProgress(completed, total);
          } else {
            await fs.copyFile(source.fd, destination.fd);
          }
        } finally {
          await fs.close(destination);
        }
      } finally {
        await fs.close(source);
      }
      result.success(null);
    } catch (error) {
      result.error('SAVE_WRITE_FAILED', String(error), null);
    }
  }

  /// Publishes measured URI copy counters through the registered platform channel.
  private reportSaveProgress(completed: number, total: number): void {
    if (this.saveChannel == null) throw new Error('Document save channel is detached');
    const progress = new Map<string, Object>();
    progress.set('completed', completed);
    progress.set('total', total);
    this.saveChannel.invokeMethod('saveProgress', progress);
  }

  /// Every allocation is bounded before bytes cross the Flutter channel.
  private async handleInput(call: MethodCall, result: MethodResult): Promise<void> {
    try {
      if (this.detached) {
        throw new Error('Document input has been detached');
      }
      const args = call.args as Map<string, Object>;
      if (call.method == 'open') {
        const path = args.get('path') as string;
        const start = args.get('start') as number;
        if (typeof path != 'string' || !Number.isSafeInteger(start) || start < 0) {
          throw new Error('Invalid document input metadata');
        }
        const file = await fs.open(path, fs.OpenMode.READ_ONLY);
        const token = String(++this.nextInputId);
        this.inputs.set(token, new DocumentInput(file, start));
        result.success(token);
        return;
      }
      const token = args.get('token') as string;
      if (call.method == 'close') {
        const input = this.inputs.get(token);
        this.inputs.delete(token);
        if (input != undefined) await fs.close(input.file);
        result.success(null);
        return;
      }
      if (call.method == 'readChunk') {
        const input = this.inputs.get(token);
        const maxBytes = args.get('maxBytes') as number;
        if (input == undefined || !Number.isSafeInteger(maxBytes) ||
          maxBytes <= 0 || maxBytes > 1024 * 1024) {
          throw new Error('Invalid document input read');
        }
        const buffer = new ArrayBuffer(maxBytes);
        const count = await fs.read(input.file.fd, buffer, {
          offset: input.offset,
          length: maxBytes,
        });
        input.offset += count;
        result.success(new Uint8Array(buffer, 0, count));
        return;
      }
      result.notImplemented();
    } catch (error) {
      result.error('DOCUMENT_INPUT_FAILED', String(error), null);
    }
  }

  static getCodec(): MessageCodec<ESObject> {
    return FileSelectorApiCodec.INSTANCE;
  }

  /// Installs the native file-backed save operation beside the generated file selector channels.
  setup(binaryMessenger: BinaryMessenger, abilityPluginBinding: AbilityPluginBinding): void {
    this.detached = false;
    this.inputChannel = new MethodChannel(binaryMessenger, 'dev.flutter.packages.file_selector_ohos/input');
    this.inputChannel.setMethodCallHandler({
      onMethodCall: (call: MethodCall, result: MethodResult): void => {
        this.inputOperations = this.inputOperations.then((): Promise<void> => this.handleInput(call, result));
      }
    });
    this.saveChannel = new MethodChannel(binaryMessenger, 'dev.flutter.packages.file_selector_ohos/save');
    this.saveChannel.setMethodCallHandler({
      /// Handles explicit source-file copies into selected document URIs.
      onMethodCall: (call: MethodCall, result: MethodResult): void => {
        if (call.method != 'copyFileFromPath' && call.method != 'copyFileFromPathWithProgress') {
          result.notImplemented();
          return;
        }
        this.copyFileReference(call, result);
      }
    });
    let api = this;
    {
      this.binding = abilityPluginBinding;
      const channel: BasicMessageChannel<ESObject> = new BasicMessageChannel<ESObject>(
        binaryMessenger, "dev.flutter.pigeon.FileSelectorApi.openFile", FileSelectorApiImpl.getCodec());
      channel.setMessageHandler({
        onMessage(msg: ESObject, reply: Reply<ESObject>): void {
          Log.d(TAG, 'onMessage reply:' + reply)
          const wrapped: Array<ESObject> = new Array<ESObject>();
          const args: Array<ESObject> = msg as Array<ESObject>;
          const initialDirectoryArg = args[0] as string;
          const allowedTypesArg = args[1] as FileTypes;
          const resultCallback: Result<FileResponse | null> = new ResultBuilder((result: FileResponse | null): void => {
            wrapped.push(result);
            reply.reply(wrapped);
          },(error: Error): void => {
            const wrappedError: Array<ESObject> = wrapError(error);
            reply.reply(wrappedError);
          })
          api.openFileWithExtensions(initialDirectoryArg, allowedTypesArg, resultCallback);
        }
      });
    }
    {
      this.binding = abilityPluginBinding;
      const channel: BasicMessageChannel<ESObject>  = new BasicMessageChannel<ESObject>(
        binaryMessenger, "dev.flutter.pigeon.FileSelectorApi.openFiles", FileSelectorApiImpl.getCodec());
      channel.setMessageHandler({
        onMessage(msg: ESObject, reply: Reply<ESObject>): void {
          Log.d(TAG, 'onMessage reply:' + reply)
          const wrapped: Array<ESObject> = new Array<ESObject>();
          const args: Array<ESObject> = msg as Array<ESObject>;
          const initialDirectoryArg = args[0] as string;
          const allowedTypesArg = args[1] as FileTypes;
          const resultCallback: Result<Array<FileResponse>> = new ResultsBuilder((result: Array<FileResponse>): void => {
            wrapped.push(result);
            reply.reply(wrapped);
          },(error: Error): void => {
            const wrappedError: Array<ESObject> = wrapError(error);
            reply.reply(wrappedError);
          })
          api.openFilesWithExtensions(initialDirectoryArg, allowedTypesArg, resultCallback);
        }
      });
    }
    {
      this.binding = abilityPluginBinding;
      const channel: BasicMessageChannel<ESObject> = new BasicMessageChannel<ESObject>(
        binaryMessenger,
        "dev.flutter.pigeon.FileSelectorApi.getSavePath",
        FileSelectorApiImpl.getCodec(),
      );
      channel.setMessageHandler({
        onMessage(msg: ESObject, reply: Reply<ESObject>): void {
          const wrapped: Array<ESObject> = new Array<ESObject>();
          const args: Array<ESObject> = msg as Array<ESObject>;
          const initialDirectoryArg = args[0] as string;
          const suggestedNameArg = args[1] as string;
          const allowedTypesArg = args[2] as FileTypes;
          const resultCallback: Result<ESObject> = new ResultBuilder(
            (result: ESObject): void => {
              wrapped.push(result);
              reply.reply(wrapped);
            },
            (error: Error): void => {
              reply.reply(wrapError(error));
            },
          );
          api.getSavePath(
            initialDirectoryArg,
            suggestedNameArg,
            allowedTypesArg,
            resultCallback,
          );
        },
      });
    }
  }

  static async toFileResponse(uri: string, type: string, name: string): Promise<FileResponse> {
    const file = await fs.open(uri, fs.OpenMode.READ_ONLY);
    try {
      const stat = await fs.stat(file.fd);
      return new FileResponse(uri, type, name, stat.size);
    } finally {
      await fs.close(file);
    }
  }

  static async toFileListResponse(uris: string[], type: string): Promise<Array<FileResponse>> {
    const files = new Array<FileResponse>();
    for (const uri of uris) {
      files.push(await FileSelectorApiImpl.toFileResponse(
        uri, type, uri.substring(uri.lastIndexOf('/') + 1)));
    }
    return files;
  }
}

/// Descriptor and cursor only; never owns a complete file buffer.
class DocumentInput {
  file: fs.File;
  offset: number;

  constructor(file: fs.File, offset: number) {
    this.file = file;
    this.offset = offset;
  }
}

function tryConvertExtensionsToMimetypes(list: Array<string>): Array<string> | null {
  if (list == undefined || list.length == 0) {
    Log.w(TAG, 'list is empty')
    return null;
  }
  let mimeTypes = new Array<string>()
  for (let str of list) {
    switch (str) {
      case 'png':
      case 'jpg':
      case 'jpeg':
        if (mimeTypes.indexOf(picker.PhotoViewMIMETypes.IMAGE_TYPE) == -1) {
          mimeTypes.push(picker.PhotoViewMIMETypes.IMAGE_TYPE)
        }
        break;
      case 'txt':
      case 'json':
      default :
        if (mimeTypes.indexOf(DOCUMENT) == -1) {
          mimeTypes.push(DOCUMENT);
        }
        break;
    }
  }
  return mimeTypes;
}

class ResultBuilder<T> {
  success : (result: T)=>void
  error: (error: Error) =>void

  constructor(success: (result: T) => void, error: (error: Error) => void) {
    this.success = success
    this.error = error
  }
}
class ResultsBuilder{
  success : (result: Array<FileResponse>)=>void
  error: (error: Error) =>void

  constructor(success:ESObject , error:ESObject) {
    this.success = success
    this.error = error
  }
}
