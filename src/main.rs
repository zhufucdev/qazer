use redb::{Database, TableDefinition};
use std::sync::Arc;
use std::time::Duration;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::prelude::{
    CallbackQuery, Dispatcher, LoggingErrorHandler, Requester, ResponseResult,
};
use teloxide::types::{Message, Update};
use teloxide::utils::command::BotCommands;
use teloxide::{dptree, Bot};
use tokio::spawn;
use tokio::sync::Mutex;

mod bot;
mod repo;
mod tencent;
mod watch;

use crate::repo::model::AccountIndex;
use crate::repo::redb::{RedbRepo, RedbRepoDefault};
use crate::tencent::model::ApplicationProgress;
use bot::cmd::Command;

type DefaultBasicLogic = bot::logic::Basic<
    RedbRepoDefault<String>,
    RedbRepo<Vec<u8>, ApplicationProgress>,
    RedbRepo<u32, Duration>,
>;

const TOKENS_TABLE: TableDefinition<AccountIndex, String> = TableDefinition::new("tokens");
const PROGRESS_TABLE: TableDefinition<AccountIndex, Vec<u8>> = TableDefinition::new("progress");
const INTERVAL_TABLE: TableDefinition<AccountIndex, u32> = TableDefinition::new("interval");

#[tokio::main]
async fn main() {
    let db = Arc::new(Database::create("qazer.redb").expect("Failed to create database"));
    let token_repo = RedbRepo::new(TOKENS_TABLE, db.to_owned());
    let progress_repo = Arc::new(Mutex::new(RedbRepo::new_proxy(
        PROGRESS_TABLE,
        db.to_owned(),
        repo::redb::Transformer {
            forward: |e| bson::from_slice::<ApplicationProgress>(e.as_slice()).unwrap(),
            backward: |e| bson::to_vec(&e).unwrap(),
        },
    )));
    let interval_repo = Arc::new(Mutex::new(RedbRepo::new_proxy(
        INTERVAL_TABLE,
        db.to_owned(),
        repo::redb::Transformer {
            forward: |minutes| Duration::from_secs((minutes as u64) * 60),
            backward: |duration| (duration.as_secs() / 60) as u32,
        },
    )));

    let clients = Arc::new(Mutex::new(bot::clients::ClientCollection::from_token_repo(
        &token_repo,
    )));
    let (ic_tx, ic_rx) = tokio::sync::mpsc::channel(1);

    let bot = Arc::new(Bot::from_env());
    let mut watch_logic = bot::logic::Watch::new(
        bot.to_owned(),
        clients.to_owned(),
        interval_repo.to_owned(),
        progress_repo.to_owned(),
        ic_rx,
    );
    let basic_logic = Arc::new(Mutex::new(bot::logic::Basic::new(
        token_repo,
        progress_repo,
        interval_repo,
        clients,
        ic_tx,
    )));

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(default_command_handler),
        )
        .branch(Update::filter_callback_query().endpoint(default_callback_handler));

    let watch_handle = spawn(async move { watch_logic.start_monitoring().await });
    Dispatcher::builder(bot.to_owned(), handler)
        .dependencies(dptree::deps![basic_logic.to_owned()])
        .error_handler(LoggingErrorHandler::new())
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    watch_handle.abort();
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
        Command::Interval => logic.lock().await.interval(bot.as_ref(), msg).await?,
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
    };
    Ok(())
}

async fn default_callback_handler(
    bot: Arc<Bot>,
    q: CallbackQuery,
    logic: Arc<Mutex<DefaultBasicLogic>>,
) -> ResponseResult<()> {
    logic
        .lock()
        .await
        .interval_callback_handler(bot.as_ref(), q)
        .await?;
    Ok(())
}
