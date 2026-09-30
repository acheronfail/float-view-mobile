//! Community voltage curves from acheronfail/floaty, BatteryDischargeCurvePicker.tsx.
//! Source: https://github.com/acheronfail/floaty/blob/c4c87fe26e91ef2345518c3e0a1adb941a211885/apps/app/components/BoardsPage/BatteryDischargeCurvePicker.tsx
//! Pinned source: c4c87fe26e91ef2345518c3e0a1adb941a211885 (tables updated 2025-05-18).
//! Approximate display state of charge, not a protection or remaining-range guarantee.
pub const CATALOG_VERSION: &str = "floaty-2025-05-18-v1";
pub struct Curve {
    pub current_a: f64,
    pub volts: [f64; 11],
}
pub struct Profile {
    pub id: &'static str,
    pub label: &'static str,
    pub curves: &'static [Curve],
}
pub static PROFILES: &[Profile] = &[
    Profile {
        id: "molicel-p42a",
        label: "Molicel P42A",
        curves: &[
            Curve {
                current_a: 0.0,
                volts: [
                    4.2, 4.05, 3.91, 3.83, 3.74, 3.65, 3.57, 3.48, 3.38, 3.0, 2.7,
                ],
            },
            Curve {
                current_a: 20.0,
                volts: [
                    4.2, 3.73, 3.69, 3.61, 3.53, 3.44, 3.35, 3.27, 3.18, 3.0, 2.7,
                ],
            },
        ],
    },
    Profile {
        id: "molicel-p45b",
        label: "Molicel P45B",
        curves: &[
            Curve {
                current_a: 0.0,
                volts: [
                    4.2, 4.08, 4.0, 3.91, 3.82, 3.73, 3.64, 3.56, 3.45, 3.23, 2.7,
                ],
            },
            Curve {
                current_a: 20.0,
                volts: [
                    4.2, 3.78, 3.73, 3.65, 3.57, 3.48, 3.38, 3.3, 3.194, 2.99, 2.7,
                ],
            },
        ],
    },
    Profile {
        id: "molicel-p28a",
        label: "Molicel P28A",
        curves: &[
            Curve {
                current_a: 0.0,
                volts: [
                    4.2, 4.09, 3.98, 3.88, 3.77, 3.69, 3.63, 3.55, 3.45, 3.24, 2.7,
                ],
            },
            Curve {
                current_a: 20.0,
                volts: [4.2, 3.67, 3.6, 3.51, 3.41, 3.33, 3.26, 3.18, 3.1, 2.91, 2.7],
            },
        ],
    },
    Profile {
        id: "molicel-m35a",
        label: "Molicel M35A",
        curves: &[
            Curve {
                current_a: 0.0,
                volts: [
                    4.2, 4.03, 3.97, 3.87, 3.79, 3.68, 3.57, 3.47, 3.32, 3.11, 2.7,
                ],
            },
            Curve {
                current_a: 10.0,
                volts: [
                    4.2, 3.62, 3.57, 3.48, 3.38, 3.29, 3.2, 3.11, 3.01, 2.87, 2.7,
                ],
            },
        ],
    },
    Profile {
        id: "dg-01-dg-40",
        label: "DG-01/DG-40",
        curves: &[Curve {
            current_a: 0.0,
            volts: [
                4.2, 4.02, 3.91, 3.83, 3.75, 3.61, 3.49, 3.35, 3.17, 2.81, 2.7,
            ],
        }],
    },
    Profile {
        id: "samsung-50s",
        label: "Samsung 50S",
        curves: &[
            Curve {
                current_a: 0.0,
                volts: [4.2, 3.95, 3.9, 3.78, 3.7, 3.58, 3.5, 3.4, 3.25, 3.0, 2.7],
            },
            Curve {
                current_a: 20.0,
                volts: [4.2, 3.72, 3.65, 3.6, 3.5, 3.4, 3.32, 3.2, 3.1, 2.9, 2.7],
            },
        ],
    },
    Profile {
        id: "generic-linear",
        label: "Generic linear (approximate)",
        curves: &[Curve {
            current_a: 0.0,
            volts: [4.2, 4.05, 3.9, 3.75, 3.6, 3.45, 3.3, 3.15, 3.0, 2.85, 2.7],
        }],
    },
];
pub fn profile(id: &str) -> Option<&'static Profile> {
    PROFILES.iter().find(|p| p.id == id)
}
