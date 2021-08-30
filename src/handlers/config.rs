use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct CompleteConfig {
    /// Connecting to Twitch.
    pub twitch: TwitchConfig,
    /// Internal functionality.
    pub terminal: TerminalConfig,
    /// How everything looks to the user.
    pub frontend: FrontendConfig,
}

#[derive(Deserialize, Clone)]
pub struct TwitchConfig {
    /// The username that this user has on Twitch.
    pub username: String,
    /// The streamer's channel name.
    pub channel: String,
    /// The IRC channel that they'd like to connect to.
    pub server: String,
}

#[derive(Deserialize, Clone)]
pub struct TerminalConfig {
    /// The delay between updates, in milliseconds.
    pub tick_delay: u64,
}

#[derive(Deserialize, Clone)]
pub struct FrontendConfig {
    /// The format of string that will show up in the terminal.
    pub date_format: String,
    /// The maximum length of a Twitch username.
    pub maximum_username_length: u16,
}
