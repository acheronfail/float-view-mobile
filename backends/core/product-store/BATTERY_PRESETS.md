# Battery preset catalog

`src/battery-presets.json` is the shared service-owned pack catalog. The development mock reads this same file. It is separate from the unchanged discharge curves. Basic setup displays familiar names without generic-curve warnings; Advanced exposes the actual profile.

A non-null ID in a patch selects all defaults atomically and rejects mixed battery overrides. Imports validate complete records, preserving capacity/empty overrides. Manual count edits clear capacity unless explicitly replaced; count/curve departures clear identity. Reads never apply defaults.

Research on 2026-09-30 inspected Float Hub 1.5.0 (package `com.thefloatlife.FloatHub`, version code 48), downloaded from [APKPure](https://apkpure.net/float-hub/com.thefloatlife.FloatHub/download/1.5.0). XAPK SHA-256: `4cc1e0e4c279b7e371038eaba84a57c701af42a96be8f872b738dfe7e4723db8`. Static Hermes v96 inspection found 48 preset records, 15 named cell records, and Custom. Its preset selection populates S/P/cell, capacity is cell Ah × P, and manual S/P changes select Custom. The app was not executed and publisher signing identity was not independently authenticated. Use the factual configuration evidence; do not vendor the APK, disassembly, or app implementation.

[Developer documentation](https://pev.dev/t/float-hub-documentation/2406) corroborates the preset approach but predates some APK additions. Manufacturer cross-checks include [X7 Long Range](https://fungineers.us/products/funwheel-x7-long-range), [X7S](https://fungineers.crisp.help/en/article/everything-you-need-to-know-about-the-x7s-1mzbf5l/), [Floatwheel Atom](https://floatwheel.co/index.php?product_id=9935&route=product%2Fproduct), [Z Battery Torque Pack](https://zbattery.solutions/products/torque-pack-18s2p-p42a), [Indy 18S2P](https://indyspeedcontrol.com/general-batteries/p/182pvesc), and [Chi 32.1](https://chibatterysystems.com/collections/onewheel-batteries/products/chivexrgt32-1). The latter two explicitly offer cell variants: an APK default is not proof of an installed pack's cell. The Atom manufacturer confirms 22S1P, resolving an inconsistent 2P description in the developer's controller table.

Seed the following bounded, variant-specific selection catalog. Each row's capacity is nominal, not measured. Include the manufacturer in ambiguous names and the cell variant where a product has alternatives; split the XR/Pint X shared research row into separate friendly labels with identical settings. Existing curve IDs are used below; `generic-linear` is internal metadata, never Basic copy. M35A identifies the M35A variant only; a Sanyo GA variant uses generic instead. Do not generalize the GT P42A entry to all GT production batteries. Retain supported installed older packs even when no longer sold; these are current hardware choices, not legacy storage compatibility.

| Manufacturer / battery variant           | S/P   | Cells           | Nominal pack Ah | Internal curve   |
| ---------------------------------------- | ----- | --------------- | --------------- | ---------------- |
| Floatwheel — ADV                         | 20S2P | LR2170LA / DG40 | 8               | `dg-01-dg-40`    |
| Floatwheel — ADV Pro                     | 20S2P | Molicel P42A    | 8.4             | `molicel-p42a`   |
| Floatwheel — ADV2                        | 20S2P | Samsung 50S     | 10              | `samsung-50s`    |
| Floatwheel — Atom                        | 22S1P | Reliance RS50   | 5               | `generic-linear` |
| Fungineers — Funwheel X7                 | 20S2P | Molicel P28A    | 5.6             | `molicel-p28a`   |
| Fungineers — Funwheel X7S                | 20S2P | Molicel P30B    | 6               | `generic-linear` |
| Fungineers — Funwheel X7LR               | 20S2P | Samsung 50S     | 10              | `samsung-50s`    |
| Fungineers — Funwheel X7HV               | 32S1P | Molicel P45B    | 4.5             | `molicel-p45b`   |
| Onewheel — GT S-Series                   | 27S1P | Molicel P45B    | 4.5             | `molicel-p45b`   |
| Onewheel — GT                            | 18S2P | Molicel P42A    | 8.4             | `molicel-p42a`   |
| Onewheel — XR Classic                    | 18S2P | Molicel P30B    | 6               | `generic-linear` |
| Onewheel — XR / PintX                    | 15S2P | VTC6 / 30Q      | 6               | `generic-linear` |
| Onewheel — Pint                          | 15S1P | Murata VTC5D    | 2.8             | `generic-linear` |
| Indy Speed Control — 32s1p               | 32S1P | Molicel P45B    | 4.5             | `molicel-p45b`   |
| Indy Speed Control — 30s1p               | 30S1P | Molicel P45B    | 4.5             | `molicel-p45b`   |
| Indy Speed Control — 20s2p GT Pack       | 20S2P | Molicel P45B    | 9               | `molicel-p45b`   |
| Indy Speed Control — 20s2p XR Split Pack | 20S2P | Molicel P45B    | 9               | `molicel-p45b`   |
| Indy Speed Control — 20s2p Battery Kit   | 20S2P | Molicel P30B    | 6               | `generic-linear` |
| Indy Speed Control — XRV PnP Kit         | 18S2P | Molicel P45B    | 9               | `molicel-p45b`   |
| Indy Speed Control — 18s2p               | 18S2P | Molicel P45B    | 9               | `molicel-p45b`   |
| Indy Speed Control — Smart Link 32s2p    | 32S2P | Reliance RS50   | 10              | `generic-linear` |
| Indy Speed Control — Smart Link 30s2p    | 30S2P | Reliance RS50   | 10              | `generic-linear` |
| The Board Garage — FAS Pack              | 30S1P | Molicel P50B    | 5               | `generic-linear` |
| The Board Garage — '777' VOW GT          | 24S2P | Molicel P45B    | 9               | `molicel-p45b`   |
| The Board Garage — GT ME4T Pack          | 20S2P | Samsung 50S     | 10              | `samsung-50s`    |
| The Board Garage — Extra Regular Pack    | 20S2P | Molicel P30B    | 6               | `generic-linear` |
| The Board Garage — Regular Pack          | 20S2P | Molicel P28A    | 5.6             | `molicel-p28a`   |
| The Board Garage — 19s2p                 | 19S2P | Molicel P45B    | 9               | `molicel-p45b`   |
| The Board Garage — XR ME4T Pack          | 18S2P | Samsung 50S     | 10              | `samsung-50s`    |
| The Board Garage — Power Pack            | 18S2P | Molicel P45B    | 9               | `molicel-p45b`   |
| Z Battery Solutions — Speed Pack         | 20S2P | Molicel P28A    | 5.6             | `molicel-p28a`   |
| Z Battery Solutions — Torque Pack [P42A] | 18S2P | Molicel P42A    | 8.4             | `molicel-p42a`   |
| Z Battery Solutions — Torque Pack [P45B] | 18S2P | Molicel P45B    | 9               | `molicel-p45b`   |
| Chi — CHI-VE XR/GT 32.1                  | 32S1P | Molicel P50B    | 5               | `generic-linear` |
| Chi — CHI-VE GT 20.2                     | 20S2P | Molicel P50B    | 10              | `generic-linear` |
| Chi — CHI-VE XR 20.2                     | 20S2P | Molicel P30B    | 6               | `generic-linear` |
| Chi — CHI-VE XR 18.2                     | 18S2P | Molicel P50B    | 10              | `generic-linear` |
| Chi — CHI-VE PintX/XR 24.1               | 24S1P | Molicel P50B    | 5               | `generic-linear` |
| Chi — CHI-VE Pint X 20.1                 | 20S1P | Molicel P50B    | 5               | `generic-linear` |
| Chi — CHI-VE Pint 20.1                   | 20S1P | Molicel P30B    | 3               | `generic-linear` |
| Chi — CBCSO                              | 20S2P | VTC6 / 30Q      | 6               | `generic-linear` |
| Chi — CBXR - Original                    | 15S3P | Molicel M35A    | 10.5            | `molicel-m35a`   |
| Chi — CBXR - VESC Power                  | 15S3P | Molicel P28A    | 8.4             | `molicel-p28a`   |
| Chi — CBXR Mini                          | 15S2P | Sanyo 20700B    | 8               | `generic-linear` |
| Chi — CBSP                               | 15S1P | Molicel P42A    | 4.2             | `molicel-p42a`   |
| Chi — Quart - Standard                   | 15S1P | Molicel P42A    | 4.2             | `molicel-p42a`   |
| Chi — Quart - S Edition                  | 15S1P | Samsung 50S     | 5               | `samsung-50s`    |

Add cell-specific alternatives confirmed by the manufacturer pages: Indy 18S2P P50B (10 Ah, generic), Chi 32.1 P45B (4.5 Ah, `molicel-p45b`) and Samsung 50S (5 Ah, `samsung-50s`). Add a CBXR Original Sanyo GA variant (15S3P, 10.5 Ah, generic), keeping it separate from M35A. These are explicit choices, not automatic cell detection. Unlisted variants use Custom. Exclude the researched 40S1P entry because it exceeds the existing 36S contract; this change does not broaden pack limits or support early LiFePO4 Onewheel chemistries with Li-ion curves. Keep source URL, retrieval date, and nominal-capacity derivation beside the implementation catalog as development provenance, not Basic UI copy. Test every row's bounds, unique identity, profile resolution, and nominal-capacity arithmetic.
