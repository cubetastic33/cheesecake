use chrono::prelude::*;
use rusqlite::{Connection, ToSql};
use std::path::{Path, PathBuf};
use super::actions::{Message, refrigerator, day_separator};
use super::parser;
use super::convertor;
use super::generic::file_type;

enum AssetType {
    Avatar,
    Attachment,
    Emoji,
}

impl AssetType {
    // Returns the name of the directory where assets of those type are stored
    fn dir(&self) -> &str {
        match self {
            Self::Avatar => "avatars",
            Self::Attachment => "attachments",
            Self::Emoji => "emoji",
        }
    }

    // Returns the name of the directory in discord's servers
    fn discord_dir(&self) -> &str {
        match self {
            Self::Emoji => "emojis",
            _ => self.dir()
        }
    }
}

use AssetType::*;

// Converts an asset path to a proper URL
fn url(backup_path: &str, asset_type: AssetType, asset_path: &str) -> String {
    if Path::new(&refrigerator())
        .join(backup_path)
        .join(asset_type.dir())
        .join(asset_path)
        .exists()
    {
        // If the asset is saved locally
        Path::new(backup_path)
            .join(asset_type.dir())
            .join(asset_path)
            .to_str()
            .unwrap()
            .to_owned()
    } else {
        // If the asset has to be fetched from discord's servers
        String::from("https://cdn.discordapp.com/") + asset_type.discord_dir() + "/" + asset_path
    }
}

fn id_to_name(conn: &Connection, table: &str, id: &str) -> (String, Option<String>) {
    // Get the name from the database
    let mut statement = conn.prepare(&format!(
        "SELECT name{} FROM {} WHERE id = $1",
        if table == "roles" {", color"} else {""},
        table
    )).unwrap();
    let mut rows = statement.query(&[id]).unwrap();

    if let Some(row) = rows.next().unwrap() {
        return (row.get(0).unwrap(), row.get(1).ok());
    }
    // The name was not found
    (String::from("unknown"), None)
}

// A function that executes an SQL command and collects the messages into Vec<Message>
pub fn populate_messages<'a>(
    database_path: &'a PathBuf,
    backup_path: &'a str,
    sql_query: &'a str,
    params: &[&dyn ToSql],
) -> Vec<Message> {
    // Create a connection to the database
    let conn = Connection::open(database_path).unwrap();
    let mut messages: Vec<Message> = Vec::new();
    let mut statement = conn.prepare(&sql_query.replace("{}", "SELECT ROWID,
        id,
        message_type,
        name,
        avatar,
        color,
        bot,
        created_timestamp,
        edited_timestamp,
        reference,
        content,
        attachments,
        reactions FROM messages WHERE")).unwrap();
    let mut rows = statement.query(params).unwrap();

    // Because the Message instance stores only a string representation of the time, we need this
    // variable for easy comparison
    let mut previous_timestamp = Local.timestamp(0, 0);

    while let Some(row) = rows.next().unwrap() {
        // Get a list of the attachments with their file types
        let mut attachments = Vec::new();
        for attachment in row.get(11).unwrap_or(String::new()).split(' ') {
            if attachment.len() > 0 {
                // If it's not an empty string
                attachments.push((
                    url(backup_path, Attachment, attachment),
                    file_type(attachment),
                    attachment.split('/').last().unwrap().starts_with("SPOILER_"),
                ));
            }
        }

        // Determine if the message should be displayed separately
        let message_type = row.get(2).unwrap();
        let name = row.get(3).unwrap();
        let avatar = url(backup_path, Avatar, &row.get::<_, String>(4).unwrap());
        let color = row.get(5).unwrap_or(String::from("#afafaf"));
        let bot = row.get(6).unwrap();
        // For checking if a day separator needs to be shown
        let created_timestamp: DateTime<Local> = row.get(7).unwrap();
        // For keeping track of whether the message should be displayed separately
        let mut separate = true;

        if messages.len() > 0 {
            // Check if the message should be displayed separately
            let previous = &messages[messages.len() - 1];
            // We override the separate variable later on if the message is a reply
            separate = !(&message_type == "default" && previous.message_type == String::from("default") && previous.name == name && previous.avatar == avatar && previous.color == color && previous.bot == bot && (created_timestamp - previous_timestamp).num_minutes() <= 5);
            // Add a day separator if necessary
            if previous_timestamp.date() != created_timestamp.date() {
                messages.push(day_separator(created_timestamp));
            }
        }

        // Replies
        let mut reference = None;
        if &message_type == "default" {
            if let Ok(reference_id) = row.get::<_, u64>(9) {
                separate = true;
                let mut statement = conn.prepare("SELECT ROWID, name, avatar, color, content, attachments FROM messages WHERE id = $1").unwrap();
                let mut rows = statement.query([reference_id]).unwrap();
                if let Some(row) = rows.next().unwrap() {
                    reference = Some((
                        row.get::<_, u64>(0).unwrap().to_string(),
                        row.get(1).unwrap(),
                        url(backup_path, Avatar, &row.get::<_, String>(2).unwrap()),
                        row.get(3).unwrap_or(String::from("#afafaf")),
                        row.get(4).unwrap_or(String::new()),
                        match row.get::<_, String>(5) {Ok(_) => true, Err(_) => false},
                    ));
                }
            }
        }

        // If the messages was edited
        let edited_timestamp = match row.get::<_, DateTime<Local>>(8) {
            Ok(timestamp) => Some(timestamp.format("%Y-%m-%d %H:%M").to_string()),
            Err(_) => None,
        };

        // Parse markdown
        let raw_content: String = row.get(10).unwrap_or(String::new());
        let ast = if bot == 2 {
            parser::parse_embed(&raw_content).unwrap()
        } else {
            parser::parse(&raw_content).unwrap()
        };
        let content = convertor::to_html(
            ast,
            |filename| (url(backup_path, Emoji, filename), None),
            |id| id_to_name(&conn, "users", id),
            |id| id_to_name(&conn, "roles", id),
            |id| id_to_name(&conn, "chats", id),
        );

        // Reactions
        let mut reactions = Vec::new();
        if let Ok(r) = row.get::<_, String>(12) {
            for reaction in r.split(' ') {
                let reaction: Vec<&str> = reaction.split('-').collect();
                let emoji: Vec<&str> = reaction[0].split(':').collect();
                let emoji_name;
                let emoji_path;
                let users = reaction[1].split(',').count();
                if emoji.len() == 1 {
                    // It's a unicode emoji
                    emoji_name = emoji[0].to_string();
                    emoji_path = None;
                } else {
                    // It's a custom emoji
                    emoji_name = emoji[1].to_string();
                    emoji_path = Some(url(
                        backup_path,
                        Emoji,
                        &format!("{}.{}", emoji[2], if emoji[2] == "a" {"gif"} else {"png"})
                    ));
                }
                reactions.push((emoji_name, emoji_path, users));
            }
        }

        messages.push(Message {
            sequential_id: row.get::<_, u64>(0).unwrap().to_string(),
            message_id: row.get::<_, u64>(1).unwrap().to_string(),
            message_type,
            name,
            avatar,
            color,
            bot,
            created_timestamp: created_timestamp.format("%Y-%m-%d %H:%M").to_string(),
            edited_timestamp,
            separate,
            reference,
            content,
            edits_list: String::new(),
            attachments,
            reactions,
        });

        previous_timestamp = created_timestamp;
    }
    messages
}
