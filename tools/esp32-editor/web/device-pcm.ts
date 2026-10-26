/** Resample continuously: retain fractional phase across blocks. */
export class SpeechPcm {
  private rate: number;
  private samples: number[] = [];
  private position = 0;
  private block: number[] = [];
  constructor(rate: number) {
    if (!Number.isFinite(rate) || rate < 16000) throw new Error('不支持的麦克风采样率');
    this.rate = rate / 16000;
  }
  push(input: Float32Array): Uint8Array[] {
    this.samples.push(...input);
    const blocks: Uint8Array[] = [];
    while (this.position + 1 < this.samples.length) {
      const index = Math.floor(this.position), fraction = this.position - index;
      const value = this.samples[index]! * (1 - fraction) + this.samples[index + 1]! * fraction;
      this.block.push(Math.round(Math.max(-1, Math.min(1, value)) * (value < 0 ? 32768 : 32767)));
      this.position += this.rate;
      if (this.block.length === 320) blocks.push(this.flush());
    }
    const consumed = Math.min(Math.floor(this.position), this.samples.length);
    this.samples = this.samples.slice(consumed); this.position -= consumed;
    return blocks;
  }
  flush(): Uint8Array {
    const bytes = new Uint8Array(this.block.length * 2), view = new DataView(bytes.buffer);
    this.block.forEach((sample, i) => view.setInt16(i * 2, sample, true));
    this.block = [];
    return bytes;
  }
}
