use lazy_static::lazy_static;

lazy_static! {
    pub static ref COMMANDS: Vec<&'static str> = vec![
        "ban",
        "unban",
        "clear",
        "color",
        "commercial",
        "delete",
        "disconnect",
        "emoteonly",
        "emoteonlyoff",
        "followers",
        "followersoff",
        "help",
        "host",
        "unhost",
        "marker",
        "me",
        "mod",
        "unmod",
        "mods",
        "r9kbeta",
        "r9kbetaoff",
        "raid",
        "unraid",
        "slow",
        "slowoff",
        "subscribers",
        "subscribersoff",
        "timeout",
        "untimeout",
        "vip",
        "unvip",
        "vips",
        "w",
    ];
}
