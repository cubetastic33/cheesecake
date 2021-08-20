#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate serde_derive;

use dotenv::dotenv;
use rocket::{http::Cookies, request::Form, response::Redirect, Config, State};
use rocket_contrib::{json::Json, serve::StaticFiles, templates::Template};
use tempfile::NamedTempFile;
use std::sync::Mutex;

mod actions;
mod discord;
mod matrix;
mod generic;
mod static_include;

#[derive(FromForm)]
pub struct Password {
    password: String,
}

#[derive(FromForm)]
pub struct JumpDetails {
    chat_id: Option<String>,
    message_id: Option<String>,
}

#[derive(FromForm)]
pub struct GetMessages {
    sequential_id: u64,
    position: String,
}

#[derive(FromForm)]
pub struct Query {
    string: String,
    filters: String,
}

pub struct DBFile {
    backup_path: String,
    file: Option<NamedTempFile>,
}

impl DBFile {
    fn reset(&mut self) {
        self.backup_path = String::new();
        self.file = None;
    }
}

#[get("/")]
fn get_index(db_file: State<Mutex<DBFile>>, cookies: Cookies) -> Template {
    let mut backup_path = "";
    let mut chat_id = "";
    if let Some(backup) = cookies.get("backup") {
        backup_path = backup.value();
        if let Some(chat) = cookies.get("chat") {
            chat_id = chat.value();
        }
    }
    Template::render("index", actions::selection_context(&db_file.lock().unwrap(), backup_path, chat_id))
}

#[get("/reader")]
fn get_reader(db_file: State<Mutex<DBFile>>, cookies: Cookies) -> Result<Template, Redirect> {
    if let Some(backup) = cookies.get("backup") {
        if backup.value() != db_file.lock().unwrap().backup_path {
            // This is not a decrypted backup
            db_file.lock().unwrap().reset();
        }
        if let Some(chat) = cookies.get("chat") {
            return Ok(Template::render(
                "reader",
                actions::chat(&db_file.lock().unwrap(), backup.value(), chat.value()),
            ));
        }
    }
    Err(Redirect::to("/"))
}

// POST requests

#[post("/decrypt", data = "<password>")]
fn post_decrypt(db_file: State<Mutex<DBFile>>, cookies: Cookies, password: Form<Password>) -> Json<Vec<[String; 2]>> {
    if let Some(backup) = cookies.get("backup") {
        db_file.lock().unwrap().backup_path = backup.value().to_owned();
        return Json(actions::decrypt(&mut db_file.lock().unwrap(), &password.password));
    }
    Json(Vec::new())
}

#[post("/jump", data = "<info>")]
fn post_jump<'a>(db_file: State<Mutex<DBFile>>, cookies: Cookies, info: Form<JumpDetails>) -> Json<actions::ChatContext<'a>> {
    if let Some(backup) = cookies.get("backup") {
        if backup.value() != db_file.lock().unwrap().backup_path {
            // This is not a decrypted backup
            db_file.lock().unwrap().reset();
        }
        // If the chat ID is not specified, take the current chat
        let chat_id = match &info.chat_id {
            Some(chat_id) => chat_id,
            None => match cookies.get("chat") {
                Some(chat) => chat.value(),
                None => return Json(actions::ChatContext::default()),
            }
        };
        let messages = Json(actions::jump_chat(&db_file.lock().unwrap().file, backup.value(), chat_id, &info.message_id));
        return messages;
    }
    Json(actions::ChatContext::default())
}

// Used for getting the messages around a specific message ID
#[post("/messages", data = "<info>")]
fn post_messages(db_file: State<Mutex<DBFile>>, cookies: Cookies, info: Form<GetMessages>) -> Json<Vec<actions::Message>> {
    let time = std::time::Instant::now();
    if let Some(backup) = cookies.get("backup") {
        if backup.value() != db_file.lock().unwrap().backup_path {
            // This is not a decrypted backup
            db_file.lock().unwrap().reset();
        }
        if let Some(chat) = cookies.get("chat") {
            // The required cookies are present, so return the messages
            let messages = actions::get_messages(
                &db_file.lock().unwrap().file,
                backup.value(),
                chat.value(),
                info.sequential_id,
                &info.position,
            );
            println!("time at main.rs: {}", (std::time::Instant::now() - time).as_millis());
            return Json(messages);
        }
    }
    // Return an empty vector by default
    Json(Vec::new())
}

#[post("/search", data = "<query>")]
fn post_search(db_file: State<Mutex<DBFile>>, cookies: Cookies, query: Form<Query>) -> Json<Vec<actions::Message>> {
    if let Some(backup) = cookies.get("backup") {
        if backup.value() != db_file.lock().unwrap().backup_path {
            // This is not a decrypted backup
            db_file.lock().unwrap().reset();
        }
        if let Some(chat) = cookies.get("chat") {
            // The required cookies are present, so return the search results
            return Json(actions::search(&db_file.lock().unwrap().file, backup.value(), chat.value(), &query.string, &query.filters));
        }
    }
    // Return an empty vector by default
    Json(Vec::new())
}

fn configure() -> Config {
    let mut config = Config::active().expect("could not load configuration");
    config.set_port(4000);
    config
}

fn rocket() -> rocket::Rocket {
    rocket::custom(configure())
        .mount(
            "/",
            routes![get_index, get_reader, post_decrypt, post_jump, post_messages, post_search],
        )
        .mount("/styles", StaticFiles::from("static/styles"))
        .mount("/scripts", StaticFiles::from("static/scripts"))
        .mount("/fonts", StaticFiles::from("static/fonts"))
        .mount("/images", StaticFiles::from("static/images"))
        .mount("/", StaticFiles::from(actions::refrigerator()).rank(20))
        .attach(Template::fairing())
}

fn main() {
    // Read environment variables from .env
    dotenv().ok();
    // Start the webserver
    rocket().manage(Mutex::new(DBFile {backup_path: String::new(), file: None})).launch();
}
