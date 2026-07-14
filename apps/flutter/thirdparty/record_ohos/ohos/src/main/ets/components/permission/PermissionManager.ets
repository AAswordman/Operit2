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

import bundleManager from '@ohos.bundle.bundleManager';
import abilityAccessCtrl, { Permissions } from '@ohos.abilityAccessCtrl';
import { UIAbility } from '@kit.AbilityKit';
import { MethodResult } from '@ohos/flutter_ohos';

const TAG = 'PermissionManager'

export default class PermissionManager {
  private ability: UIAbility | null = null;
  private atManager: abilityAccessCtrl.AtManager;
  private permissions: Array<Permissions> = ['ohos.permission.MICROPHONE'];

  /** Creates the system access-token manager. */
  constructor() {
    this.atManager = abilityAccessCtrl.createAtManager();
  }

  /** Sets the foreground ability used by permission prompts. */
  setUIAbility(ability: UIAbility | null) {
    this.ability = ability;
  }

  /** Checks the microphone permission and requests an explicit decision. */
  async checkPermission(): Promise<boolean> {
    if (this.ability == null) {
      return false;
    }

    try {
      if (this.isPermissionGranted() === abilityAccessCtrl.GrantStatus.PERMISSION_GRANTED) {
        return true;
      }

      return await this.requestNecessaryPermissions(this.permissions);
    } catch (err) {
      const message = err instanceof Error ? err.message : `${err}`;
      console.error(TAG + ` checkPermission failed: ${message}`);
      throw new Error(`checkPermission failed: ${message}`);
    }
  }

  /** Requests the declared microphone permission and returns its exact result. */
  async requestNecessaryPermissions(permissions: Array<Permissions>): Promise<boolean> {
    const data = await this.atManager.requestPermissionsFromUser(this.ability!.context, permissions);
    for (let index = 0; index < data.permissions.length; index++) {
      if (data.permissions[index] == permissions[0]) {
        return data.authResults[index] === abilityAccessCtrl.GrantStatus.PERMISSION_GRANTED;
      }
    }
    throw new Error('The microphone permission result is missing.');
  }

  /** Returns the current microphone permission and optionally requests it. */
  hasPermission(result: MethodResult, request: boolean) {
    if (this.ability == null) {
      result.success(false);
      return;
    }
    try {
      const granted = this.isPermissionGranted() === abilityAccessCtrl.GrantStatus.PERMISSION_GRANTED;
      if (granted || !request) {
        result.success(granted);
        return;
      }
      this.requestNecessaryPermissions(this.permissions).then((permissionGranted: boolean) => {
        result.success(permissionGranted);
      }).catch((error: Error) => {
        result.error(TAG + ' permission', error.message, null);
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : `${error}`;
      result.error(TAG + ' permission', message, null);
    }
  }

  /** Returns the exact current microphone grant status. */
  isPermissionGranted(): abilityAccessCtrl.GrantStatus {
    const bundleInfo = bundleManager.getBundleInfoForSelfSync(
      bundleManager.BundleFlag.GET_BUNDLE_INFO_WITH_APPLICATION,
    );
    return this.atManager.checkAccessTokenSync(bundleInfo.appInfo.accessTokenId, this.permissions[0]);
  }
}
