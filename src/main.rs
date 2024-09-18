use std::cell::LazyCell;

use regex::Regex;
use serenity::{
    all::{ApplicationId, ClientBuilder, ExecuteWebhook, UserId},
    async_trait,
    json::json,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use url::Url;

const CLIENT_ID: u64 = 1283210364883828766;

const INVITE_LINK: &'static str = const_format::formatcp!("https://discord.com/oauth2/authorize?client_id={}&permissions=536881152&integration_type=0&scope=bot", CLIENT_ID);

const URL_REGEX: LazyCell<Regex> = LazyCell::new(|| {
    Regex::new(
        r#"(http|ftp|https):\/\/([\w_-]+(?:(?:\.[\w_-]+)+))([\w.,@?^=%&:\/~+#-]*[\w@?^=%&\/~+#-])"#,
    )
    .unwrap()
});

const WEBHOOK_ID: &'static str = "I can fix it";

struct Handler;

impl Handler {
    async fn handle_message(&self, ctx: Context, message: Message) -> serenity::Result<()> {
        match message.guild_id {
            None => return Ok(()),
            _ => (),
        };

        match self.try_handle_links(&ctx, &message).await? {
            true => return Ok(()),
            _ => (),
        };

        self.try_handle_deletions(&ctx, &message).await.map(|_| ())
    }

    async fn try_handle_deletions(
        &self,
        ctx: &Context,
        message: &Message,
    ) -> serenity::Result<bool> {
        let replied_to = match &message.referenced_message {
            Some(message) => message,
            None => return Ok(false),
        };

        dbg!(replied_to.webhook_id);
        dbg!(replied_to.application_id);
        dbg!(replied_to.author_nick(&ctx).await);
        match replied_to.webhook_id {
            None => return Ok(false),
            _ => (),
        };

        if replied_to.application_id != Some(ApplicationId::new(CLIENT_ID)) {
            return Ok(false);
        }

        let author_nick = match replied_to.author_nick(&ctx).await {
            Some(nick) => nick,
            _ => replied_to
                .author
                .global_name
                .clone()
                .unwrap_or(replied_to.author.name.clone()),
        };

        let deletion_intent =
            message.content.contains("supprime") || message.content.contains("atomise");
        let is_authorized = message.author.id == UserId::new(319181790269014016)
            || author_nick.starts_with(&message.author_nick(&ctx).await.unwrap_or_else(|| {
                message
                    .author
                    .global_name
                    .clone()
                    .unwrap_or(replied_to.author.name.clone())
            }));

        dbg!(deletion_intent, is_authorized);
        if deletion_intent && is_authorized {
            ctx.http()
                .delete_message(replied_to.channel_id, replied_to.id, Some("tgl"))
                .await?;
        }

        Ok(deletion_intent && is_authorized)
    }

    async fn try_handle_links(&self, ctx: &Context, message: &Message) -> serenity::Result<bool> {
        let detected_urls = (*URL_REGEX)
            .captures_iter(&message.content)
            .filter_map(|url| url.get(0))
            .collect::<Vec<_>>();

        if detected_urls.is_empty() {
            return Ok(false);
        }

        println!("Message sent \"{}\"", message.content);

        let modifications = detected_urls
            .into_iter()
            .rev()
            .map(|r#match| (r#match, Url::parse(r#match.as_str())))
            .filter_map(|(r#match, url)| match url {
                Ok(url) => Some((r#match, url)),
                Err(_) => None,
            })
            .filter(|(_, url)| ["x.com", "twitter.com"].contains(&url.domain().unwrap_or_default()))
            .map(|(r#match, mut url)| {
                let _ = url.set_host(Some("fxtwitter.com"));

                (r#match, url)
            })
            .map(|(r#match, url)| Modification {
                start: r#match.start(),
                end: r#match.end(),
                string: url.to_string(),
            })
            .collect::<Vec<_>>();

        if modifications.is_empty() {
            return Ok(false);
        }

        let mut content = message.content.clone();
        for modification in modifications {
            println!(
                "Replacing text from {} to {} with {}",
                modification.start, modification.end, &modification.string
            );
            content.replace_range(modification.start..modification.end, &modification.string);
        }

        println!("{content}");

        let author = message
            .author_nick(ctx.http())
            .await
            .or(message.author.global_name.clone())
            .unwrap_or(message.author.name.clone());

        // This url uses the .webp format and is 1024x1024 in size
        // We need to modify it to use .png and 128x128 dimensions
        let author_profile_picture_url = message
            .author
            .face()
            .replace("1024", "128")
            .replace(".webp", ".png");

        dbg!(&author_profile_picture_url);

        let webhook = ctx
            .http()
            .get_channel_webhooks(message.channel_id)
            .await?
            .into_iter()
            .find(|webhook| webhook.name.clone().unwrap_or_default() == WEBHOOK_ID);

        let webhook = match webhook {
            Some(webhook) => webhook,
            None => {
                ctx.http()
                    .create_webhook(
                        message.channel_id,
                        &json!({"name": WEBHOOK_ID}),
                        Some("Finito ce nab"),
                    )
                    .await?
            }
        };

        ctx.http()
            .delete_message(message.channel_id, message.id, Some("Finito ce nab xd"))
            .await?;

        webhook
            .execute(
                ctx,
                false,
                ExecuteWebhook::new()
                    .username(format!("{author} 🤖🗿"))
                    .avatar_url(author_profile_picture_url)
                    .content(format!(":nerd: :point_up: {content}")),
            )
            .await?;

        println!("Executed webhook command with id {}", &webhook.id);

        Ok(true)
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        println!("Invite me with this link {INVITE_LINK}");
    }

    async fn message(&self, _ctx: Context, message: Message) {
        match self.handle_message(_ctx, message).await {
            Err(err) => {
                println!("An error occured (this shouldn't be the case)");
                dbg!(err);
            }
            _ => {}
        };
    }
}

struct Modification {
    pub start: usize,
    pub end: usize,
    pub string: String,
}

#[tokio::main]
async fn main() {
    let intents = GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS;
    let bot_token = std::env::var("BOT_TOKEN").expect("No token found");

    let mut client = ClientBuilder::new(&bot_token, intents)
        .event_handler(Handler)
        .await
        .unwrap();

    if let Err(cause) = client.start().await {
        println!("{:?}", cause);
    }

    // 536881152

    println!("Hello, world!");
}
