use proj_core::{Bell, Row};
use serde::{ser::SerializeSeq, Serializer};

/// Required so that folding params default to open
#[inline]
pub fn get_true() -> bool {
    true
}

/// Required so that we can omit `"is_ruleoff": false` when serialising
#[inline]
pub fn is_false(b: &bool) -> bool {
    !b
}

/// Required so that we can omit `"is_proved": true` when serialising
#[inline]
pub fn is_true(b: &bool) -> bool {
    !b
}

/// Required so that we can omit `"music_highlights": [[], [], [], ...]` when serialising (to save
/// memory space and also improve serialisation/deserialisation time).
pub fn is_all_empty(vs: &[Vec<usize>]) -> bool {
    vs.iter().all(Vec::is_empty)
}

/// Custom serialiser to serialise `Option<Vec<Row>>` into `[[<bell-index>]]`.  This way, we don't have to
/// mutilate our own data structures to get nice serialisation.
pub fn ser_opt_rows<S: Serializer>(rows: &Option<Vec<Row>>, s: S) -> Result<S::Ok, S::Error> {
    ser_rows(rows.as_ref().unwrap(), s)
}

/// Custom serialiser to serialise `[Row]` into `[[<bell-index>]]`.  This way, we don't have to
/// mutilate our own data structures to get nice serialisation.
pub fn ser_rows<S: Serializer>(rows: &[Row], s: S) -> Result<S::Ok, S::Error> {
    let mut seq_ser = s.serialize_seq(Some(rows.len()))?;
    for r in rows {
        seq_ser.serialize_element(&r.bells().map(Bell::index).collect::<Vec<_>>())?;
    }
    seq_ser.end()
}
