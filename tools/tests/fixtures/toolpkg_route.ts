import type {} from "../../../plugins/types/toolpkg";
import type {} from "../../../plugins/types/core";

/** Implements ordinary async services used by the route integration tests. */
class ExampleService {
  static count = 0;
  /** Executes the implemented method on the Binding-selected runtime. */
  @ToolPkg.route({ binding: 0 })
  static async read(chatId: string, amount: number): Promise<{ chatId: string; count: number }> {
    this.count += amount;
    return { chatId, count: this.count };
  }
  /** Emits values using standard async generator syntax. */
  @ToolPkg.route({ binding: 0 })
  static async *values(chatId: string, signal?: AbortSignal): AsyncGenerator<string> {
    if (!signal) throw new Error("Collector signal is required");
    yield chatId;
    await new Promise<void>((resolve) => { signal.addEventListener("abort", resolve); });
  }
  /** Consumes ordered input using a standard for-await loop. */
  @ToolPkg.route({ binding: 0 })
  static async consume(chatId: string, input: AsyncIterable<number>): Promise<void> {
    for await (const value of input) {
      if (value < 0) throw new Error("Consumer rejected a negative value");
      this.count += value;
    }
  }
}

/** Verifies that instance receivers and same-named methods on other classes stay distinct. */
class InstanceService {
  prefix = "instance:";
  /** Preserves the registered instance as the remote receiver. */
  @ToolPkg.route({ binding: 0 })
  async read(chatId: string): Promise<string> { return this.prefix + chatId; }
}

Object.assign(globalThis, { ExampleService, instanceService: new InstanceService() });
