import { createHash } from 'node:crypto';
import { inflateSync } from 'node:zlib';

interface Field {
  name: string;
  offset: number;
  bytes: number;
  signed: boolean;
  scale: number;
  minimum: number;
  maximum: number;
}

/** Fixed-width value layout from an immutable, fingerprint-checked fixture.
 * This deliberately reads only the audited XML's numeric wire metadata, not
 * arbitrary device XML or the HTML descriptions embedded in it.
 */
export class RefloatConfigurationValuesLayout {
  private readonly fields: readonly Field[];
  readonly byteLength: number;

  constructor(compressed: Uint8Array, fingerprint: string) {
    const expanded = inflateSync(compressed.subarray(4), { maxOutputLength: 384 * 1_024 });
    const envelope = new DataView(compressed.buffer, compressed.byteOffset, compressed.byteLength);
    if (
      expanded.length !== envelope.getUint32(0) ||
      createHash('sha256').update(expanded).digest('hex') !== fingerprint
    ) {
      throw new Error('Invalid Refloat configuration schema fixture');
    }
    const xml = expanded.toString('utf8');
    const parameters = new Map(
      [...section(xml, 'Params').matchAll(/<([\w.]+)>([\s\S]*?)<\/\1>/g)].map((match) => [match[1]!, match[2]!])
    );
    let offset = 4; // Wire signature precedes the serialized fields.
    this.fields = [...section(xml, 'SerOrder').matchAll(/<ser>([\w.]+)<\/ser>/g)].map((match) => {
      const name = match[1]!;
      const metadata = parameters.get(name);
      if (!metadata) throw new Error(`Missing fixture metadata for ${name}`);
      const type = numeric(metadata, 'type');
      const wire = type === 1 || type === 2 ? numeric(metadata, 'vTx') : 0;
      const bytes =
        type === 1
          ? wire === 7
            ? 2
            : wire === 8
              ? 4
              : 0
          : type === 2
            ? ({ 1: 1, 2: 1, 3: 2, 4: 2, 5: 4, 6: 4 }[wire] ?? 0)
            : type === 4 || type === 5
              ? 1
              : 0;
      if (!bytes) throw new Error(`Unsupported fixture wire encoding for ${name}`);
      const scale = type === 1 ? numeric(metadata, 'vTxDoubleScale') : 1;
      if (scale <= 0) throw new Error(`Invalid fixture scale for ${name}`);
      const field = {
        name,
        offset,
        bytes,
        scale,
        signed: type !== 2 || wire % 2 === 0,
        minimum: type === 1 ? numeric(metadata, 'minDouble') : type === 2 ? numeric(metadata, 'minInt') : 0,
        maximum:
          type === 1
            ? numeric(metadata, 'maxDouble')
            : type === 2
              ? numeric(metadata, 'maxInt')
              : type === 4
                ? [...metadata.matchAll(/<enumNames(?:\s*\/|>[\s\S]*?<\/enumNames)>/g)].length - 1
                : 1,
      };
      offset += bytes;
      return field;
    });
    if (!this.fields.length || this.fields.length > 512 || offset > 1_024)
      throw new Error('Invalid fixture value layout');
    this.byteLength = offset;
  }

  rejection(values: Uint8Array, signature: number): string | undefined {
    if (values.length !== this.byteLength)
      return `configuration length mismatch (expected ${this.byteLength}, received ${values.length})`;
    const view = new DataView(values.buffer, values.byteOffset, values.byteLength);
    if (view.getUint32(0) !== signature) return 'configuration wire signature mismatch';
    for (const field of this.fields) {
      const value = read(view, field) / field.scale;
      if (value < field.minimum || value > field.maximum) return `invalid value for ${field.name}`;
    }
    return undefined;
  }

  normalize(values: Uint8Array, rules: readonly { field: string; value: number }[]): Uint8Array {
    const copy = values.slice();
    const view = new DataView(copy.buffer);
    for (const rule of rules) {
      const field = this.fields.find((item) => item.name === rule.field);
      // The audited releases only clear the one-byte meta.is_default flag.
      if (!field || field.bytes !== 1 || rule.value !== 0) throw new Error('Unsupported fixture normalization');
      view.setUint8(field.offset, rule.value);
    }
    return copy;
  }
}

function section(xml: string, tag: string): string {
  const value = xml.match(new RegExp(`<${tag}>([\\s\\S]*?)</${tag}>`))?.[1];
  if (value === undefined) throw new Error(`Missing fixture section ${tag}`);
  return value;
}

function numeric(xml: string, tag: string): number {
  const value = Number(section(xml, tag));
  if (!Number.isFinite(value)) throw new Error(`Invalid fixture metadata ${tag}`);
  return value;
}

function read(view: DataView, field: Field): number {
  if (field.bytes === 1) return field.signed ? view.getInt8(field.offset) : view.getUint8(field.offset);
  if (field.bytes === 2) return field.signed ? view.getInt16(field.offset) : view.getUint16(field.offset);
  return field.signed ? view.getInt32(field.offset) : view.getUint32(field.offset);
}
