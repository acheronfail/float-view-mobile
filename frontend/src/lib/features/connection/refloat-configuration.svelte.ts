import type {
  RefloatConfigurationField,
  RefloatConfigurationIdentity,
  RefloatConfigurationSchemaPage,
  RefloatConfigurationSummary,
  RefloatConfigurationValue,
  RefloatConfigurationValues,
} from '$lib/backend/contract';

export interface RefloatConfigurationBackend {
  openRefloatConfiguration(): Promise<RefloatConfigurationSummary | null>;
  refloatConfigurationSchema(cursor?: string | null): Promise<RefloatConfigurationSchemaPage | null>;
  readRefloatConfiguration(source: 'current' | 'default'): Promise<RefloatConfigurationValues | null>;
  writeRefloatConfiguration(
    identity: RefloatConfigurationIdentity,
    baseRevision: number,
    values: Record<string, RefloatConfigurationValue>
  ): Promise<RefloatConfigurationValues | null>;
}

export type RefloatConfigurationDiff = {
  name: string;
  label: string;
  before: RefloatConfigurationValue;
  after: RefloatConfigurationValue;
};

export type RefloatConfigurationSection = {
  key: string;
  group: string;
  subgroup: string;
  label: string;
  fields: RefloatConfigurationField[];
};

export function refloatConfigurationSectionKey(group: string, subgroup: string): string {
  return JSON.stringify([group, subgroup]);
}

export function refloatConfigurationFieldLabel(field: RefloatConfigurationField): string {
  return field.name === 'leds.on' || field.name === 'leds.headlights_on'
    ? `${field.label} (persistent startup default)`
    : field.label;
}

export function refloatConfigurationFieldHelp(field: RefloatConfigurationField): string {
  return field.description || field.name;
}

export function resolveRefloatConfigurationSection(
  sections: readonly RefloatConfigurationSection[],
  selectedKey: string
): RefloatConfigurationSection | null {
  return sections.find((section) => section.key === selectedKey) ?? sections[0] ?? null;
}

export class RefloatConfigurationController {
  summary = $state<RefloatConfigurationSummary | null>(null);
  pages = $state<RefloatConfigurationSchemaPage[]>([]);
  loaded = $state<RefloatConfigurationValues | null>(null);
  draft = $state<Record<string, RefloatConfigurationValue>>({});
  source = $state<'current' | 'default'>('current');
  search = $state('');
  loading = $state(false);
  error = $state<string | null>(null);

  constructor(private readonly backend: RefloatConfigurationBackend) {}

  get identity(): RefloatConfigurationIdentity | null {
    return this.pages[0]?.identity ?? null;
  }

  get fields(): RefloatConfigurationField[] {
    return this.pages.flatMap((page) => page.fields);
  }

  get sections(): RefloatConfigurationSection[] {
    const sections: Array<Omit<RefloatConfigurationSection, 'label'>> = [];
    for (const page of this.pages) {
      const key = refloatConfigurationSectionKey(page.group, page.subgroup);
      const section = sections.find((item) => item.key === key);
      if (section) section.fields.push(...page.fields);
      else sections.push({ key, group: page.group, subgroup: page.subgroup, fields: [...page.fields] });
    }
    return sections.map((section) => ({
      ...section,
      label:
        sections.filter((item) => item.subgroup === section.subgroup).length === 1
          ? section.subgroup
          : `${section.group} · ${section.subgroup}`,
    }));
  }

  filteredSectionFields(section: RefloatConfigurationSection | null): RefloatConfigurationField[] {
    if (!section) return [];
    const query = this.search.trim().toLocaleLowerCase();
    if (!query) return section.fields;
    return section.fields.filter((field) =>
      [section.group, section.subgroup, field.name, field.label, field.description].some((value) =>
        value.toLocaleLowerCase().includes(query)
      )
    );
  }

  get validationErrors(): Record<string, string> {
    return Object.fromEntries(
      this.fields.flatMap((field) => {
        const error = validateRefloatField(field, this.draft[field.name]);
        return error ? [[field.name, error]] : [];
      })
    );
  }

  get diff(): RefloatConfigurationDiff[] {
    if (!this.loaded) return [];
    return this.fields.flatMap((field) => {
      const before = this.loaded!.values[field.name];
      const after = this.draft[field.name];
      return before && after && !sameRefloatValue(before, after)
        ? [{ name: field.name, label: field.label, before, after }]
        : [];
    });
  }

  get canWrite(): boolean {
    return Boolean(
      this.summary?.writable &&
        this.source === 'current' &&
        this.loaded?.source === 'current' &&
        this.identity &&
        this.diff.length &&
        !Object.keys(this.validationErrors).length
    );
  }

