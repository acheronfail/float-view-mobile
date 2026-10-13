import type {
  RefloatConfigurationField,
  RefloatEditorDraft,
  RefloatConfigurationIdentity,
  RefloatConfigurationSchemaPage,
  RefloatConfigurationSummary,
  RefloatConfigurationValue,
  RefloatConfigurationValues,
} from '$lib/backend/contract';

export interface RefloatConfigurationBackend {
  getRefloatEditorDraft(deviceId: string): Promise<RefloatEditorDraft | null>;
  setRefloatEditorDraft(deviceId: string, baseRevision: number, draft: RefloatEditorDraft): Promise<void>;
  openRefloatConfiguration(): Promise<RefloatConfigurationSummary | null>;
  refloatConfigurationSchema(cursor?: string | null): Promise<RefloatConfigurationSchemaPage | null>;
  readRefloatConfiguration(
    source: 'current' | 'default',
    refresh?: boolean
  ): Promise<RefloatConfigurationValues | null>;
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
  position = $state<RefloatEditorDraft['position']>({
    sectionKey: '',
    search: '',
    controlsExpanded: true,
    scrollTop: 0,
  });
  inputs = $state<Record<string, string>>({});
  incompatible = $state<RefloatEditorDraft | null>(null);
  restored = $state(false);
  saveError = $state<string | null>(null);
  private checkpointReady = false;
  private fresh = false;
  private queued: { draft: RefloatEditorDraft; revision: number } | null = null;
  private saving: Promise<void> | null = null;
  private lastSaved = '';
  private positionTimer: ReturnType<typeof setTimeout> | null = null;

  get search(): string {
    return this.position.search;
  }
  set search(search: string) {
    this.setPosition({ search, scrollTop: 0 });
  }
  loading = $state(false);
  error = $state<string | null>(null);

