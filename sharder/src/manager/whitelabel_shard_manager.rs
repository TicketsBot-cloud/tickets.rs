use async_trait::async_trait;

use super::ShardManager;

use crate::gateway::payloads::Identify;
use crate::gateway::{Shard, ShardInfo};

use super::whitelabel_decision::{auth_failure_action, row_action, AuthFailureAction, RowAction};
use crate::gateway::event_forwarding::EventForwarder;
use crate::{
    Config, GatewayError, InternalCommand, RedisSessionStore, SessionData, SessionStore,
    ShardIdentifier,
};
use common::token_change;
use common::token_crypto::TokenKeyring;
use database::{Database, WhitelabelBot};
use deadpool_redis::redis;
use deadpool_redis::Pool;
use futures::{FutureExt, StreamExt};
use model::user::{ActivityType, StatusType, StatusUpdate};
use model::Snowflake;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, Notify, RwLock};
use tokio::time::{sleep, timeout};
use tracing::{error, info, warn};

const TOKEN_RESOLVE_BACKOFF: Duration = Duration::from_secs(30);
const RECHECK_BACKOFF_BASE: Duration = Duration::from_secs(1);
const RECHECK_BACKOFF_MAX: Duration = Duration::from_secs(60);

pub struct WhitelabelShardManager<T: EventForwarder> {
    config: Arc<Config>,
    database: Arc<Database>,
    redis: Arc<Pool>,
    session_store: RedisSessionStore,
    event_forwarder: Arc<T>,
    shutdown_tx: broadcast::Sender<mpsc::Sender<(ShardIdentifier, Option<SessionData>)>>,
    shard_command_channels: RwLock<HashMap<Snowflake, mpsc::Sender<InternalCommand>>>,
    keyring: Arc<TokenKeyring>,
    running: Arc<Running>,
}

/// Bots that already have a connect loop, so a token change signal cannot
/// start a second shard for the same bot. Each entry carries the bot's
/// reconnect signal, which exists from the moment the bot is registered, so a
/// nudge can never find the bot running but have nowhere to go.
#[derive(Default)]
struct Running {
    inner: parking_lot::Mutex<HashMap<Snowflake, Arc<Notify>>>,
}

impl Running {
    /// Returns the bot's reconnect signal, or None when a loop already exists.
    fn register(&self, bot_id: Snowflake) -> Option<Arc<Notify>> {
        match self.inner.lock().entry(bot_id) {
            std::collections::hash_map::Entry::Occupied(_) => None,
            std::collections::hash_map::Entry::Vacant(v) => {
                let notify = Arc::new(Notify::new());
                v.insert(Arc::clone(&notify));
                Some(notify)
            }
        }
    }

    /// Stores a reconnect request for a running bot. Notify keeps a single
    /// permit, so any number of requests before the loop looks coalesce into
    /// one reconnect, and a request made before the loop is listening is kept.
    fn nudge(&self, bot_id: Snowflake) -> bool {
        match self.inner.lock().get(&bot_id) {
            Some(notify) => {
                notify.notify_one();
                true
            }
            None => false,
        }
    }

    fn contains(&self, bot_id: Snowflake) -> bool {
        self.inner.lock().contains_key(&bot_id)
    }

    fn remove(&self, bot_id: Snowflake) {
        self.inner.lock().remove(&bot_id);
    }
}

/// Clears a bot from the running set on every exit path, including a panic in
/// the connect loop, so the bot can always be started again.
struct RunningGuard {
    running: Arc<Running>,
    bot_id: Snowflake,
}

impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.running.remove(self.bot_id);
    }
}

#[derive(Debug, PartialEq, Eq)]
enum RecheckAction {
    Restart,
    Stop,
    Retry,
}

/// A missing row is a deliberate delete or an authentication removal and must
/// stay stopped. A database error says nothing about the row, so it is retried.
fn recheck_action<T, E>(row: &Result<Option<T>, E>) -> RecheckAction {
    match row {
        Ok(Some(_)) => RecheckAction::Restart,
        Ok(None) => RecheckAction::Stop,
        Err(_) => RecheckAction::Retry,
    }
}

