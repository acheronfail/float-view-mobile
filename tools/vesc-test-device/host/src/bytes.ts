export function fromHex(value: string): Uint8Array {
  if (value.length === 0 || value.length % 2 !== 0 || !/^[0-9a-f]+$/i.test(value)) {
    throw new Error("hex must contain a non-empty, even number of hexadecimal characters");
  }
  return Uint8Array.from(Buffer.from(value, "hex"));
}

export function toHex(value: Uint8Array): string {
  return Buffer.from(value).toString("hex");
}

export function concatBytes(...values: Uint8Array[]): Uint8Array {
  const length = values.reduce((total, value) => total + value.length, 0);
  const result = new Uint8Array(length);
  let offset = 0;
  for (const value of values) {
    result.set(value, offset);
    offset += value.length;
  }
  return result;
}
