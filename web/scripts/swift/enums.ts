import { Enums, enumVariants } from "../common";

export function generateSwiftEnums(enums: Enums) {
  const lines: string[] = [];

  for (const [enumName, e] of Object.entries(enums)) {
    const variants = enumVariants(e);
    lines.push(`enum ${enumName}: Int {`);
    for (const [name, value] of variants) {
      lines.push(`    case ${name} = ${value}`);
    }
    lines.push('}');
    lines.push('');
  }

  return lines.join('\n');
}