  async open(): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      const summary = await this.backend.openRefloatConfiguration();
      if (!summary) throw new Error('Refloat configuration is unavailable');
      this.summary = summary;
      const pages: RefloatConfigurationSchemaPage[] = [];
      let cursor: string | null = null;
      for (let count = 0; count < 64; count += 1) {
        const page = await this.backend.refloatConfigurationSchema(cursor);
        if (!page) throw new Error('Unable to load the Refloat schema');
        pages.push(page);
        cursor = page.nextCursor;
        if (!cursor) break;
      }
      if (cursor) throw new Error('Refloat schema page limit exceeded');
      if (!pages.length) throw new Error('Refloat schema is empty');
      this.pages = pages;
      if (summary.currentLoaded) await this.load('current');
      else if (summary.defaultLoaded) await this.load('default');
    } catch (error) {
      this.error = error instanceof Error ? error.message : String(error);
    } finally {
      this.loading = false;
    }
  }

  async load(source: 'current' | 'default'): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      const values = await this.backend.readRefloatConfiguration(source);
      if (!values || !this.identity || !sameRefloatIdentity(values.identity, this.identity)) {
        throw new Error('Configuration identity changed while loading values');
      }
      this.source = source;
      this.loaded = values;
      this.draft = structuredClone(values.values);
    } catch (error) {
      this.error = error instanceof Error ? error.message : String(error);
    } finally {
      this.loading = false;
    }
  }

  setValue(name: string, value: RefloatConfigurationValue): void {
    if (!this.fields.some((field) => field.name === name)) return;
    this.draft = { ...this.draft, [name]: value };
  }

  resetDraft(): void {
    if (this.loaded) this.draft = structuredClone(this.loaded.values);
  }

  reconcile(summary: RefloatConfigurationSummary | null): void {
    this.summary = summary;
    const identity = this.identity;
    if (
      !summary ||
      !identity ||
      summary.packageVersion !== identity.packageVersion ||
      summary.schemaFingerprint !== identity.schemaFingerprint ||
      summary.wireSignature !== identity.wireSignature ||
      !summary.currentLoaded
    ) {
      this.loaded = null;
      this.draft = {};
    }
  }

  async write(): Promise<boolean> {
    const identity = this.identity;
    const loaded = this.loaded;
    if (!identity || !loaded || !this.canWrite) return false;
    this.loading = true;
    try {
      const result = await this.backend.writeRefloatConfiguration(identity, loaded.revision, this.draft);
      if (!result) return false;
      this.loaded = result;
      this.draft = structuredClone(result.values);
      return true;
    } finally {
      this.loading = false;
    }
  }
}

export function validateRefloatField(
  field: RefloatConfigurationField,
  value: RefloatConfigurationValue | undefined
): string | null {
  if (!value) return 'A value is required.';
  if (field.type === 'double') {
    if (value.type !== 'double' || !Number.isFinite(value.value)) return 'Enter a finite number.';
    if (field.minimum !== null && value.value < field.minimum) return `Minimum ${field.minimum}.`;
    if (field.maximum !== null && value.value > field.maximum) return `Maximum ${field.maximum}.`;
  } else if (['integer', 'enum', 'bitfield'].includes(field.type)) {
    if (value.type !== 'integer' || !Number.isSafeInteger(value.value)) return 'Enter a whole number.';
    if (field.minimum !== null && value.value < field.minimum) return `Minimum ${field.minimum}.`;
    if (field.maximum !== null && value.value > field.maximum) return `Maximum ${field.maximum}.`;
    if (field.type === 'enum' && (value.value < 0 || value.value >= field.choices.length))
      return 'Choose a listed value.';
  } else if (field.type === 'boolean' && value.type !== 'boolean') return 'Choose on or off.';
  else if (
    field.type === 'string' &&
    (value.type !== 'string' || new TextEncoder().encode(value.value).length > 4_096)
  ) {
    return 'Text is too long.';
  }
  return null;
}

export function sameRefloatValue(left: RefloatConfigurationValue, right: RefloatConfigurationValue): boolean {
  return left.type === right.type && left.value === right.value;
}

export function sameRefloatIdentity(left: RefloatConfigurationIdentity, right: RefloatConfigurationIdentity): boolean {
  return (
    left.packageName === right.packageName &&
    left.packageVersion === right.packageVersion &&
    left.schemaFingerprint === right.schemaFingerprint &&
    left.wireSignature === right.wireSignature
  );
}

export function formatRefloatValue(value: RefloatConfigurationValue, field?: RefloatConfigurationField): string {
  if (value.type === 'boolean') return value.value ? 'On' : 'Off';
  if (value.type === 'integer' && field?.type === 'enum') return field.choices[value.value] ?? String(value.value);
  return `${value.value}${field?.suffix ? ` ${field.suffix}` : ''}`;
}
