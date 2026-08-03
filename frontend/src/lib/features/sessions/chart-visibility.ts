export interface ChartVisibilityField {
  name: string;
}

export function toggleChartField(hidden: ReadonlySet<string>, fieldName: string): Set<string> {
  const next = new Set(hidden);
  if (next.has(fieldName)) next.delete(fieldName);
  else next.add(fieldName);
  return next;
}

export function firstVisibleChartField(
  fields: readonly ChartVisibilityField[],
  hidden: ReadonlySet<string>
): string | null {
  return fields.find((field) => !hidden.has(field.name))?.name ?? null;
}
