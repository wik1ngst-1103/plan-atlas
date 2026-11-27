//! Poll `PRAGMA data_version` and broadcast what the hook process wrote to
//! the database: agent steps, changes to the live-agent set, plus hourly
//! pruning of old events.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time::{Instant, MissedTickBehavior};

use super::LiveEvent;
use crate::store::{LiveTask, Store};

/// Events (and session links) older than this are deleted.
pub const EVENT_RETENTION_DAYS: u32 = 7;

#[derive(Debug, Clone, Copy)]
pub struct PollerConfig {
    /// How often `data_version` is checked.
    pub poll: Duration,
    /// While agents are live, how often the live set is recomputed without any
    /// new write (so idle agents expire from the UI).
    pub activity_every: Duration,
    /// How often old events are pruned.
    pub prune_every: Duration,
}

impl Default for PollerConfig {
    fn default() -> Self {
        PollerConfig {
            poll: Duration::from_millis(500),
            activity_every: Duration::from_secs(15),
            prune_every: Duration::from_secs(3600),
        }
    }
}

/// Identity of the live set: which agents are live on which tasks.
type LiveKey = Vec<(String, String, String)>;

fn live_key(live: &[LiveTask]) -> LiveKey {
    live.iter()
        .flat_map(|t| {
            t.agents
                .iter()
                .map(|a| (t.plan_id.clone(), t.task_id.clone(), a.agent_id.clone()))
        })
        .collect()
}

/// [`spawn_live_poller_with`] using the default timings.
pub fn spawn_live_poller(store: Arc<Store>, tx: broadcast::Sender<LiveEvent>) -> JoinHandle<()> {
    spawn_live_poller_with(store, tx, PollerConfig::default())
}

/// Every `poll`, when another connection has committed, broadcast each new
/// linked step as [`LiveEvent::AgentStep`] and, if the set of live agents
/// changed, [`LiveEvent::ActivityChanged`]. While that set is non-empty it is
/// also recomputed every `activity_every`, so agents that went idle drop out.
/// Steps recorded before the poller started are not replayed. Old events are
/// pruned every `prune_every`.
pub fn spawn_live_poller_with(
    store: Arc<Store>,
    tx: broadcast::Sender<LiveEvent>,
    cfg: PollerConfig,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let s = store.clone();
        let start = tokio::task::spawn_blocking(move || {
            Ok::<_, anyhow::Error>((s.data_version()?, s.max_event_id()?, s.live_agents()?))
        })
        .await;
        let (mut version, mut last_id, mut live) = match start {
            Ok(Ok((v, id, live))) => (v, id, live_key(&live)),
            Ok(Err(e)) => {
                eprintln!("plan-atlas: live poller disabled: {e:#}");
                return;
            }
            Err(_) => return,
        };
        let mut tick = tokio::time::interval(cfg.poll);
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut live_at = Instant::now();
        let mut pruned_at = Instant::now();
        loop {
            tick.tick().await;
            let now = Instant::now();
            let prune = now.duration_since(pruned_at) >= cfg.prune_every;
            let recheck = !live.is_empty() && now.duration_since(live_at) >= cfg.activity_every;
            let s = store.clone();
            let polled = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
                if prune && let Err(e) = s.prune_events(EVENT_RETENTION_DAYS) {
                    eprintln!("plan-atlas: warning: pruning old events failed: {e:#}");
                }
                let v = s.data_version()?;
                let changed = v != version;
                let steps = if changed {
                    s.steps_after(last_id)?
                } else {
                    Vec::new()
                };
                let live = if changed || recheck {
                    Some(live_key(&s.live_agents()?))
                } else {
                    None
                };
                Ok((v, steps, live))
            })
            .await;
            if prune {
                pruned_at = now;
            }
            let Ok(Ok((v, steps, new_live))) = polled else {
                continue;
            };
            version = v;
            for st in steps {
                last_id = last_id.max(st.id);
                let _ = tx.send(LiveEvent::AgentStep {
                    plan_id: st.plan_id,
                    task_id: st.task_id,
                    agent: st.agent,
                });
            }
            if let Some(new_live) = new_live {
                live_at = now;
                if new_live != live {
                    live = new_live;
                    let _ = tx.send(LiveEvent::ActivityChanged);
                }
            }
        }
    })
}
