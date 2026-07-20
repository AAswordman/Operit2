import { isPrime, nthPrime } from "./wasm/core";

/** Validates a positive integer input for the demo tools. */
function positiveInteger(value: number, name: string): number {
  if (!Number.isInteger(value) || value < 1) {
    throw new Error(`${name} must be a positive integer`);
  }
  return value;
}

/** Returns whether the provided integer is prime. */
export async function is_prime(params: { n: number }) {
  const n = positiveInteger(params.n, "n");
  return {
    n,
    is_prime: await isPrime(n),
  };
}

/** Returns the nth prime number for a one-based index. */
export async function nth_prime(params: { index: number }) {
  const index = positiveInteger(params.index, "index");
  return {
    index,
    prime: await nthPrime(index),
  };
}
