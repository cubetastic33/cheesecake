use chrono::prelude::*;
use rusqlite::{Connection, ToSql};
use std::path::PathBuf;
use super::actions::{Message, day_separator};
use super::generic::{AssetType::*, url};

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
        created_timestamp,
        edits,
        reference,
        content,
        formatted_content FROM messages WHERE")).unwrap();
    let mut rows = statement.query(params).unwrap();

    // Because the Message instance stores only a string representation of the time, we need this
    // variable for easy comparison
    let mut previous_timestamp = Local.timestamp(0, 0);

    let time = std::time::Instant::now();

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
        if message_type == "m.image" || message_type == "m.file" {
            let file_type = if message_type == "m.image" {"image"} else {"unknown"}.to_string();
            messages.push(Message {
                sequential_id: row.get::<_, u64>(0).unwrap().to_string(),
                message_id: row.get(1).unwrap(),
                message_type: String::from("default"),
                name,
                avatar,
                color,
                created_timestamp: created_timestamp.format("%Y-%m-%d %H:%M").to_string(),
                separate,
                attachments: vec![(url(backup_path, Attachment, &row.get::<_, String>(9).unwrap()), file_type, false)],
                ..Default::default()
            });
        } else if message_type == "m.room.redaction" {
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
        } else if message_type == "m.text" {
            let content = match row.get(10) {
                Ok(formatted_content) => formatted_content,
                Err(_) => row.get(9).unwrap_or(String::new()),
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
                ..Default::default()
            });
        }

        previous_timestamp = created_timestamp;
    }
    println!("time at matrix.rs: {}", (std::time::Instant::now() - time).as_millis());
    messages
}
