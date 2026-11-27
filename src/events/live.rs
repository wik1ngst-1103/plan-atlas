//! Poll `PRAGMA data_version` and broadcast agent steps the hook process
//! wrote to the database.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;

use super::LiveEvent;
use crate::store::Store;

const POLL: Duration = Duration::from_millis(500);

/// Every 500 ms, when another connection has committed, broadcast each new
/// linked step as [`LiveEvent::AgentStep`]. Steps recorded before the
/// poller started are not replayed.
pub fn spawn_live_poller(store: Arc<Store>, tx: broadcast::Sender<LiveEvent>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let s = store.clone();
        let start = tokio::task::spawn_blocking(move || {
            Ok::<_, anyhow::Error>((s.data_version()?, s.max_event_id()?))
        })
        .await;
        let (mut version, mut last_id) = match start {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => {
                eprintln!("plan-atlas: live poller disabled: {e:#}");
                return;
            }
            Err(_) => return,
        };
        let mut tick = tokio::time::interval(POLL);
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let s = store.clone();
            let polled = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
                let v = s.data_version()?;
                if v == version {
                    return Ok((v, Vec::new()));
                }
                Ok((v, s.steps_after(last_id)?))
            })
            .await;
            let Ok(Ok((v, steps))) = polled else { continue };
            version = v;
            for st in steps {
                last_id = last_id.max(st.id);
                let _ = tx.send(LiveEvent::AgentStep {
                    plan_id: st.plan_id,
                    task_id: st.task_id,
                    agent: st.agent,
                });
            }
        }
    })
}
