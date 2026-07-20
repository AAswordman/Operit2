const moduleId = "core";

/** Calls one i32 AssemblyScript export and validates the scalar result. */
async function callI32(exportName: string, value: number): Promise<number> {
  const result = await ToolPkg.wasm.call(moduleId, exportName, [{ type: "i32", value }]);
  if (typeof result !== "number") {
    throw new Error(`${moduleId}.${exportName} returned a non-number result`);
  }
  return result;
}

/** Returns whether the provided integer is prime by calling WASM. */
export async function isPrime(n: number): Promise<boolean> {
  return (await callI32("isPrime", n)) === 1;
}

/** Returns the nth prime number by calling WASM. */
export async function nthPrime(index: number): Promise<number> {
  return callI32("nthPrime", index);
}
