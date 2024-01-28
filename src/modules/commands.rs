use std::{collections::HashMap, str::FromStr};

use poise::serenity_prelude as serenity;
use tokio::sync::Mutex;

use crate::{
    db::{claim_key_with_user, set_config_val},
    Args,
};
pub struct Data {
    db: sqlx::SqlitePool,
    args: Args,
    #[allow(dead_code)]
    config: Mutex<HashMap<String, String>>,
} // User data, which is stored and accessible in all command invocations

impl Data {
    pub fn new(db: sqlx::SqlitePool, args: Args, config: Mutex<HashMap<String, String>>) -> Self {
        Self { db, args, config }
    }
}

// Types used by all command functions
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

/// Command to explain other commands
///
/// example invocation: `/help give_key`
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "This is an example",
            ..Default::default()
        },
    )
    .await?;

    Ok(())
}

#[poise::command(slash_command, required_permissions = "ADMINISTRATOR", ephemeral)]
pub async fn set_key_role(
    ctx: Context<'_>,
    #[autocomplete = "poise::builtins::autocomplete_command"]
    #[description = "Role to give to users who claim a key"]
    role: serenity::Role,
) -> Result<(), Error> {
    let mut d = ctx.data().config.lock().await;

    d.insert(String::from("role_id"), role.id.to_string());

    set_config_val(&ctx.data().db, "role_id", &role.id.to_string()).await?;
    drop(d);

    ctx.say(format!("Key role set to {}", role.name)).await?;

    Ok(())
}

// Command to give a key to a user
//
// Works as a slash command and a context menu command
// example invocation: `/give_key @user`
// example invocation: Right click on username -> apps -> Give Key
#[poise::command(
    slash_command,
    required_permissions = "ADMINISTRATOR",
    context_menu_command = "Give Key"
)]
pub async fn give_key(
    ctx: Context<'_>,
    #[description = "Give key to this user, key is sent as a DM to the user"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    user: serenity::User,
) -> Result<(), Error> {
    let key = claim_key_with_user(&ctx.data().db, &user.name).await;

    if let Err(e) = key {
        ctx.defer_ephemeral().await?;
        ctx.say(format!(
            "Could not get key, please try again later\n\nError: {e}"
        ))
        .await?;
        return Err(e.into());
    }

    let msg = serenity::CreateMessage::new().content(String::from(format!(
        r#"Congratulations, you have been given a key!
You can claim your key by entering it into steam.
Your key is: {}
"#,
        key.expect("Could not get key, this options should be unreachable, please contact Yousof if you see this message")
    )));
    user.direct_message(&ctx, msg).await?;

    ctx.defer_ephemeral().await?;
    ctx.say(format!("Key sent to {}", user.name)).await?;

    Ok(())
}

#[poise::command(slash_command, required_permissions = "ADMINISTRATOR", track_edits)]
pub async fn create_key_post(
    ctx: Context<'_>,
    #[description = "Duration of the giveaway in seconds, defaults to 1 hour"] duration: Option<
        u64,
    >,
    message: Option<String>,
) -> Result<(), Error> {
    let data = ctx.data().config.lock().await;
    let role = data.get("role_id");

    let role = if let Some(role) = role {
        role
    } else {
        ctx.say("No role set, please set a role using /set_key_role")
            .await?;
        return Ok(());
    };

    let reply = {
        let embed = serenity::CreateEmbed::default().image("https://upload.wikimedia.org/wikipedia/commons/thumb/8/83/Steam_icon_logo.svg/512px-Steam_icon_logo.svg.png");

        let components = vec![serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new("1")
                .label("Get key")
                .style(serenity::ButtonStyle::Primary),
        ])];

        poise::CreateReply::default()
            .content(message.unwrap_or_else(|| {
                format!(
                    "If you have the role <@&{}>\n\nClick the button below to get a beta key",
                    role
                )
            }))
            .embed(embed)
            .components(components)
    };
    let res = ctx.send(reply).await?;

    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx)
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(std::time::Duration::from_secs(
            duration.unwrap_or_else(|| ctx.data().args.duration_giveaway),
        ))
        .filter(move |mci| mci.data.custom_id == "1")
        .await
    {
        // check if interaction uer has permission to claim a key
        // mci.user.has_role(ctx, ctx.guild_id());
        let has_role = mci
            .user
            .has_role(
                ctx,
                ctx.guild_id().expect("Could not get the guildID"),
                serenity::RoleId::from_str(role).expect("Could not parse role id"),
            )
            .await?;

        if has_role {
            let key = claim_key_with_user(&ctx.data().db, &mci.user.name).await;

            if let Err(e) = key {
                ctx.defer_ephemeral().await?;
                ctx.say(format!(
                    "Could not get key, please try again later\n\nError: {e}"
                ))
                .await?;
                return Err(e.into());
            }

            let msg = serenity::CreateMessage::new().content(String::from(format!(
        r#"Congratulations, you have been given a key!
You can claim your key by entering it into steam.
Your key is: {}
"#,
        key.expect("Could not get key, this options should be unreachable, please contact Yousof if you see this message")
    )));
            mci.user.direct_message(&ctx, msg).await?;
        } else {
            mci.user
                .direct_message(
                    &ctx,
                    serenity::CreateMessage::new().content(
                        "You do not have permission to claim a key, please contact an admin if you think this is a mistake",
                    ),
                )
                .await?;
        }

        mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
            .await?;
    }

    res.edit(
        ctx,
        poise::reply::CreateReply::default()
            .content("This key giveaway is over!")
            .components(vec![]),
    )
    .await?;

    Ok(())
}
