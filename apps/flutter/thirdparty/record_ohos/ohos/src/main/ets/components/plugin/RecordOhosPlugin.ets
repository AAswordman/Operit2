/*
 * Copyright (C) 2025 Huawei Device Co., Ltd.
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
import {
  AbilityAware,
  AbilityPluginBinding,
  FlutterPlugin,
  FlutterPluginBinding,
  MethodChannel,
} from '@ohos/flutter_ohos';
import { MethodCallHandlerImpl } from './MethodCallHandlerImpl';
import PermissionManager from '../permission/PermissionManager';

export default class RecordOhosPlugin implements FlutterPlugin, AbilityAware {
  private readonly TAG: string = 'RecordOhosPlugin';
  private readonly MESSAGES_CHANNEL = 'com.llfbandit.record/messages';
  private channel: MethodChannel | null = null;
  private methodCallHandler: MethodCallHandlerImpl | null = null;
  private permissionManager: PermissionManager;
  private abilityBinding: AbilityPluginBinding | null = null;

  /** Creates the plugin-owned microphone permission manager. */
  constructor() {
    this.permissionManager = new PermissionManager();
  }

  /** Returns the stable Flutter plugin class identifier. */
  getUniqueClassName(): string {
    return "RecordOhosPlugin"
  }

  /** Registers the record method channel with the Flutter engine. */
  onAttachedToEngine(binding: FlutterPluginBinding): void {
    this.channel = new MethodChannel(binding.getBinaryMessenger(), this.MESSAGES_CHANNEL);
    this.methodCallHandler = new MethodCallHandlerImpl(binding.getBinaryMessenger(), this.permissionManager);
    this.channel.setMethodCallHandler(this.methodCallHandler);
  }

  /** Releases recorders and unregisters the method channel. */
  async onDetachedFromEngine(binding: FlutterPluginBinding): Promise<void> {
    console.info(`${this.TAG}: Detaching from engine, cleaning up resources...`);

    if (this.methodCallHandler != null) {
      await this.methodCallHandler.dispose()
      this.methodCallHandler = null;
    }

    if (this.channel != null) {
      this.channel.setMethodCallHandler(null);
      this.channel = null;
    }
  }

  /** Supplies the foreground ability used for permission requests. */
  onAttachedToAbility(binding: AbilityPluginBinding): void {
    this.abilityBinding = binding;
    this.permissionManager.setUIAbility(this.abilityBinding.getAbility());
  }

  /** Clears the foreground ability when the plugin detaches. */
  onDetachedFromAbility(): void {
    console.info(`${this.TAG}: Detaching from ability, performing final cleanup...`);
    this.permissionManager.setUIAbility(null);
  }
}
