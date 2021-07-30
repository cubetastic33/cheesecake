use glob::glob;
use chrono::prelude::*;
use rusqlite::{Connection, ToSql};
use std::{collections::HashMap, env, fs, path::Path};
use super::{parser, convertor};

#[derive(Serialize)]
pub struct SelectionContext<'a> {
    backup_path: &'a str,
    chat_id: &'a str,
    backups: Vec<[String; 2]>,
    chats: Vec<[String; 2]>,
    mapped_chats: String,
}

#[derive(Serialize)]
pub struct Message {
    id: String,
    message_type: String,
    name: String,
    avatar: String,
    color: String,
    bot: u8,
    created_timestamp: String,
    edited_timestamp: Option<String>,
    separate: bool,
    reference: Option<(String, String, String, String, String, bool)>, // id, name, avatar, color, message, attachments?
    content: String,
    attachments: Vec<(String, String, bool)>, // source, type, spoiler?
    reactions: Vec<(String, Option<String>, usize)>, // name, source, count
}

#[derive(Serialize)]
pub struct ChatContext<'a> {
    name: String,
    topic: String,
    messages: Vec<Message>,
    selection_context: Option<SelectionContext<'a>>,
}

impl Default for ChatContext<'_> {
    fn default() -> Self {
        ChatContext {
            name: String::new(),
            topic: String::new(),
            messages: Vec::new(),
            selection_context: None,
        }
    }
}

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

pub fn refrigerator() -> String {
    env::var("REFRIGERATOR").unwrap_or(String::from("refrigerator"))
}

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

