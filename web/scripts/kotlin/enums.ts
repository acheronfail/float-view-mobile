import { Enums, enumVariants } from "../types";

export function generateKotlinEnums(banner: string, enums: Enums) {
	const lines = [banner];

	for (const [enumName, e] of Object.entries(enums)) {
		const variants = enumVariants(e);
    lines.push(`enum class ${enumName}(val value: Int) {`);
    for (const [name, value] of variants) {
      lines.push(`    ${name}(${value}),`);
    }
    lines.push(`    ;`);
    lines.push(``);
    lines.push(`    companion object {`);
    lines.push(`        private val map = values().associateBy(${enumName}::value)`);
    lines.push(`        fun fromInt(type: Int) = map[type]`);
    lines.push(`    }`);
    lines.push(`}`);
    lines.push(``);
	}

	return lines.join('\n');
}
