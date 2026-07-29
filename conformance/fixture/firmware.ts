export type FirmwareVersion = '6.02' | '6.05' | '6.06';

export interface ControllerIdentity {
	hardwareName: 'THOR300';
	hardwareType: 0;
	uuid: Uint8Array;
}

export interface FirmwareVariant {
	version: FirmwareVersion;
	major: number;
	minor: number;
	hardwareCrc?: Uint8Array;
	payload: Uint8Array;
	provenance: string;
}

export const thor300: ControllerIdentity = {
	hardwareName: 'THOR300',
	hardwareType: 0,
	uuid: new Uint8Array(12)
};

const provenance =
	'VESC COMM_FW_VERSION field ordering derived from vedderb/bldc and vedderb/vesc_fw_archive version changelogs; THOR300 identity is deliberate fixture data';

export const firmwareVariants: Readonly<Record<FirmwareVersion, FirmwareVariant>> = {
	'6.02': variant(6, 2),
	'6.05': variant(6, 5),
	'6.06': variant(6, 6, Uint8Array.of(0, 0, 0, 0))
};

function variant(major: number, minor: number, hardwareCrc?: Uint8Array): FirmwareVariant {
	const version = `${major}.${minor.toString().padStart(2, '0')}` as FirmwareVersion;
	return {
		version,
		major,
		minor,
		hardwareCrc,
		payload: encodeFirmwarePayload(thor300, major, minor, hardwareCrc),
		provenance
	};
}

export function encodeFirmwarePayload(
	identity: ControllerIdentity,
	major: number,
	minor: number,
	hardwareCrc?: Uint8Array
): Uint8Array {
	const name = new TextEncoder().encode(identity.hardwareName);
	const flags = Uint8Array.of(0, 0, identity.hardwareType, 0, 0, 0, 0, 0);
	return Uint8Array.from([
		0,
		major,
		minor,
		...name,
		0,
		...identity.uuid,
		...flags,
		...(hardwareCrc ?? [])
	]);
}