  constructor(
    private readonly backend: RefloatConfigurationBackend,
    private readonly deviceId = ''
  ) {}

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
        const input = this.inputs[field.name];
        const error =
          input !== undefined && (!input.trim() || !Number.isFinite(Number(input)))
            ? 'Enter a finite number.'
            : input !== undefined && field.type !== 'double' && !Number.isSafeInteger(Number(input))
              ? 'Enter a whole number.'
              : validateRefloatField(field, this.draft[field.name]);
        return error ? [[field.name, error]] : [];
      })
    );
  }

  get diff(): RefloatConfigurationDiff[] {
    if (!this.loaded) return [];
    return this.fields.flatMap((field) => {
      const before = this.loaded!.values[field.name];
      const after = this.draft[field.name];
      const input = this.inputs[field.name];
      const changedInput = input !== undefined && (!input.trim() || Number(input) !== before?.value);
      return before && after && (!sameRefloatValue(before, after) || changedInput)
        ? [{ name: field.name, label: field.label, before, after }]
        : [];
    });
  }

  get canWrite(): boolean {
    return Boolean(
      this.summary?.writable &&
        this.summary.currentLoaded &&
        this.fresh &&
        !this.incompatible &&
        this.source === 'current' &&
        this.loaded?.source === 'current' &&
        this.identity &&
        this.diff.length &&
        !Object.keys(this.validationErrors).length
    );
  }

  async open(): Promise<void> {
    if (this.loading) return;
    const local = this.loaded ? this.checkpoint() : null;
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
      let saved = local;
      if (this.deviceId && !saved) {
        try {
          saved = await this.backend.getRefloatEditorDraft(this.deviceId);
          this.checkpointReady = true;
        } catch {
          this.saveError = 'Unable to restore your saved draft. Retry before editing.';
          this.checkpointReady = false;
          return;
        }
      } else this.checkpointReady = true;
      const source = saved?.source ?? (summary.currentLoaded ? 'current' : 'default');
      await this.load(source, saved, false);
      if (saved) {
        this.position = structuredClone(saved.position);
        this.restored = true;
      }
    } catch (error) {
      this.error = error instanceof Error ? error.message : String(error);
    } finally {
      this.loading = false;
    }
  }

  async load(source: 'current' | 'default', saved: RefloatEditorDraft | null = null, refresh = true): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      const values = await this.backend.readRefloatConfiguration(source, refresh);
      if (!values || !this.identity || !sameRefloatIdentity(values.identity, this.identity)) {
        throw new Error('Configuration identity changed while loading values');
      }
      this.source = source;
      this.loaded = values;
      this.draft = structuredClone(values.values);
      this.inputs = {};
      this.incompatible = null;
      this.fresh = true;
      this.checkpointReady = true;
      if (saved && !sameRefloatIdentity(saved.identity, this.identity)) {
        this.incompatible = saved;
      } else if (saved) {
        this.draft = structuredClone(saved.values);
        this.inputs = structuredClone(saved.inputs);
      }
      if (!saved) {
        this.restored = false;
        this.capture();
      }
    } catch (error) {
      this.error = error instanceof Error ? error.message : String(error);
    } finally {
      this.loading = false;
    }
  }

  setValue(name: string, value: RefloatConfigurationValue): void {
    if (!this.fields.some((field) => field.name === name)) return;
    this.draft = { ...this.draft, [name]: value };
    const inputs = { ...this.inputs };
    delete inputs[name];
    this.inputs = inputs;
    this.capture();
  }

  setNumericInput(name: string, input: string): void {
    const field = this.fields.find((field) => field.name === name);
    if (!field || input.length > 128) return;
    this.inputs = { ...this.inputs, [name]: input };
    const value = Number(input);
    if (input.trim() && Number.isFinite(value) && (field.type === 'double' || Number.isSafeInteger(value)))
      this.draft = { ...this.draft, [name]: { type: field.type === 'double' ? 'double' : 'integer', value } };
    this.capture();
  }

  setPosition(patch: Partial<RefloatEditorDraft['position']>): void {
    this.position = { ...this.position, ...patch };
    if (Object.keys(patch).length === 1 && 'scrollTop' in patch) {
      if (!this.positionTimer) this.positionTimer = setTimeout(() => this.capture(), 100);
    } else this.capture();
  }

  acceptValues(values: RefloatConfigurationValues): void {
    this.loaded = values;
    this.source = values.source;
    this.draft = structuredClone(values.values);
    this.inputs = {};
    this.incompatible = null;
    this.fresh = true;
    this.capture();
  }

  private checkpoint(): RefloatEditorDraft | null {
    if (this.incompatible) return this.incompatible;
    if (!this.loaded || !this.identity) return null;
    return {
      version: 1,
      identity: this.identity,
      source: this.source,
      values: this.draft,
      inputs: this.inputs,
      position: this.position,
    };
  }

  capture(): void {
    if (this.positionTimer) {
      clearTimeout(this.positionTimer);
      this.positionTimer = null;
    }
    if (!this.deviceId || !this.loaded || !this.checkpointReady || !this.fresh || this.incompatible) return;
    const draft = this.checkpoint();
    if (!draft) return;
    const encoded = JSON.stringify(draft);
    if (encoded === this.lastSaved && !this.saving) return;
    this.queued = { draft: JSON.parse(encoded) as RefloatEditorDraft, revision: this.loaded.revision };
    if (!this.saving)
      this.saving = this.drain().finally(() => {
        this.saving = null;
        if (this.queued) this.capture();
      });
  }

  private async drain(): Promise<void> {
    while (this.queued) {
      const next = this.queued;
      this.queued = null;
      try {
        await this.backend.setRefloatEditorDraft(this.deviceId, next.revision, next.draft);
        this.lastSaved = JSON.stringify(next.draft);
        this.saveError = null;
      } catch {
        this.saveError = 'Unable to save your draft. Keep this screen open and retry.';
      }
    }
  }

  async flush(): Promise<boolean> {
    this.capture();
    while (this.saving) await this.saving;
    return this.saveError === null;
  }

  reconcile(summary: RefloatConfigurationSummary | null): void {
    this.summary = summary;
    const identity = this.identity;
    const valid = Boolean(
      summary &&
        identity &&
        summary.packageVersion === identity.packageVersion &&
        summary.schemaFingerprint === identity.schemaFingerprint &&
        summary.wireSignature === identity.wireSignature &&
        summary.currentLoaded
    );
    if (!valid) this.fresh = false;
    if (summary?.availability === 'ready' && summary.currentLoaded && !this.fresh && this.loaded && !this.loading)
      void this.open();
  }

  async write(): Promise<boolean> {
    const identity = this.identity;
    const loaded = this.loaded;
    if (!identity || !loaded || !this.canWrite) return false;
    this.loading = true;
    try {
      const result = await this.backend.writeRefloatConfiguration(identity, loaded.revision, this.draft);
      if (!result) return false;
      this.acceptValues(result);
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
