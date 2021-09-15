use chrono::prelude::*;
use rusqlite::{Connection, ToSql};
use std::path::{Path, PathBuf};
use super::actions::{Message, day_separator};

pub enum AssetType {
    Avatar,
    Attachment,
}

impl AssetType {
    // Returns the name of the directory where assets of those type are stored
    fn dir(&self) -> &str {
        match self {
            Self::Avatar => "avatars",
            Self::Attachment => "attachments",
        }
    }
}

use AssetType::*;

// Converts an asset path to a proper URL
pub fn url(backup_path: &str, asset_type: AssetType, asset_path: &str) -> String {
    Path::new(backup_path)
        .join(asset_type.dir())
        .join(asset_path)
        .to_str()
        .unwrap()
        .to_owned()
}

pub fn file_type(file_name: &str) -> String {
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

// A function that executes an SQL command and collects the messages into Vec<Message>
pub fn populate_messages<'a>(
    database_path: &'a PathBuf,
    backup_path: &'a str,
    sql_query: &'a str,
    params: &[&dyn ToSql],
) -> Vec<Message> {
    let conn = Connection::open(database_path).unwrap();
    let mut messages: Vec<Message> = Vec::new();
    let mut statement = conn.prepare(&sql_query.replace("{}", "SELECT ROWID,
        id,
        message_type,
        name,
        avatar,
        color,
        created_timestamp,
        edited_timestamp,
        reference,
        content,
        formatted_content,
        attachments FROM messages WHERE")).unwrap();
    let mut rows = statement.query(params).unwrap();

    // Because the Message instance stores only a string representation of the time, we need this
    // variable for easy comparison
    let mut previous_timestamp = Local.timestamp(0, 0);

    while let Some(row) = rows.next().unwrap() {
        let message_type: String = row.get(2).unwrap();
        let name = row.get(3).unwrap();
        let avatar = match row.get::<_, String>(4) {
            Ok(path) => url(backup_path, Avatar, &path),
            Err(_) => String::from("/images/default.svg"),
        };
        let color = row.get(5).unwrap_or(String::from("#afafaf"));
        // For checking if a day separator needs to be shown
        let created_timestamp: DateTime<Local> = row.get(6).unwrap();
        // For keeping track of whether the message should be displayed separately
        let mut separate = true;

        if messages.len() > 0 {
            // Check if the message should be displayed separately
            let previous = &messages[messages.len() - 1];
            // We override the separate variable later on if the message is a reply
            separate = !(previous.name == name && previous.avatar == avatar && previous.color == color && (created_timestamp - previous_timestamp).num_minutes() <= 5);
            // Add a day separator if necessary
            if previous_timestamp.date() != created_timestamp.date() {
                messages.push(day_separator(created_timestamp));
            }
        }

        // TODO edits

        // Attachments
        let mut attachments = Vec::new();
        if let Ok(raw_json) = row.get::<_, String>(11) {
            let json: serde_json::Value =
                serde_json::from_str(&raw_json).unwrap();
            for attachment in json.as_array().unwrap() {
                attachments.push((
                    url(backup_path, Attachment, attachment.as_str().unwrap()),
                    file_type(attachment.as_str().unwrap()),
                    false,
                ));
            }
        }

        if message_type == "redacted" {
            messages.push(Message {
                sequential_id: row.get::<_, u64>(0).unwrap().to_string(),
                message_id: row.get(1).unwrap(),
                message_type: String::from("redacted"),
                name,
                avatar,
                color,
                created_timestamp: created_timestamp.format("%Y-%m-%d %H:%M").to_string(),
                separate: true,
                ..Default::default()
            });
        } else if message_type == "default" {
            let content = match row.get(10) {
                Ok(formatted_content) => formatted_content,
                Err(_) => html_escape::encode_text(&row.get(9).unwrap_or(String::new())).to_string().replace('\n', "<br>"),
            };

            messages.push(Message {
                sequential_id: row.get::<_, u64>(0).unwrap().to_string(),
                message_id: row.get(1).unwrap(),
                message_type: String::from("default"),
                name,
                avatar,
                color,
                created_timestamp: created_timestamp.format("%Y-%m-%d %H:%M").to_string(),
                separate,
                content,
                attachments,
                ..Default::default()
            });
        }

        previous_timestamp = created_timestamp;
    }
    messages
}
