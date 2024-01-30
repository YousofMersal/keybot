mod modules;
use modules::{
    commands::*,
    db::{get_config_val, get_round, read_beta_keys_file, set_round},
    *,
};
use tokio::sync::Mutex;

use config::Config;
use std::{collections::HashMap, sync::Arc, time::Duration};

use clap::Parser;
use dotenv::dotenv;
use poise::serenity_prelude as serenity;
use serenity::{
    all::{Ready, ResumedEvent},
    async_trait,
    gateway::ShardManager,
    prelude::*,
    prelude::{Context, EventHandler, TypeMapKey},
};
use tracing::{debug, error, info};

#[derive(Parser, Debug)]
#[command(author, version, long_about)]
/// Discord bot for storing and retrieving beta keys will read keys off a file called
/// "fresh_keys.txt" in the current directory
///
/// The bot will read all keys in the file and add them to a local database.
/// any new keys added to the file will be added to the database.
/// Any new keys added to the database will be added to the file.
/// The file can at any point be cleared and the bot will continue to function.
/// it will check every 30 seconds for new keys in the file.
struct Args {
    /// Name of the sqlite database file, remember to include the .db extension
    #[arg(short, long)]
    #[clap(default_value = "beta_keys.db")]
    file_name: String,

    /// Discord bot token, can also be provided via the TOKEN environment variable or a .env file in the current directory
    #[arg(short, long)]
    token: Option<String>,

    /// Giveaway duration in seconds
    #[arg(short, long)]
    #[clap(default_value = "3600")]
    duration_giveaway: u64,
}

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<ShardManager>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        if let Some(shard) = ready.shard {
            println!(
                "{} is connected on shard {}/{}",
                ready.user.name,
                shard.id.0 + 1,
                shard.total
            );
        }
    }
    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    tracing_subscriber::fmt::init();

    let pool = match modules::db::connect_or_create(&args.file_name).await {
        Ok(pool) => {
            let table_res = db::add_tables(&pool).await;
            if let Err(e) = table_res {
                panic!("Error adding tables: {:?}", e);
            } else {
                pool
            }
        }
        Err(e) => {
            panic!("Could not create and connect to db: {:?}", e);
        }
    };

    let pool2 = pool.clone();

    let token = if let Some(token) = args.token.as_deref() {
        token.to_owned()
    } else {
        let var_res = std::env::var("TOKEN");

        if let Err(_) = var_res {
            dotenv().ok();
        }

        let token = std::env::var("TOKEN");

        let Ok(token) = token else {
            panic!("No bot token provided, please set the TOKEN environment variable or pass it as an argument, see --help for more information");
        };

        token
    };

    let settings = Config::builder()
        .add_source(config::File::with_name("config").format(config::FileFormat::Json5))
        .build()
        .expect("Config missing or invalid: config.json5")
        .try_deserialize::<HashMap<String, String>>()
        .expect("Could not serialize");

    let options = poise::FrameworkOptions {
        commands: vec![help(), give_key(), create_key_post(), set_key_role()],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("!".into()),
            edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                Duration::from_secs(280),
            ))),
            ..Default::default()
        },
        on_error: |error| Box::pin(on_error(error)),
        ..Default::default()
    };

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::DIRECT_MESSAGE_REACTIONS;

    let mut config: HashMap<String, String> = std::collections::HashMap::new();

    if let Ok(value) = get_config_val(&pool, "role_id").await {
        config.insert(String::from("role_id"), value);
    };

    // if get_round is OK, check if it's None, if it is, create a new round
    if let Ok(None) = get_round(&pool).await {
        set_round(&pool, 1, &mut config)
            .await
            .expect("Error setting round");
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);

                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data::new(pool.clone(), args, Mutex::new(config)))
            })
        })
        .options(options)
        .build();

    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .await
        .expect("Error creating client");

    // Here i clone a lock to the ShardManager, and then move it into a new thread. The thread
    // will unlock the manager and print shards' status on a loop.
    let manager = client.shard_manager.clone();

    tokio::task::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;
            debug!("Checking for new keys");
            if let Err(e) = read_beta_keys_file(&pool2, "./fresh_keys.txt").await {
                println!("Error reading keys: {:?}", e);
            };
        }
    });

    tokio::task::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;

            let _shard_runners = manager.runners.lock().await;
        }
    });

    // start shards
    if let Err(why) = client.start_shards(2).await {
        error!("Client error: {why:?}");
    }
}
