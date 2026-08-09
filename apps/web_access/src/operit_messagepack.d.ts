/** Provides the MessagePack API used by the generated worker module. */
export interface OperitMessagePack {
  encode(value: unknown): Uint8Array;
  decode(bytes: Uint8Array): unknown;
}

export const MessagePack: OperitMessagePack;
