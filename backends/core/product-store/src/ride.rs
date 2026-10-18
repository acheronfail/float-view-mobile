//! A bounded durable recording owner, independent of connection and frontend lifetime.
use crate::{Error, ProductStore, Result, boards::timestamp};
use rusqlite::{OptionalExtension, params};

#[derive(Clone)]
pub struct RideCheckpoint {
    pub session_id: String,
    pub saved_device_id: String,
    pub controller_uuid: [u8; 12],
    pub window_minutes: u64,
    pub paused_at_ms: Option<i64>,
    pub resume_until_ms: Option<i64>,
}
impl ProductStore {
    pub fn ride_checkpoint(&self) -> Result<Option<RideCheckpoint>> {
        let row = self.catalog.query_row("SELECT session_id,saved_device_id,controller_uuid,window_minutes,paused_at_ms,resume_until_ms FROM ride_checkpoint WHERE singleton=1", [], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Vec<u8>>(2)?,r.get::<_,u64>(3)?,r.get::<_,Option<i64>>(4)?,r.get::<_,Option<i64>>(5)?))).optional()?;
        row.map(
            |(session_id, saved_device_id, uuid, window_minutes, paused_at_ms, resume_until_ms)| {
                if !(1..=1440).contains(&window_minutes)
                    || paused_at_ms.is_some() != resume_until_ms.is_some()
                    || paused_at_ms.zip(resume_until_ms).is_some_and(|(a, b)| {
                        a < 0
                            || b > 253_402_300_799_999
                            || b < a
                            || b - a != window_minutes as i64 * 60_000
                    })
                {
                    return Err(Error::Corrupt);
                }
                Ok(RideCheckpoint {
                    session_id,
                    saved_device_id,
                    controller_uuid: uuid.try_into().map_err(|_| Error::Corrupt)?,
                    window_minutes,
                    paused_at_ms,
                    resume_until_ms,
                })
            },
        )
        .transpose()
    }
    pub fn own_ride(
        &mut self,
        session_id: &str,
        saved_device_id: &str,
        uuid: [u8; 12],
        window_minutes: u64,
    ) -> Result<()> {
        if !(1..=1440).contains(&window_minutes)
            || self.session(session_id)?.device_history_id
                != self.board(saved_device_id)?.device_history_id
        {
            return Err(Error::InvalidInput);
        }
        self.catalog.execute("INSERT INTO ride_checkpoint(singleton,session_id,saved_device_id,controller_uuid,window_minutes) VALUES (1,?,?,?,?) ON CONFLICT(singleton) DO UPDATE SET window_minutes=excluded.window_minutes,paused_at_ms=NULL,resume_until_ms=NULL WHERE ride_checkpoint.session_id=excluded.session_id AND ride_checkpoint.controller_uuid=excluded.controller_uuid",params![session_id,saved_device_id,uuid.as_slice(),window_minutes])?;
        let checkpoint = self.ride_checkpoint()?.ok_or(Error::Corrupt)?;
        if checkpoint.session_id != session_id || checkpoint.controller_uuid != uuid {
            return Err(Error::ActiveSession);
        }
        Ok(())
    }
    pub fn pause_ride(&mut self, at_ms: i64) -> Result<()> {
        timestamp(at_ms)?;
        let minutes = self.settings()?.ride_resume_window_minutes;
        if at_ms > 253_402_300_799_999 - minutes as i64 * 60_000 {
            return Err(Error::InvalidInput);
        }
        self.catalog.execute("UPDATE ride_checkpoint SET window_minutes=?,paused_at_ms=?,resume_until_ms=?+?*60000 WHERE singleton=1 AND paused_at_ms IS NULL",params![minutes,at_ms,at_ms,minutes])?;
        Ok(())
    }
    pub(crate) fn recover_ride(&mut self, id: &str) -> Result<bool> {
        let Some(ride) = self.ride_checkpoint()?.filter(|r| r.session_id == id) else {
            return Ok(false);
        };
        if ride.paused_at_ms.is_none() {
            let mut reader = float_view_session_sqlite::SessionReader::open(self.session_path(id)?)
                .map_err(|_| Error::Corrupt)?;
            let marker = reader.marker().map_err(|_| Error::Corrupt)?;
            self.pause_ride(marker.committed_at_epoch_ms)?;
        }
        Ok(true)
    }
}
