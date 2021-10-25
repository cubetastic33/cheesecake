use glob::glob;
use rusqlite::Connection;
use tempfile::NamedTempFile;
use directories::ProjectDirs;
use std::{collections::HashMap, path::Path, io::{self, Write}, path::PathBuf, env, fs};
use super::{discord, matrix, generic, DBFile};

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
    pub sequential_id: String, // This is useful because sometimes message_id is not sequential (like in matrix)
    pub message_id: String,
    pub message_type: String,
    pub name: String,
    pub avatar: String,
    pub color: String,
    pub bot: u8,
    pub created_timestamp: String,
    pub edited_timestamp: Option<String>,
    pub separate: bool,
    pub reference: Option<(String, String, String, String, String, bool)>, // sequential_id, name, avatar, color, message, attachments?
    pub content: String,
    pub edits_list: String,
    pub attachments: Vec<(String, String, bool)>, // source, type, spoiler?
    pub reactions: Vec<(String, Option<String>, usize)>, // name, source, count
}

impl Default for Message {
    fn default() -> Self {
        Message {
            sequential_id: String::new(),
            message_id: String::new(),
            message_type: String::new(),
            name: String::new(),
            avatar: String::new(),
            color: String::from("#afafaf"),
            bot: 0,
            created_timestamp: String::new(),
            edited_timestamp: None,
            separate: true,
            reference: None,
            content: String::new(),
            edits_list: String::new(),
            attachments: Vec::new(),
            reactions: Vec::new(),
        }
    }
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

pub fn refrigerator() -> String {
    // First priority is the refrigerator environment variable
    // If that doesn't exist, try a directory named `refrigerator` in the working directory
    let path = env::var("REFRIGERATOR").unwrap_or(String::from("refrigerator"));
    if Path::new(&path).is_dir() {
        // If the refrigerator path is a valid directory
        path
    } else {
        // If both of those failed, get the refrigerator path from the config file
        let project_dirs = ProjectDirs::from("", "", "cheesecake").unwrap();
        let config_dir = project_dirs.config_dir();
        let mut new_path = String::new();
        // The config file already exists
        if config_dir.join("refrigerator_path").exists() {
            // Read the config file
            new_path = fs::read_to_string(config_dir.join("refrigerator_path")).unwrap().trim().to_string();
            if Path::new(&new_path).is_dir() {
                // If the path specified in the config file is a valid directory, return it
                return new_path;
            }
        } else {
            // The config file doesn't exist yet
            // Get a path to test out
            print!("Path `{}` is not valid. Either create the directory and restart cheesecake, or provide a new path here: ", path);
            io::stdout().flush().unwrap();
            // Get the new path
            io::stdin().read_line(&mut new_path).unwrap();
        }
        // Keep asking for a new path until we get a valid one
        while !Path::new(new_path.trim()).exists() {
            print!("Path `{}` is not valid. Please enter a valid path: ", new_path.trim());
            io::stdout().flush().unwrap();
            // Reset `new_path`
            new_path = String::new();
            // Get the new path
            io::stdin().read_line(&mut new_path).unwrap();
        }
        // Store the new path in the config file
        fs::create_dir_all(config_dir).unwrap();
        fs::write(config_dir.join("refrigerator_path"), new_path.trim()).unwrap();
        return new_path.trim().to_string();
    }
}

pub fn day_separator(timestamp: chrono::DateTime<chrono::Local>) -> Message {
    Message {
        message_type: String::from("day_separator"),
        content: timestamp.date().format("%Y-%m-%d").to_string(),
        ..Default::default()
    }
}

fn backup_type(backup_path: &str) -> String {
    let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        Path::new(&refrigerator()).join(backup_path).join("info.json")
    ).unwrap()).unwrap();
    json["type"].as_str().unwrap().to_owned()
}

fn chat_list(conn: Connection) -> Vec<[String; 2]> {
    let mut chats = Vec::new();
    let mut statement = conn.prepare("SELECT id, name FROM chats").unwrap();
    let mut rows = statement.query([]).unwrap();
    while let Some(chat) = rows.next().unwrap() {
        chats.push([
            chat.get(0).unwrap(),
            chat.get(1).unwrap(),
        ]);
    }
    chats
}

