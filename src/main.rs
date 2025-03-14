use std::ops::Deref;
use redb::{Database, TableDefinition};
use std::sync::Arc;
use std::time::Duration;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::prelude::{Dispatcher, LoggingErrorHandler, Requester, ResponseResult, Update};
use teloxide::types::Message;
use teloxide::utils::command::BotCommands;
use teloxide::{dptree, Bot};
use tokio::sync::Mutex;

mod bot;
mod repo;
mod tencent;

use crate::repo::model::AccountIndex;
use crate::repo::redb::{RedbRepo, RedbRepoDefault};
use crate::tencent::model::ApplicationProgress;
use bot::cmd::Command;
use log::log;

type DefaultBasicLogic =
    bot::logic::Basic<RedbRepoDefault<String>, RedbRepo<Vec<u8>, ApplicationProgress>>;

const TOKENS_TABLE: TableDefinition<AccountIndex, String> = TableDefinition::new("tokens");
const PROGRESS_TABLE: TableDefinition<AccountIndex, Vec<u8>> = TableDefinition::new("progress");
#[tokio::main]
async fn main() {
    let db = Arc::new(Database::create("qazer.redb").expect("Failed to create database"));
    let token_repo = RedbRepo::new(TOKENS_TABLE, db.to_owned());
    let progress_repo = RedbRepo::new_proxy(
        PROGRESS_TABLE,
        db.clone(),
        repo::redb::Transformer {
            forward: |e| bson::from_slice::<ApplicationProgress>(e.as_slice()).unwrap(),
            backward: |e| bson::to_vec(&e).unwrap(),
        },
    );
    
    let bot = Arc::new(Bot::from_env());
    let logic = bot::logic::Basic::new(bot.to_owned(), token_repo, progress_repo, Duration::from_secs(60 * 5));
    let handler = Update::filter_message().chain(
        dptree::entry()
            .filter_command::<Command>()
            .endpoint(default_command_handler),
    );

    Dispatcher::builder(bot.to_owned(), handler)
        .dependencies(dptree::deps![Arc::new(Mutex::new(logic))])
        .error_handler(LoggingErrorHandler::new())
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn default_command_handler(
    bot: Arc<Bot>,
    msg: Message,
    cmd: Command,
    logic: Arc<Mutex<DefaultBasicLogic>>,
) -> ResponseResult<()> {
    match cmd {
        Command::Get => logic.lock().await.get(bot.as_ref(), msg).await?,
        Command::SignIn { token } => logic.lock().await.signin(bot.as_ref(), msg, token).await?,
        Command::SignOut => logic.lock().await.signout(bot.as_ref(), msg).await?,
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        },
    };
    Ok(())
}
