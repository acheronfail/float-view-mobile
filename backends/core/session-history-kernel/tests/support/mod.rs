#![allow(dead_code)]

use float_view_session_history_kernel::{
    HistoryError, PruneBucketSelection, PruneConfig, PruneObservation, PruneSelector, RidePhase,
};

#[derive(Clone)]
pub struct Row {
    pub sequence: u64,
    pub elapsed_ms: u64,
    pub ride_phase: RidePhase,
    pub protected: bool,
    pub values: Vec<Option<f64>>,
}

pub fn data_lines(input: &str) -> impl Iterator<Item = &str> {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

pub fn parse_value(value: &str) -> Result<Option<f64>, &'static str> {
    if value == "-" {
        return Ok(None);
    }
    let value = value.parse::<f64>().map_err(|_| "invalid number")?;
    value
        .is_finite()
        .then_some(Some(value))
        .ok_or("non-finite number")
}

pub fn parse_phase(value: &str) -> Result<RidePhase, &'static str> {
    match value {
        "riding" => Ok(RidePhase::Riding),
        "other" => Ok(RidePhase::Other),
        _ => Err("invalid phase"),
    }
}

pub fn parse_series_rows(input: &str) -> Result<Vec<Row>, &'static str> {
    data_lines(input)
        .map(|line| {
            let columns: Vec<_> = line.split('\t').collect();
            if columns.len() != 6 {
                return Err("invalid series extent");
            }
            Ok(Row {
                sequence: columns[0].parse().map_err(|_| "invalid sequence")?,
                elapsed_ms: columns[1].parse().map_err(|_| "invalid elapsed")?,
                ride_phase: parse_phase(columns[2])?,
                protected: false,
                values: columns[3..]
                    .iter()
                    .map(|value| parse_value(value))
                    .collect::<Result<_, _>>()?,
            })
        })
        .collect()
}

pub fn parse_prune_rows(input: &str) -> Result<Vec<Row>, &'static str> {
    data_lines(input)
        .map(|line| {
            let columns: Vec<_> = line.split('\t').collect();
            if columns.len() != 7 {
                return Err("invalid prune extent");
            }
            Ok(Row {
                sequence: columns[0].parse().map_err(|_| "invalid sequence")?,
                elapsed_ms: columns[1].parse().map_err(|_| "invalid elapsed")?,
                ride_phase: parse_phase(columns[2])?,
                protected: columns[3].parse().map_err(|_| "invalid protection")?,
                values: columns[4..]
                    .iter()
                    .map(|value| parse_value(value))
                    .collect::<Result<_, _>>()?,
            })
        })
        .collect()
}

pub fn parse_dense_prune_rows(input: &str) -> Result<Vec<Row>, &'static str> {
    data_lines(input)
        .map(|line| {
            let columns: Vec<_> = line.split('\t').collect();
            if columns.len() != 12 {
                return Err("invalid dense prune extent");
            }
            Ok(Row {
                sequence: columns[0].parse().map_err(|_| "invalid sequence")?,
                elapsed_ms: columns[1].parse().map_err(|_| "invalid elapsed")?,
                ride_phase: RidePhase::Other,
                protected: false,
                values: columns[2..]
                    .iter()
                    .map(|value| parse_value(value))
                    .collect::<Result<_, _>>()?,
            })
        })
        .collect()
}

pub fn select(
    rows: &[Row],
    fields: &[&str],
    resolution_seconds: u64,
) -> Result<Vec<PruneBucketSelection>, HistoryError> {
    let mut selector = PruneSelector::new(PruneConfig {
        fields: fields.iter().map(|field| (*field).to_owned()).collect(),
        resolution_seconds,
    })?;
    let mut selections = Vec::new();
    for row in rows {
        if let Some(selection) = selector.append(PruneObservation {
            elapsed_ms: row.elapsed_ms,
            sequence: row.sequence,
            ride_phase: row.ride_phase,
            values: &row.values,
            protected: row.protected,
        })? {
            selections.push(selection);
        }
    }
    if let Some(selection) = selector.finish()? {
        selections.push(selection);
    }
    Ok(selections)
}