// Creates context with information to select a chat from
pub fn selection_context<'a>(db_file: &'a DBFile, backup_path: &'a str, chat_id: &'a str) -> SelectionContext<'a> {
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
                if json["version"] == "0.1.0" && ["discord", "matrix", "generic"].contains(&json["type"].as_str().unwrap()) {
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

                    if json["salt"].is_string() && db_file.backup_path != current_backup_path {
                        // The backup is encrypted, insert an empty list of chats
                        mapped_chats.insert(current_backup_path, Vec::new());
                    } else {
                        let database_path;
                        if json["salt"].is_string() {
                            // The backup is decrypted
                            database_path = db_file.file.as_ref().unwrap().path().into();
                        } else {
                            database_path = path.parent().unwrap().join("backup.db");
                        }
                        // Get the list of chats
                        let conn = Connection::open(database_path).unwrap();
                        let chats = chat_list(conn);
                        // If there were no chats, don't include the backup
                        // We don't insert an empty list because that'll appear like an encrypted
                        // backup
                        if chats.len() > 0 {
                            mapped_chats.insert(current_backup_path, chats);
                        }
                    }
                }
            }
            Err(e) => println!("{:?}", e),
        }
    }
    let chats = if backups.len() == 0 {
        Vec::new()
    } else {
        mapped_chats
            .get(&backups[selected_backup][0])
            .unwrap()
            .to_owned()
    };
    SelectionContext {
        backup_path,
        chat_id,
        backups,
        chats,
        mapped_chats: serde_json::to_string(&mapped_chats).unwrap(),
    }
}

pub fn decrypt(db_file: &mut DBFile, password: &str) -> Vec<[String; 2]> {
    let info_path = Path::new(&refrigerator())
        .join(&db_file.backup_path)
        .join("info.json");
    let info: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&info_path).unwrap()).unwrap();
    if let Some(salt) = info["salt"].as_str() {
        if let Some(iterations) = info["iterations"].as_u64() {
            // Generate a key with the given password
            let mut key = [0; 32];
            openssl::pkcs5::pbkdf2_hmac(
                password.as_bytes(),
                base64::decode_config(salt, base64::URL_SAFE).unwrap().as_slice(),
                iterations as usize,
                openssl::hash::MessageDigest::sha256(),
                &mut key,
            ).unwrap();

            // It'll be None if the generated key was wrong
            if let Some(fernet) = fernet::Fernet::new(&base64::encode(key)) {
                let ciphertext = std::fs::read_to_string(info_path.parent().unwrap().join("backup.db")).unwrap();
                let mut file = NamedTempFile::new().unwrap();
                if let Ok(decrypted) = fernet.decrypt(&ciphertext) {
                    // If the file was successfully decrypted with the generated key
                    file.write_all(decrypted.as_slice()).unwrap();
                    // Open a connection to the decrypted database
                    let conn = Connection::open(file.path()).unwrap();
                    // Store the NamedTempFile instance to State so that the file doesn't get destroyed
                    db_file.file = Some(file);
                    // Return the list of chats
                    return chat_list(conn);
                }
            }
        }
    }
    Vec::new()
}

fn database_path(db_file: &Option<NamedTempFile>, backup_path: &str) -> PathBuf {
    match db_file {
        Some(file) => file.path().into(),
        None => Path::new(&refrigerator()).join(backup_path).join("backup.db"),
    }
}

pub fn chat<'a>(db_file: &'a DBFile, backup_path: &'a str, chat_id: &'a str) -> ChatContext<'a> {
    let populate_messages = match backup_type(backup_path).as_str() {
        "discord" => discord::populate_messages,
        "matrix" => matrix::populate_messages,
        "generic" => generic::populate_messages,
        _ => return ChatContext::default(),
    };
    let database_path = &database_path(&db_file.file, backup_path);
    let messages = populate_messages(
        database_path,
        backup_path,
        "SELECT * FROM ({} chat = $1 ORDER BY created_timestamp DESC LIMIT 100) ORDER BY created_timestamp",
        &[&chat_id],
    );
    // Create a connection to the database
    let conn = Connection::open(database_path).unwrap();
    // Get the chat name and topic
    let mut statement = conn
        .prepare("SELECT name, topic FROM chats WHERE id = $1")
        .unwrap();
    let mut rows = statement.query([chat_id]).unwrap();
    let chat_details = rows.next().unwrap().unwrap();
    // Return the ChatContext
    ChatContext {
        name: chat_details.get(0).unwrap(),
        topic: chat_details.get(1).unwrap_or(String::new()),
        messages,
        selection_context: Some(selection_context(db_file, backup_path, chat_id)),
    }
}

