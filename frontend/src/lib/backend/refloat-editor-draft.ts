import { MAX_REFLOAT_EDITOR_DRAFT_BYTES, type RefloatEditorDraft } from './contract.ts';

const record = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);
const keys = (v: Record<string, unknown>, names: string[]) =>
  Object.keys(v).length === names.length && names.every((name) => name in v);
const text = (v: unknown, max: number): v is string =>
  typeof v === 'string' && new TextEncoder().encode(v).length <= max;
function value(v: unknown): boolean {
  if (!record(v) || !keys(v, ['type', 'value'])) return false;
  switch (v.type) {
    case 'double':
      return typeof v.value === 'number' && Number.isFinite(v.value);
    case 'integer':
      return Number.isSafeInteger(v.value);
    case 'boolean':
      return typeof v.value === 'boolean';
    case 'string':
      return text(v.value, 4096);
    default:
      return false;
  }
}
export function validRefloatEditorDraft(v: unknown): v is RefloatEditorDraft {
  if (
    !record(v) ||
    !keys(v, ['version', 'identity', 'source', 'values', 'inputs', 'position']) ||
    v.version !== 1 ||
    !['current', 'default'].includes(String(v.source)) ||
    new TextEncoder().encode(JSON.stringify(v)).length > MAX_REFLOAT_EDITOR_DRAFT_BYTES
  )
    return false;
  const identity = v.identity;
  if (
    !record(identity) ||
    !keys(identity, ['packageName', 'packageVersion', 'schemaFingerprint', 'wireSignature']) ||
    identity.packageName !== 'refloat' ||
    !text(identity.packageVersion, 128) ||
    !identity.packageVersion ||
    typeof identity.schemaFingerprint !== 'string' ||
    !/^[0-9a-f]{64}$/.test(identity.schemaFingerprint) ||
    !Number.isInteger(identity.wireSignature) ||
    Number(identity.wireSignature) < 0 ||
    Number(identity.wireSignature) > 0xffffffff
  )
    return false;
  const p = v.position;
  if (
    !record(p) ||
    !keys(p, ['sectionKey', 'search', 'controlsExpanded', 'scrollTop']) ||
    !text(p.sectionKey, 1024) ||
    !text(p.search, 512) ||
    typeof p.controlsExpanded !== 'boolean' ||
    typeof p.scrollTop !== 'number' ||
    !Number.isFinite(p.scrollTop) ||
    p.scrollTop < 0 ||
    p.scrollTop > 10_000_000
  )
    return false;
  return (
    record(v.values) &&
    Object.keys(v.values).length > 0 &&
    Object.keys(v.values).length <= 512 &&
    Object.entries(v.values).every(([name, v]) => name.length > 0 && text(name, 128) && value(v)) &&
    record(v.inputs) &&
    Object.keys(v.inputs).length <= 512 &&
    Object.entries(v.inputs).every(([name, input]) => Object.hasOwn(v.values as object, name) && text(input, 128))
  );
}