fn file_type(file_name: &str) -> String {
    if let Some(extension) = Path::new(file_name).extension() {
        let ext = extension.to_str().unwrap().to_ascii_lowercase();
        if [
            "apng", "avif", "gif", "jpg", "jpeg", "jfif", "pjpeg", "pjp", "png", "svg", "webp",
        ]
        .contains(&ext.as_str())
        {
            return String::from("image");
        } else if ["mp4", "ogv", "ogg"].contains(&ext.as_str()) {
            return String::from("video");
        } else if ["wav", "flac", "mp3", "oga"].contains(&ext.as_str()) {
            return String::from("audio");
        }
    }
    String::from("unknown")
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

// Creates context with information to select a chat from
pub fn selection_context<'a>(backup_path: &'a str, chat_id: &'a str) -> SelectionContext<'a> {
    let mut selected_backup = 0;
    let mut backups = Vec::new();
    let mut mapped_chats = HashMap::new();
    // Iterate over all the cheesecakes found in the refrigerator
    for entry in glob(
        Path::new(&refrigerator())
            .join("*/info.json")
            .to_str()
            .unwrap(),
    ).unwrap() {
        match entry {
            Ok(path) => {
                let json: serde_json::Value =
                    serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
                if json["version"] == "0.1.0" && (json["type"] == "discord" || json["type"] == "matrix") {
                    // The path of the backup (like "123456789123456789")
                    let current_backup_path = path
                        .parent()
                        .unwrap()
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned();
                    // The name of the backup (like "Archive 1")
                    let current_backup_name = json["name"].as_str().unwrap().to_owned();
                    backups.push([current_backup_path.clone(), current_backup_name]);
                    if backup_path == current_backup_path {
                        selected_backup = backups.len() - 1;
                    }
                    // Get the list of chats
                    let mut chats = Vec::new();
                    let conn = Connection::open(path.parent().unwrap().join("backup.db")).unwrap();
                    let mut statement = conn.prepare("SELECT id, name FROM chats").unwrap();
                    let mut rows = statement.query([]).unwrap();
                    while let Some(chat) = rows.next().unwrap() {
                        chats.push([
                            chat.get(0).unwrap(),
                            chat.get(1).unwrap(),
                        ]);
                    }
                    mapped_chats.insert(current_backup_path, chats);
                }
            }
            Err(e) => println!("{:?}", e),
        }
    }
    let chats = mapped_chats
        .get(&backups[selected_backup][0])
        .unwrap()
        .to_owned();
    SelectionContext {
        backup_path,
        chat_id,
        backups,
        chats,
        mapped_chats: serde_json::to_string(&mapped_chats).unwrap(),
    }
}

// A function that executes an SQL command and collects the messages into Vec<Message>
fn populate_messages<'a>(
    backup_path: &'a str,
    sql_query: &'a str,
    params: &[&dyn ToSql],
) -> Vec<Message> {
    // Create a connection to the database
    let database_path = Path::new(&refrigerator())
        .join(backup_path)
        .join("backup.db");
    let conn = Connection::open(database_path).unwrap();
    let mut messages: Vec<Message> = Vec::new();
    let mut statement = conn.prepare(&sql_query.replace("{}", "SELECT id,
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
    // for easy comparison
    let mut previous_timestamp = Local.timestamp(0, 0);

    while let Some(row) = rows.next().unwrap() {
        // Get a list of the attachments with their file types
        let mut attachments = Vec::new();
        for attachment in row.get(10).unwrap_or(String::new()).split(' ') {
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
        let message_type = row.get(1).unwrap();
        let name = row.get(2).unwrap();
        let avatar = url(backup_path, Avatar, &row.get::<_, String>(3).unwrap());
        let color = row.get(4).unwrap_or(String::from("#afafaf"));
        let bot = row.get(5).unwrap();
        // For checking if a day separator needs to be shown
        let created_timestamp: DateTime<Local> = row.get(6).unwrap();
        // For keeping track of whether the message should be displayed separately
        let mut separate = true;

        if messages.len() > 0 {
            // Check if the message should be displayed separately
            let previous = &messages[messages.len() - 1];
            // We override the separate variable later on if the message is a reply
            separate = !(&message_type == "default" && previous.message_type == String::from("default") && previous.name == name && previous.avatar == avatar && previous.color == color && previous.bot == bot && (created_timestamp - previous_timestamp).num_minutes() <= 5);
            // Add a day separator if necessary
            if previous_timestamp.date() != created_timestamp.date() {
                messages.push(Message {
                    id: String::new(),
                    message_type: String::from("day_separator"),
                    name: String::new(),
                    avatar: String::new(),
                    color: String::new(),
                    bot: 0,
                    created_timestamp: String::new(),
                    edited_timestamp: None,
                    separate: true,
                    reference: None,
                    content: created_timestamp.date().format("%Y-%m-%d").to_string(),
                    attachments: Vec::new(),
                    reactions: Vec::new(),
                });
            }
        }

        // Replies
        let mut reference = None;
        if &message_type == "default" {
            if let Ok(reference_id) = row.get::<_, u64>(8) {
                separate = true;
                let mut statement = conn.prepare("SELECT name, avatar, color, content, attachments FROM messages WHERE id = $1").unwrap();
                let mut rows = statement.query([reference_id]).unwrap();
                if let Some(row) = rows.next().unwrap() {
                    reference = Some((
                        reference_id.to_string(),
                        row.get(0).unwrap(),
                        url(backup_path, Avatar, &row.get::<_, String>(1).unwrap()),
                        row.get(2).unwrap_or(String::from("#afafaf")),
                        row.get(3).unwrap_or(String::new()),
                        match row.get::<_, String>(4) {Ok(_) => true, Err(_) => false},
                    ));
                }
            }
        }

        // If the messages was edited
        let edited_timestamp = match row.get::<_, DateTime<Local>>(7) {
            Ok(timestamp) => Some(timestamp.format("%Y-%m-%d %H:%M").to_string()),
            Err(_) => None,
        };

        // Parse markdown
        let raw_content: String = row.get(9).unwrap_or(String::new());
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
        if let Ok(r) = row.get::<_, String>(11) {
            println!("{}", r);
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
            id: row.get::<_, u64>(0).unwrap().to_string(),
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
            attachments,
            reactions,
        });

        previous_timestamp = created_timestamp;
    }
    messages
}

pub fn chat<'a>(backup_path: &'a str, chat_id: &'a str) -> ChatContext<'a> {
    let messages = populate_messages(
        backup_path,
        "SELECT * FROM ({} channel = $1 ORDER BY id DESC LIMIT 100) ORDER BY id",
        &[&chat_id],
    );
    // Create a connection to the database
    let database_path = Path::new(&refrigerator())
        .join(backup_path)
        .join("backup.db");
    let conn = Connection::open(database_path).unwrap();
    // Get the channel name and topic
    let mut statement = conn
        .prepare("SELECT name, topic FROM chats WHERE id = $1")
        .unwrap();
    let mut rows = statement.query([chat_id.parse::<u64>().unwrap()]).unwrap();
    let channel_details = rows.next().unwrap().unwrap();
    // Return the ChatContext
    ChatContext {
        name: channel_details.get(0).unwrap(),
        topic: channel_details.get(1).unwrap_or(String::new()),
        messages,
        selection_context: Some(selection_context(backup_path, chat_id)),
    }
}

pub fn jump_chat<'a>(backup_path: &'a str, chat_id: u64, message_id: Option<u64>) -> ChatContext<'static> {
    // Create a connection to the database
    let database_path = Path::new(&refrigerator())
        .join(backup_path)
        .join("backup.db");
    let conn = Connection::open(database_path).unwrap();
    // Get the channel name and topic
    let mut statement = conn
        .prepare("SELECT name, topic FROM chats WHERE id = $1")
        .unwrap();
    let mut rows = statement.query([chat_id]).unwrap();
    match rows.next().unwrap() {
        Some(channel_details) => {
            let messages = match message_id {
                Some(id) => populate_messages(
                    backup_path,
                    "SELECT * FROM ({} channel = $1 AND id <= $2 ORDER BY id DESC LIMIT 50)
                    UNION SELECT * FROM ({} channel = $1 AND id > $2 ORDER BY id LIMIT 50) ORDER BY id",
                    &[&chat_id, &id]
                ),
                None => populate_messages(
                    backup_path,
                    "SELECT * FROM ({} channel = $1 ORDER BY id DESC LIMIT 100) ORDER BY id",
                    &[&chat_id]
                ),
            };

            ChatContext {
                name: channel_details.get(0).unwrap(),
                topic: channel_details.get(1).unwrap_or(String::new()),
                messages,
                selection_context: None,
            }
        },
        // If the channel doesn't exist in the database
        None => ChatContext::default(),
    }
}

pub fn get_messages<'a>(
    backup_path: &'a str,
    chat_id: &'a str,
    message_id: u64,
    position: &'a str,
) -> Vec<Message> {
    let condition = if position == "above" {
        "SELECT * FROM ({} channel = $1 AND id < $2 ORDER BY id DESC LIMIT 100) ORDER BY id"
    } else if position == "below" {
        "{} channel = $1 AND id > $2 ORDER BY id LIMIT 100"
    } else {
        "SELECT * FROM ({} channel = $1 AND id <= $2 ORDER BY id DESC LIMIT 50) UNION SELECT * FROM ({} channel = $1 AND id > $2 ORDER BY id LIMIT 50) ORDER BY id"
    };
    populate_messages(
        backup_path,
        condition,
        &[&chat_id, &message_id]
    )
}

pub fn search<'a>(backup_path: &'a str, chat_id: &'a str, query: &str, mut filters: &str) -> Vec<Message> {
    if filters.len() == 0 {
        filters = "TRUE";
    }
    populate_messages(
        backup_path,
        &("{} channel = $1 AND id IN (SELECT id FROM message_search WHERE message_search MATCH $2 ORDER BY rank) AND ".to_owned() + filters),
        &[&chat_id, &query]
    )
}