pub fn jump_chat<'a>(db_file: &'a Option<NamedTempFile>, backup_path: &'a str, chat_id: &'a str, message_id: &Option<String>) -> ChatContext<'static> {
    let populate_messages = match backup_type(backup_path).as_str() {
        "discord" => discord::populate_messages,
        "matrix" => matrix::populate_messages,
        "generic" => generic::populate_messages,
        _ => return ChatContext::default(),
    };
    // Create a connection to the database
    let database_path = &database_path(db_file, backup_path);
    let conn = Connection::open(database_path).unwrap();
    // Get the chat name and topic
    let mut statement = conn
        .prepare("SELECT name, topic FROM chats WHERE id = $1")
        .unwrap();
    let mut rows = statement.query([chat_id]).unwrap();
    match rows.next().unwrap() {
        Some(chat_details) => {
            let messages = match message_id {
                Some(id) => {
                    // Get sequential ID from message ID
                    let mut statement = conn
                        .prepare("SELECT ROWID FROM messages WHERE id = $1")
                        .unwrap();
                    let mut rows = statement.query(&[&id]).unwrap();
                    let row = rows.next().unwrap().unwrap();
                    let sequential_id: u64 = row.get(0).unwrap();
                    populate_messages(
                        database_path,
                        backup_path,
                        "SELECT * FROM ({} chat = $1 AND ROWID <= $2 ORDER BY created_timestamp DESC LIMIT 50)
                        UNION SELECT * FROM ({} chat = $1 AND ROWID > $2 ORDER BY created_timestamp LIMIT 50) ORDER BY created_timestamp",
                        &[&chat_id, &sequential_id]
                    )
                },
                None => populate_messages(
                    database_path,
                    backup_path,
                    "SELECT * FROM ({} chat = $1 ORDER BY created_timestamp DESC LIMIT 100) ORDER BY created_timestamp",
                    &[&chat_id]
                ),
            };

            ChatContext {
                name: chat_details.get(0).unwrap(),
                topic: chat_details.get(1).unwrap_or(String::new()),
                messages,
                selection_context: None,
            }
        },
        // If the chat doesn't exist in the database
        None => ChatContext::default(),
    }
}

pub fn get_messages<'a>(
    db_file: &'a Option<NamedTempFile>,
    backup_path: &'a str,
    chat_id: &'a str,
    sequential_id: u64,
    position: &'a str,
) -> Vec<Message> {
    let condition = if position == "above" {
        "SELECT * FROM ({} chat = $1 AND ROWID < $2 ORDER BY created_timestamp DESC LIMIT 100) ORDER BY created_timestamp"
    } else if position == "below" {
        "{} chat = $1 AND ROWID > $2 ORDER BY created_timestamp LIMIT 100"
    } else {
        "SELECT * FROM ({} chat = $1 AND ROWID <= $2 ORDER BY created_timestamp DESC LIMIT 50) UNION SELECT * FROM ({} chat = $1 AND ROWID > $2 ORDER BY created_timestamp LIMIT 50) ORDER BY created_timestamp"
    };
    let populate_messages = match backup_type(backup_path).as_str() {
        "discord" => discord::populate_messages,
        "matrix" => matrix::populate_messages,
        "generic" => generic::populate_messages,
        _ => return Vec::new(),
    };
    populate_messages(
        &database_path(db_file, backup_path),
        backup_path,
        condition,
        &[&chat_id, &sequential_id]
    )
}

pub fn search<'a>(db_file: &'a Option<NamedTempFile>, backup_path: &'a str, chat_id: &'a str, query: &str, mut filters: &str) -> Vec<Message> {
    if filters.len() == 0 {
        filters = "TRUE";
    }
    let populate_messages = match backup_type(backup_path).as_str() {
        "discord" => discord::populate_messages,
        "matrix" => matrix::populate_messages,
        "generic" => generic::populate_messages,
        _ => return Vec::new(),
    };
    populate_messages(
        &database_path(db_file, backup_path),
        backup_path,
        &("{} chat = $1 AND id IN (SELECT id FROM message_search WHERE message_search MATCH $2 ORDER BY rank) AND ".to_owned() + filters),
        &[&chat_id, &query]
    )
}
