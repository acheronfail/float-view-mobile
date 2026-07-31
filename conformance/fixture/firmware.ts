export type FirmwareVersion = '6.02' | '6.05' | '6.06' | '7.00';

export interface ControllerIdentity {
	hardwareName: string;
	hardwareType: number;
	uuid: Uint8Array;
}

export interface FirmwareVariant {
	version: FirmwareVersion;
	major: number;
	minor: number;
	hardwareCrc?: Uint8Array;
	testVersion: number;
	payload: Uint8Array;
	provenance: string;
}

export const thor300: ControllerIdentity = {
	hardwareName: 'THOR300',
	hardwareType: 0,
	uuid: new Uint8Array(12)
};

/** VESC 7.00 COMM_GET_MCCONF with current limits and an 18S battery setup. */
export const motorConfigurationPayload = (() => {
	const payload = new Uint8Array(489);
	payload.set(
		[
			14,
			0x57, 0xad, 0x8f, 0x53,
			0, 0, 0, 0,
			0x42, 0xc8, 0, 0,
			0xc2, 0xc8, 0, 0,
			0x42, 0x48, 0, 0,
			0xc1, 0xa0, 0, 0
		],
		0
	);
	payload[463] = 18;
	return payload;
})();

const provenance =
	'VESC COMM_FW_VERSION field ordering derived from vedderb/bldc and vedderb/vesc_fw_archive version changelogs; THOR300 identity is deliberate fixture data';

export const firmwareVariants: Readonly<Record<FirmwareVersion, FirmwareVariant>> = {
	'6.02': variant(6, 2),
	'6.05': variant(6, 5, Uint8Array.of(0, 0, 0, 0)),
	'6.06': variant(6, 6, Uint8Array.of(0, 0, 0, 0)),
	'7.00': variant(7, 0, Uint8Array.of(0, 0, 0, 0))
};

function variant(
	major: number,
	minor: number,
	hardwareCrc?: Uint8Array,
	testVersion = 0
): FirmwareVariant {
	const version = `${major}.${minor.toString().padStart(2, '0')}` as FirmwareVersion;
	return {
		version,
		major,
		minor,
		hardwareCrc,
		testVersion,
		payload: encodeFirmwarePayload(thor300, major, minor, hardwareCrc, testVersion),
		provenance
	};
}

export function encodeFirmwarePayload(
	identity: ControllerIdentity,
	major: number,
	minor: number,
	hardwareCrc?: Uint8Array,
	testVersion = 0
): Uint8Array {
	const name = new TextEncoder().encode(identity.hardwareName);
	const flags = Uint8Array.of(0, testVersion, identity.hardwareType, 0, 0, 0, 0, 0);
	return Uint8Array.from([
		0,
		major,
		minor,
		...name,
		0,
		...identity.uuid,
		...flags,
		0,
		...(hardwareCrc ?? [])
	]);
}
