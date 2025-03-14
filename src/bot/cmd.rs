use teloxide::macros::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "revoke your token if applicable, replace it with a new one, which is the UserInfo cookie from the recruiter's website.")]
    SignIn { token: String },
    #[command(description = "get the current application state.")]
    Get,
    #[command(description = "revoke your token and stop receiving notifications.")]
    SignOut,
}
