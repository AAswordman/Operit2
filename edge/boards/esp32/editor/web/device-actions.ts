import type {RuntimeModule} from './types.js';

export interface SpaceReviewContext {
  requestId?: string;
  assignmentVersion?: number;
}

export interface DeviceActionRequest extends SpaceReviewContext {
  action: string;
}

/** Match firmware actions: a new attempt clears the old bounded diagnostic;
 * a failed RPC displays its real error without fabricating a device state.
 * Reviews use the identity captured with the displayed prompt, never a new
 * pending request discovered after the user clicked. */
export async function performDeviceAction(
  ui: Pick<RuntimeModule, 'ccall'> | null,
  action: string,
  invoke: (request: DeviceActionRequest) => Promise<unknown>,
  review?: SpaceReviewContext,
): Promise<void> {
  ui?.ccall('operit_ui_action_error', null, ['string'], ['']);
  try {
    const request: DeviceActionRequest = {action};
    if (action === 'edge_space_approve' || action === 'edge_space_reject') {
      if (!review?.requestId?.trim() || typeof review.assignmentVersion !== 'number' ||
          !Number.isSafeInteger(review.assignmentVersion) || review.assignmentVersion < 0) {
        throw new Error('缺少屏幕上的设备空间申请，请刷新后重试');
      }
      request.requestId = review.requestId;
      request.assignmentVersion = review.assignmentVersion;
    }
    await invoke(request);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    ui?.ccall('operit_ui_action_error', null, ['string'], [message]);
    throw error;
  }
}