/// Doubles from the base delay up to a fixed ceiling. Attempt 0 is the first
/// retry. Also used between restarts after a panic so a bot that panics
/// immediately cannot spin.
fn recheck_backoff(attempt: u32) -> Duration {
    RECHECK_BACKOFF_BASE
        .saturating_mul(1u32 << attempt.min(16))
        .min(RECHECK_BACKOFF_MAX)
}

/// mpsc::channel panics on a zero capacity, and no shard may be subscribed
/// (every bot can be in decrypt backoff), so the capacity is clamped.
fn shutdown_channel_capacity(receivers: usize) -> usize {
    receivers.max(1)
}

impl<T: EventForwarder> WhitelabelShardManager<T> {
    pub fn new(
        config: Config,
        database: Arc<Database>,
        redis: Arc<Pool>,
        session_store: RedisSessionStore,
        event_forwarder: Arc<T>,
        keyring: Arc<TokenKeyring>,
    ) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        WhitelabelShardManager {
            config: Arc::new(config),
            database,
            redis,
            session_store,
            event_forwarder,
            shutdown_tx,
            shard_command_channels: RwLock::new(HashMap::new()),
            keyring,
            running: Arc::new(Running::default()),
        }
    }

    async fn connect_bot(self: Arc<Self>, bot: WhitelabelBot) {
        let bot_id = Snowflake(bot.bot_id as u64);

        // A loop that already exists re-reads the row on every reconnect, so
        // asking it to reconnect is enough.
        if !self.start_loop(bot_id) {
            self.running.nudge(bot_id);
        }
    }

    /// Returns false when a loop already exists for the bot.
    fn start_loop(self: &Arc<Self>, bot_id: Snowflake) -> bool {
        let Some(notify) = self.running.register(bot_id) else {
            return false;
        };

        let sm = Arc::clone(self);
        tokio::spawn(async move { sm.supervise(bot_id, notify).await });

        true
    }

    /// Runs the connect loop in an inner task so a panic still reaches the
    /// re-check, then decides from the row whether to run it again. The
    /// running entry is cleared before the re-check so a signal arriving after
    /// the row was read starts its own loop instead of being dropped.
    async fn supervise(self: Arc<Self>, bot_id: Snowflake, mut notify: Arc<Notify>) {
        let mut panics: u32 = 0;

        loop {
            let guard = RunningGuard {
                running: Arc::clone(&self.running),
                bot_id,
            };
            let worker = Arc::clone(&self);
            let result = tokio::spawn(async move { worker.run_bot(bot_id, notify).await }).await;
            drop(guard);

            if let Err(e) = result {
                error!(bot_id = %bot_id, error = %e, "Bot loop terminated abnormally");
                sleep(recheck_backoff(panics)).await;
                panics = panics.saturating_add(1);
            } else {
                panics = 0;
            }

            let mut attempt: u32 = 0;
            loop {
                // Another signal may have started a loop while this one waited.
                if self.running.contains(bot_id) {
                    return;
                }

                let row = self.database.whitelabel.get_bot_by_id(bot_id).await;
                match recheck_action(&row) {
                    RecheckAction::Stop => return,
                    RecheckAction::Restart => break,
                    RecheckAction::Retry => {
                        if let Err(e) = &row {
                            error!(bot_id = %bot_id, error = %e, "Failed to re-check bot row after loop exit, will retry");
                        }
                        sleep(recheck_backoff(attempt)).await;
                        attempt = attempt.saturating_add(1);
                    }
                }
            }

            info!(bot_id = %bot_id, "Bot row present after loop exit, restarting");
            match self.running.register(bot_id) {
                Some(n) => notify = n,
                None => return,
            }
        }
    }

    async fn run_bot(self: &Arc<Self>, bot_id: Snowflake, reconnect: Arc<Notify>) {
        let sm = Arc::clone(self);

        // retrieve bot status; None means the owner set no custom status, so the bot
        // connects with no activity displayed.
        let status: Option<(String, ActivityType)> =
            match self.database.whitelabel_status.get(bot_id).await {
                Ok((status, Some(status_type))) => Some((status, status_type)),
                Ok((_, None)) => {
                    warn!(bot_id = %bot_id, "Bot has invalid status type");
                    None
                }
                Err(database::sqlx::Error::RowNotFound) => None,
                Err(e) => {
                    error!(bot_id = %bot_id, error = %e, "Error occurred while retrieving status");
                    None
                }
            };

        let mut resume_data = match sm.session_store.get(bot_id.0).await {
            Ok(data) => data,
            Err(e) => {
                error!(bot_id = %bot_id, error = %e, "Failed to fetch session data");
                None
            }
        };

        loop {
            // A request made before this point is covered by the row read
            // below, so the stored permit is dropped. A request made after it
            // is delivered to the shard by the forwarder started further down.
            let _ = reconnect.notified().now_or_never();

            // The row is re-read on every iteration so a rotated token is
            // picked up, and the same row is the snapshot used for any
            // conditional delete after an authentication failure.
            let current = match sm.database.whitelabel.get_bot_by_id(bot_id).await {
                Ok(Some(current)) => current,
                Ok(None) => {
                    info!(bot_id = %bot_id, "Bot no longer exists, stopping");
                    break;
                }
                Err(e) => {
                    error!(bot_id = %bot_id, error = %e, "Error checking if bot still exists in the database");
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
            };

            let token = match row_action(&sm.keyring, &current) {
                RowAction::Connect(token) => token,
                RowAction::BackOff(e) => {
                    error!(bot_id = %bot_id, error = %e, "Could not resolve bot token, will retry");
                    sleep(TOKEN_RESOLVE_BACKOFF).await;
                    continue;
                }
            };

            let shard_info = ShardInfo::new(0, 1);
            let presence = match &status {
                Some((text, activity_type)) => {
                    StatusUpdate::new(*activity_type, text.clone(), StatusType::Online)
                }
                None => StatusUpdate::without_activity(StatusType::Online),
            };
            let identify = Identify::new(
                token,
                None,
                shard_info,
                Some(presence),
                super::get_intents(),
            );

            let (command_tx, command_rx) = mpsc::channel(4);

            // Waits for the reconnect signal and then blocks on the channel
            // until the shard has room, so the request cannot be dropped by a
            // full queue. One task per connection, aborted when it ends.
            let forwarder = {
                let tx = command_tx.clone();
                let reconnect = Arc::clone(&reconnect);
                tokio::spawn(async move {
                    reconnect.notified().await;
                    let _ = tx.send(InternalCommand::Shutdown).await;
                })
            };

            let shard = Shard::new(
                self.config.clone(),
                identify,
                1,
                Arc::clone(&self.redis),
                bot_id,
                Arc::clone(&self.event_forwarder),
                None,
                self.shutdown_tx.subscribe(),
                Arc::clone(&self.database),
                command_rx,
            );

            info!(bot_id = %bot_id, "Starting");

            {
                sm.shard_command_channels
                    .write()
                    .await
                    .insert(bot_id, command_tx);
            }

            let res = shard.connect(resume_data.clone()).await;
            forwarder.abort();

            {
                sm.shard_command_channels.write().await.remove(&bot_id);
            }

            match res {
                Ok(session_data) => {
                    resume_data = session_data;
                    self.log_for_bot(bot_id, "Exited with Ok");
                }
                Err(GatewayError::AuthenticationError { data, .. }) => {
                    let deleted = self.database.whitelabel.delete_if_unchanged(&current).await;

                    match auth_failure_action(&deleted) {
                        AuthFailureAction::RecordAndStop => {
                            self.log_err_for_bot(
                                bot_id,
                                "Exited with authentication error, removed bot",
                                &GatewayError::custom(&data.error),
                            );

                            if let Err(e) = self
                                .database
                                .whitelabel_errors
                                .append(Snowflake(current.user_id as u64), data.error)
                                .await
                            {
                                self.log_err_for_bot(
                                    bot_id,
                                    "Error occurred while recording error to database",
                                    &GatewayError::DatabaseError(e),
                                );
                            }

                            break;
                        }
                        AuthFailureAction::Reconnect => {
                            info!(bot_id = %bot_id, "Credentials changed during connection, reconnecting with new credentials");
                            resume_data = None;
                            continue;
                        }
                        AuthFailureAction::RetryLater => {
                            if let Err(e) = deleted {
                                self.log_err_for_bot(
                                    bot_id,
                                    "Error occurred while deleting bot",
                                    &GatewayError::DatabaseError(e),
                                );
                            }
                            sleep(Duration::from_secs(10)).await;
                            continue;
                        }
                    }
                }
                Err(e) => self.log_err_for_bot(bot_id, "Exited with error", &e),
            }

            sleep(Duration::from_millis(500)).await;
        }
    }

    pub async fn listen_status_updates(self: Arc<Self>) -> Result<(), GatewayError> {
        let database = Arc::clone(&self.database);

        let mut conn = redis::Client::open(self.config.get_redis_uri())
            .unwrap()
            .get_async_connection()
            .await?
            .into_pubsub();

        conn.subscribe(common::status_updates::KEY).await?;

        tokio::spawn(async move {
            let mut stream = conn.on_message();

            while let Some(m) = stream.next().await {
                match m.get_payload::<String>().map(|s| s.parse::<Snowflake>()) {
                    Ok(Ok(bot_id)) => {
                        let tx = self
                            .shard_command_channels
                            .read()
                            .await
                            .get(&bot_id)
                            .cloned();
                        if let Some(tx) = tx {
                            println!("[RPC] Received status update payload for bot {bot_id}");

                            // retrieve new status; if the row is gone (status cleared),
                            // send a presence with no activity so the clear propagates to
                            // Discord instead of being a no-op
                            let new_status = match database.whitelabel_status.get(bot_id).await {
                                Ok((status, Some(status_type))) => {
                                    Some(StatusUpdate::new(status_type, status, StatusType::Online))
                                }
                                Err(database::sqlx::Error::RowNotFound) => {
                                    Some(StatusUpdate::without_activity(StatusType::Online))
                                }
                                Ok((_, None)) => {
                                    eprintln!("Bot {} has invalid status type", bot_id);
                                    None
                                }
                                Err(e) => {
                                    eprintln!("Error retrieving status from db: {}", e);
                                    None
                                }
                            };

                            if let Some(status) = new_status {
                                let cmd = InternalCommand::StatusUpdate { status };

                                if let Err(e) = tx.send(cmd).await {
                                    eprintln!(
                                        "An error occured while updating status for {}: {}",
                                        bot_id, e
                                    );
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => eprintln!("An error occured while reading status updates: {}", e),
                    Err(e) => eprintln!("An error occured while reading status updates: {}", e),
                }
            }
        });

        Ok(())
    }

    pub async fn listen_new_tokens(self: Arc<Self>) -> Result<(), GatewayError> {
        let mut conn = redis::Client::open(self.config.get_redis_uri())
            .unwrap()
            .get_async_connection()
            .await?
            .into_pubsub();

        conn.subscribe(common::token_change::KEY).await?;

        tokio::spawn(async move {
            let mut stream = conn.on_message();

            while let Some(m) = stream.next().await {
                let manager = Arc::clone(&self);

                match serde_json::from_slice::<token_change::Payload>(m.get_payload_bytes()) {
                    Ok(payload) => {
                        // start new bot; new_id 0 comes from a first-bot rollback and has nothing to start
                        if payload.new_id.0 != 0
                            && payload.new_id.0 % (manager.config.sharder_total as u64)
                            == manager.config.sharder_id as u64
                        {
                            match self.database.whitelabel.get_bot_by_id(payload.new_id).await {
                                Ok(Some(bot)) => {
                                    manager.connect_bot(bot).await;
                                }
                                Ok(None) => {
                                    eprintln!("Couldn't find row for bot {}", payload.new_id)
                                }
                                Err(e) => eprintln!("Error retrieving bot from DB: {}", e),
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("An error occurred while decoding new token payload: {}", e)
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn listen_delete(self: Arc<Self>) -> Result<(), GatewayError> {
        let mut conn = redis::Client::open(self.config.get_redis_uri())
            .unwrap()
            .get_async_connection()
            .await?
            .into_pubsub();

        conn.subscribe("tickets:whitelabeldelete").await?;

        let sm = Arc::clone(&self);
        tokio::spawn(async move {
            let mut stream = conn.on_message();

            while let Some(m) = stream.next().await {
                match m.get_payload::<String>().map(|s| s.parse::<Snowflake>()) {
                    Ok(Ok(bot_id)) => {
                        println!("[RPC] Received delete payload for {bot_id}");

                        // The nudge is held while the loop is between its row read
                        // and connecting, so a delete in that window is not lost.
                        sm.running.nudge(bot_id);
                    }
                    Ok(Err(e)) => {
                        eprintln!("An error occurred while reading delete payload: {}", e)
                    }
                    Err(e) => eprintln!("An error occurred while reading delete payload: {}", e),
                }
            }
        });

        Ok(())
    }

    pub fn log_for_bot(&self, bot_id: Snowflake, msg: impl Display) {
        info!(bot_id = %bot_id, "{msg}");
    }

    pub fn log_err_for_bot(&self, bot_id: Snowflake, msg: impl Display, err: &GatewayError) {
        error!(bot_id = %bot_id, error = %err, "{msg}");
    }
}

#[async_trait]
impl<T: EventForwarder> ShardManager for WhitelabelShardManager<T> {
    async fn connect(self: Arc<Self>) {
        // we should panic if we cant read db
        let bots = self
            .database
            .whitelabel
            .get_bots_by_sharder(self.config.sharder_total, self.config.sharder_id)
            .await
            .unwrap();

        for bot in bots {
            Arc::clone(&self).connect_bot(bot).await;
        }
    }

    async fn shutdown(self: Arc<Self>) {
        if self.shutdown_tx.receiver_count() == 0 {
            info!("No active shards to shut down");
            return;
        }

        let (tx, mut rx) =
            mpsc::channel(shutdown_channel_capacity(self.shutdown_tx.receiver_count()));

        // Every shard may have exited between the count and the send.
        let Ok(receivers) = self.shutdown_tx.send(tx) else {
            info!("No active shards to shut down");
            return;
        };

        let mut sessions = HashMap::new();
        for _ in 0..receivers {
            let (identifier, session_data) = match timeout(Duration::from_secs(30), rx.recv()).await
            {
                Ok(Some((identifier, Some(session_data)))) => (identifier, session_data),
                Ok(Some((identifier, None))) => {
                    let bot_id = identifier.bot_id;
                    info!(bot_id = %bot_id, "Shard sent None session data");
                    continue;
                }
                Ok(None) => {
                    warn!("Shutdown session data receiver is closed");
                    break;
                }
                Err(e) => {
                    warn!(error = %e, "Timeout while waiting for shard to shutdown");
                    break;
                }
            };

            sessions.insert(identifier.bot_id.0, session_data);
        }

        if let Err(e) = self.session_store.set_bulk(sessions).await {
            error!(error = %e, "Failed to save session data");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    fn running_with(id: u64) -> Arc<Running> {
        let running = Arc::new(Running::default());
        running.register(Snowflake(id)).unwrap();
        running
    }

    #[test]
    fn guard_removes_entry_on_drop() {
        let running = running_with(1);
        running.register(Snowflake(2)).unwrap();

        drop(RunningGuard {
            running: Arc::clone(&running),
            bot_id: Snowflake(1),
        });

        assert!(!running.contains(Snowflake(1)));
        assert!(running.contains(Snowflake(2)));
    }

    #[test]
    fn guard_removes_entry_on_panic() {
        let running = running_with(1);
        let guard = RunningGuard {
            running: Arc::clone(&running),
            bot_id: Snowflake(1),
        };

        let res = catch_unwind(AssertUnwindSafe(move || {
            let _guard = guard;
            panic!("run_bot failed");
        }));

        assert!(res.is_err());
        assert!(!running.contains(Snowflake(1)));
    }

    #[test]
    fn second_register_is_refused_until_removed() {
        let running = running_with(1);
        assert!(running.register(Snowflake(1)).is_none());
        running.remove(Snowflake(1));
        assert!(running.register(Snowflake(1)).is_some());
    }

    #[test]
    fn nudge_for_unknown_bot_reports_not_running() {
        let running = Running::default();
        assert!(!running.nudge(Snowflake(9)));
    }

    #[tokio::test]
    async fn nudge_before_listener_is_not_lost() {
        let running = Running::default();
        let notify = running.register(Snowflake(1)).unwrap();

        assert!(running.nudge(Snowflake(1)));

        timeout(Duration::from_secs(1), notify.notified())
            .await
            .expect("nudge sent before anyone was listening must still wake the listener");
    }

    #[tokio::test]
    async fn many_nudges_coalesce_into_one_wakeup() {
        let running = Running::default();
        let notify = running.register(Snowflake(1)).unwrap();

        for _ in 0..50 {
            running.nudge(Snowflake(1));
        }

        timeout(Duration::from_secs(1), notify.notified())
            .await
            .unwrap();
        assert!(
            timeout(Duration::from_millis(50), notify.notified())
                .await
                .is_err(),
            "a consumed signal must not wake a second time"
        );
    }

    #[tokio::test]
    async fn stale_permit_is_dropped_by_now_or_never() {
        let running = Running::default();
        let notify = running.register(Snowflake(1)).unwrap();
        running.nudge(Snowflake(1));

        let _ = notify.notified().now_or_never();

        assert!(timeout(Duration::from_millis(50), notify.notified())
            .await
            .is_err());
    }

    #[test]
    fn recheck_decisions() {
        let some: Result<Option<u8>, &str> = Ok(Some(1));
        let none: Result<Option<u8>, &str> = Ok(None);
        let err: Result<Option<u8>, &str> = Err("db down");

        assert_eq!(recheck_action(&some), RecheckAction::Restart);
        assert_eq!(recheck_action(&none), RecheckAction::Stop);
        assert_eq!(recheck_action(&err), RecheckAction::Retry);
    }

    #[test]
    fn backoff_grows_and_is_capped() {
        assert_eq!(recheck_backoff(0), Duration::from_secs(1));
        assert_eq!(recheck_backoff(1), Duration::from_secs(2));
        assert_eq!(recheck_backoff(5), Duration::from_secs(32));
        assert_eq!(recheck_backoff(6), RECHECK_BACKOFF_MAX);
        assert_eq!(recheck_backoff(u32::MAX), RECHECK_BACKOFF_MAX);
        for attempt in 0..100 {
            assert!(recheck_backoff(attempt) >= RECHECK_BACKOFF_BASE);
            assert!(recheck_backoff(attempt) <= RECHECK_BACKOFF_MAX);
        }
    }

    #[test]
    fn shutdown_capacity_is_never_zero() {
        assert_eq!(shutdown_channel_capacity(0), 1);
        assert_eq!(shutdown_channel_capacity(1), 1);
        assert_eq!(shutdown_channel_capacity(7), 7);
    }

    #[test]
    fn zero_receiver_channel_construction_does_not_panic() {
        let (tx, _rx) = mpsc::channel::<u8>(shutdown_channel_capacity(0));
        assert!(tx.try_send(1).is_ok());
    }
}
