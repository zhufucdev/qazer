use crate::bot::change::StatusChange;
use crate::repo::model::{AccountIndex, Repository};
use crate::tencent::model::ApplicationProgress;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::{Message, Requester, ResponseResult, UserId};
use teloxide::Bot;
use teloxide::types::User;
use tokio::task::JoinSet;
use tokio::time;

pub struct Basic<Tokens, APs>
where
    Tokens: Repository<String>,
    APs: Repository<ApplicationProgress>,
{
    bot: Arc<Bot>,
    tokens: Tokens,
    cache: APs,
    interval: Duration,
    clients: HashMap<AccountIndex, Arc<crate::tencent::Client>>,
}

impl<R, T> Basic<R, T>
where
    R: Repository<String> + Sync,
    T: Repository<ApplicationProgress> + Sync,
    <R as Repository<String>>::Err: Debug,
    <T as Repository<ApplicationProgress>>::Err: Debug,
{
    pub fn new(bot: Arc<Bot>, tokens: R, cache: T, interval: Duration) -> Basic<R, T> {
        let clients = HashMap::from_iter(
            tokens
                .entries()
                .expect("Failed to list registered user id")
                .map(|(id, token)| (id, Arc::new(crate::tencent::Client::with_token(&token)))),
        );
        Self {
            bot,
            tokens,
            cache,
            interval,
            clients,
        }
    }

    pub async fn start_monitoring(&mut self) {
        let mut interval = time::interval(self.interval);
        loop {
            interval.tick().await;
            let changes = self.get_status_changes().await;
            for (acc, ch) in changes.iter() {
                let push_result = self
                    .bot
                    .send_message(UserId(*acc), format!("Progress update: {}", ch))
                    .await;
                if let Err(e) = push_result {
                    println!("Error while pushing to {}: {}", acc, e)
                }
            }
        }
    }

    async fn get_status_changes(&self) -> HashMap<AccountIndex, StatusChange> {
        let mut set = JoinSet::new();
        for (account, old_progress) in self
            .cache
            .entries()
            .expect("Failed to list registered users and cached status")
        {
            let client = self
                .clients
                .get(&account)
                .expect(format!("Missing client for user {}", account).as_str())
                .to_owned();
            set.spawn(async move {
                match client.get_application_progress().await {
                    Ok(progress) => {
                        if old_progress != progress {
                            Some((account, StatusChange::Progress(progress)))
                        } else {
                            None
                        }
                    }
                    Err(crate::tencent::error::Error::TokenExpired) => {
                        Some((account, StatusChange::Expiry))
                    }
                    Err(e) => {
                        println!("{}", e);
                        None
                    }
                }
            });
        }

        let mut result = HashMap::new();
        while let Some(res) = set.join_next().await {
            if let Some((account, change)) = res.expect("Failed to process concurrent query response") {
                result.insert(account, change);
            }
        }
        result
    }

    fn update_status(&mut self, account: AccountIndex, value: &ApplicationProgress) {
        self.cache
            .put(account, value.clone())
            .expect("Failed to update database")
    }
    
    pub async fn get(&mut self, bot: &Bot, msg: Message) -> ResponseResult<()> {
        match msg.from {
            None => {
                bot.send_message(msg.chat.id, "No user info bound to current context.")
                    .await?;
            }
            Some(user) => {
                let acc_idx = user.id.0;
                match self.clients.get(&acc_idx) {
                    None => {
                        bot.send_message(msg.chat.id, "No token associated with current context. Use the /signin command to get started.").await?;
                    }
                    Some(client) => {
                        match client.get_application_progress().await {
                            Ok(ap) => {
                                self.update_status(acc_idx, &ap);
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
            },
        }
        Ok(())
    }

    pub async fn signin(&mut self, bot: &Bot, msg: Message, token: String) -> ResponseResult<()> {
        let new_client = Arc::new(crate::tencent::Client::with_token(&token));
        match new_client.get_application_progress().await {
            Ok(ap) => match msg.from {
                None => {
                    bot.send_message(msg.chat.id, "No user info is bound to this context.")
                        .await?;
                }
                Some(user) => {
                    let acc_idx = user.id.0;
                    self.update_status(acc_idx, &ap);
                    self.clients.insert(acc_idx, new_client);
                    match self.tokens.put(acc_idx, token) {
                        Ok(_) => {
                            bot.send_message(msg.chat.id, "Token has been updated.")
                                .await?;
                        }
                        Err(e) => {
                            bot.send_message(msg.chat.id, format!("Error updating database. {}", Self::get_contact_admin_text(acc_idx))).await?;
                            eprintln!("Error while inserting token, user id = {}. {:?}", acc_idx, e)
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
    
    pub async fn signout(&mut self, bot: &Bot, msg: Message) -> ResponseResult<()> {
        match msg.from {
            None => {
                bot.send_message(msg.chat.id, "No user info bound to this context.").await?;
            },
            Some(user) => {
                let acc_idx = user.id.0;
                match self.cache.revoke(acc_idx) {
                    Ok(Some(_)) => {
                        self.clients.remove(&acc_idx);
                        bot.send_message(msg.chat.id, "Revoked previously stored token.").await?;
                    }
                    Ok(None) => {
                        bot.send_message(msg.chat.id, "No stored token. This operation carries no effect.").await?;
                    }
                    Err(e) => {
                        bot.send_message(msg.chat.id, format!("Failed to update database. {}", Self::get_contact_admin_text(acc_idx))).await?;
                        eprintln!("Error while revoking token, user id = {}: {:?}", acc_idx, e)
                    }
                }
            }
        }
        Ok(())
    }
    
    fn get_contact_admin_text(user_id: AccountIndex) -> String {
        format!("Please contact the system administrator, with your user id {}.", user_id)
    } 
}
