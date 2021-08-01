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
            let mut content = match row.get(10) {
                Ok(formatted_content) => formatted_content,
                Err(_) => row.get(9).unwrap_or(String::new()),
            };

            let mut edited_timestamp = None;
            let mut edits_list = Vec::new();

            // Edits
            if let Ok(edits) = row.get::<_, String>(7) {
                let edits: serde_json::Value = serde_json::from_str(&edits).unwrap();
                edits_list.push([created_timestamp.format("%Y-%m-%d %H:%M").to_string(), content.clone()]);
                for edit in edits.as_array().unwrap() {
                    let edit = edit.as_array().unwrap();
                    content = match edit[4].as_str() {
                        Some(formatted_content) => formatted_content,
                        None => edit[2].as_str().unwrap(),
                    }.to_string();
                    let timestamp = edit[0].as_i64().unwrap();
                    let timestamp = Local.timestamp(timestamp / 1000, timestamp as u32 % 1000);
                    edited_timestamp = Some(timestamp.format("%Y-%m-%d %H:%M").to_string());
                    edits_list.push([timestamp.format("%Y-%m-%d %H:%M").to_string(), content.clone()]);
                }
            }

            let edits_list = if edits_list.is_empty() {
                String::new()
            } else {
                serde_json::to_string(&edits_list).unwrap()
            };

            messages.push(Message {
                sequential_id: row.get::<_, u64>(0).unwrap().to_string(),
                message_id: row.get(1).unwrap(),
                message_type: String::from("default"),
                name,
                avatar,
                color,
                created_timestamp: created_timestamp.format("%Y-%m-%d %H:%M").to_string(),
                edited_timestamp,
                separate,
                content,
                edits_list,
                ..Default::default()
            });
        }

        previous_timestamp = created_timestamp;
    }
    println!("time at matrix.rs: {}", (std::time::Instant::now() - time).as_millis());
    messages
}
