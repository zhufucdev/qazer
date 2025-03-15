use crate::bot::change::StatusChange;
use crate::bot::clients::ClientCollection;
use crate::repo::model::{AccountIndex, Repository};
use crate::tencent::model::ApplicationProgress;
use crate::tencent::ClientResult;
use crate::watch::Watcher;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{ChatId, Message, Requester, ResponseResult, UserId};
use teloxide::types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup};
use teloxide::Bot;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;

type TClient = crate::tencent::Client;

pub struct Basic<Tokens, APs, Intervals>
where
    Tokens: Repository<String>,
    APs: Repository<ApplicationProgress>,
{
    tokens: Tokens,
    cache: Arc<Mutex<APs>>,
    intervals: Intervals,
    clients: Arc<Mutex<ClientCollection>>,
    ic_tx: Sender<AccountIndex>,
}

impl<R, T, I> Basic<R, T, I>
where
    R: Repository<String>,
    T: Repository<ApplicationProgress>,
    I: Repository<Duration>,
    <R as Repository<String>>::Err: Debug,
    <T as Repository<ApplicationProgress>>::Err: Debug,
    <I as Repository<Duration>>::Err: Debug,
{
    pub fn new(
        tokens: R,
        cache: Arc<Mutex<T>>,
        intervals: I,
        clients: Arc<Mutex<ClientCollection>>,
        interval_change_tx: Sender<AccountIndex>,
    ) -> Basic<R, T, I> {
        Self {
            tokens,
            cache,
            intervals,
            clients,
            ic_tx: interval_change_tx,
        }
    }

    async fn update_status(&self, account: AccountIndex, value: &ApplicationProgress) {
        self.cache
            .lock()
            .await
            .put(account, value.clone())
            .expect("Failed to update database")
    }

    pub async fn get(&mut self, bot: &Bot, msg: Message) -> ResponseResult<()> {
        match msg.from {
            None => {
                bot.send_message(msg.chat.id, "No user info is bound to this context.")
                    .await?;
            }
            Some(user) => {
                let acc_idx = user.id.0;
                match self.clients.lock().await.get(acc_idx).await {
                    None => {
                        bot.send_message(msg.chat.id, "No token associated with current context. Use the /signin command to get started.").await?;
                    }
                    Some(client) => {
                        match client.lock().await.get_application_progress().await {
                            Ok(ap) => {
                                self.update_status(acc_idx, &ap).await;
                                match ap.get_current_step() {
                                    Ok(Some(step)) => bot.send_message(msg.chat.id, format!("Current progress is {:?}.", step)).await?,
                                    Ok(None) => bot.send_message(msg.chat.id, "Current progress is empty or doesn't make sense. Check the web page for more info.").await?,
                                    Err(err) => bot.send_message(msg.chat.id, format!("Fetch succeeded but can't make sense of the result because {}", err)).await?
                                }
                            }
                            Err(e) => {
                                bot.send_message(msg.chat.id, format!("Fetch failed because {}", e))
                                    .await?
                            }
                        };
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn signin(&mut self, bot: &Bot, msg: Message, token: String) -> ResponseResult<()> {
        if token.is_empty() {
            bot.send_message(msg.chat.id, "Empty token. This operation has no effect.")
                .await?;
            return Ok(());
        }
        let new_client = TClient::with_token(&token);
        match new_client.get_application_progress().await {
            Ok(ap) => match msg.from {
                None => {
                    bot.send_message(msg.chat.id, "No user info bound to this context.")
                        .await?;
                }
                Some(user) => {
                    let acc_idx = user.id.0;
                    self.update_status(acc_idx, &ap).await;
                    self.clients.lock().await.insert(acc_idx, new_client).await;
                    match self.tokens.put(acc_idx, token) {
                        Ok(_) => {
                            bot.send_message(msg.chat.id, "Token has been updated.")
                                .await?;
                            bot.edit_message_text(msg.chat.id, msg.id, "/signin")
                                .await?;
                        }
                        Err(e) => {
                            eprintln!(
                                "Error while inserting token, user id = {}. {:?}",
                                acc_idx, e
                            );
                            bot.send_message(
                                msg.chat.id,
                                format!(
                                    "Error updating database. {}",
                                    get_contact_admin_text(acc_idx)
                                ),
                            )
                            .await?;
                        }
                    }
                }
            },
            Err(e) => {
                bot.send_message(msg.chat.id, format!("Invalid token: {}", e))
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn interval(&mut self, bot: &Bot, msg: Message) -> ResponseResult<()> {
        match msg.from {
            None => {
                send_no_user(msg.chat.id, bot).await?;
            }
            Some(_) => {
                bot.send_message(msg.chat.id, "Choose one of the following intervals.")
                    .reply_markup(make_interval_keyboard())
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn interval_callback_handler(
        &mut self,
        bot: &Bot,
        query: CallbackQuery,
    ) -> ResponseResult<()> {
        match query.data {
            None => {
                bot.send_message(query.from.id, "Your invalid click has no effect.")
                    .await?;
            }
            Some(ref min) => match min.parse::<u32>() {
                Ok(min) => {
                    let acc = &query.from.id.0;
                    let result: String = if let Err(e) = self
                        .intervals
                        .put(*acc, Duration::from_secs(min as u64 * 60))
                    {
                        eprintln!(
                            "Error while updating interval database, user id = {}: {:?}",
                            acc, e
                        );
                        format!(
                            "Failed to update database. {}",
                            get_contact_admin_text(*acc)
                        )
                    } else {
                        self.ic_tx.send(*acc).await.expect(
                            format!("Failed to notify interval change, user id = {}", *acc)
                                .as_str(),
                        );
                        format!("Polling interval has been updated to {min}min.")
                    };
                    bot.answer_callback_query(&query.id).await?;
                    if let Some(msg) = query.regular_message() {
                        bot.edit_message_text(msg.chat.id, msg.id, result).await?;
                    } else if let Some(id) = query.inline_message_id {
                        bot.edit_message_text_inline(id, result).await?;
                    }
                }
                Err(_) => {
                    bot.send_message(
                        query.from.id,
                        "Your message carries invalid data thus has no effect.",
                    )
                    .await?;
                }
            },
        }
        Ok(())
    }

    pub async fn signout(&mut self, bot: &Bot, msg: Message) -> ResponseResult<()> {
        match msg.from {
            None => {
                send_no_user(msg.chat.id, bot).await?;
            }
            Some(user) => {
                let acc_idx = user.id.0;
                match self.cache.lock().await.revoke(acc_idx) {
                    Ok(Some(_)) => {
                        self.clients.lock().await.remove(acc_idx).await;
                        bot.send_message(msg.chat.id, "Revoked previously stored token.")
                            .await?;
                    }
                    Ok(None) => {
                        bot.send_message(
                            msg.chat.id,
                            "No stored token. This operation carries no effect.",
                        )
                        .await?;
                    }
                    Err(e) => {
                        eprintln!("Error while revoking token, user id = {}: {:?}", acc_idx, e);
                        bot.send_message(
                            msg.chat.id,
                            format!(
                                "Failed to update database. {}",
                                get_contact_admin_text(acc_idx)
                            ),
                        )
                        .await?;
                    }
                }
            }
        }
        Ok(())
    }
}

pub struct Watch<AP: Repository<ApplicationProgress>> {
    bot: Arc<Bot>,
    clients: Arc<Mutex<ClientCollection>>,
    cache: Arc<Mutex<AP>>,
    watch: Watcher<AccountIndex>,
    ic_rx: Receiver<AccountIndex>,
}

impl<AP> Watch<AP>
where
    AP: Repository<ApplicationProgress>,
    <AP as Repository<ApplicationProgress>>::Err: Debug,
{
    pub fn new<I>(
        bot: Arc<Bot>,
        clients: Arc<Mutex<ClientCollection>>,
        intervals: &I,
        cache: Arc<Mutex<AP>>,
        change_rx: Receiver<AccountIndex>,
    ) -> Self
    where
        I: Repository<Duration>,
        <I as Repository<Duration>>::Err: Debug,
    {
        let watch: Watcher<AccountIndex> = intervals
            .entries()
            .expect("Failed to list user intervals")
            .collect();

        Self {
            bot,
            clients,
            cache,
            watch,
            ic_rx: change_rx,
        }
    }

    async fn notify_if_applicable(&self, account: AccountIndex) {
        match self.get_status_changes(account).await {
            Ok(Some(change)) => {
                let push_result = self
                    .bot
                    .send_message(UserId(account), format!("Progress update: {}", change))
                    .await;
                if let Err(e) = push_result {
                    println!("Error while pushing: {}, user id = {}", e, account)
                }
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!("Error while monitoring: {}, user id = {}", e, account)
            }
        }
    }

    async fn get_status_changes(
        &self,
        account: AccountIndex,
    ) -> ClientResult<Option<StatusChange>> {
        let old_progress = self.cache.lock().await.get(account).expect(
            format!(
                "Database failed to query progress cache, user id = {}",
                account
            )
            .as_str(),
        );
        let curr = self
            .clients
            .lock()
            .await
            .get(account)
            .await
            .expect(format!("Missing client, user id = {}", account).as_str())
            .to_owned()
            .lock()
            .await
            .get_application_progress()
            .await;
        match curr {
            Ok(progress) => {
                if old_progress.map_or(true, |o| o != progress) {
                    Ok(Some(StatusChange::Progress(progress)))
                } else {
                    Ok(None)
                }
            }
            Err(crate::tencent::error::Error::TokenExpired) => Ok(Some(StatusChange::Expiry)),
            Err(e) => Err(e),
        }
    }

    pub async fn start_monitoring(&mut self) {
        loop {
            let next = self.watch.next().await;
            match next {
                Some(acc) => self.notify_if_applicable(acc).await,
                None => {
                    self.ic_rx.recv().await;
                }
            }
        }
    }
}

async fn send_no_user(chat_id: ChatId, bot: &Bot) -> ResponseResult<Message> {
    bot.send_message(chat_id, "No user info bound to this context.")
        .await
}

fn get_contact_admin_text(user_id: AccountIndex) -> String {
    format!(
        "Please contact the system administrator, with your user id {}.",
        user_id
    )
}

fn make_interval_keyboard() -> InlineKeyboardMarkup {
    let options = vec![1, 3, 5, 10, 30, 60, 120, 360, 1440];
    let mut keys: Vec<Vec<_>> = Vec::new();
    for row in options.chunks(3) {
        keys.push(
            row.iter()
                .map(|&min| InlineKeyboardButton::callback(format!("{min}min"), min.to_string()))
                .collect(),
        )
    }
    InlineKeyboardMarkup::new(keys)
}
