#[macro_use]
extern crate rocket;
#[macro_use]
extern crate serde_derive;

use dotenv::dotenv;
use rocket::{http::CookieJar, form::Form, response::Redirect, serde::json::Json, fs::{FileServer, relative}, Config, State};
use rocket_dyn_templates::{Template, tera::Tera};
use tempfile::NamedTempFile;
use std::sync::Mutex;

use crate::static_include::static_file;

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
fn get_index(db_file: &State<Mutex<DBFile>>, cookies: &CookieJar<'_>) -> Template {
    let mut backup_path = "";
    let mut chat_id = "";
    if let Some(backup) = cookies.get("backup") {
        backup_path = backup.value();
        if let Some(chat) = cookies.get("chat") {
            chat_id = chat.value();
        }
    }
    Template::render("index.html", actions::selection_context(&db_file.lock().unwrap(), backup_path, chat_id))
}

#[get("/reader")]
fn get_reader(db_file: &State<Mutex<DBFile>>, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    if let Some(backup) = cookies.get("backup") {
        if backup.value() != db_file.lock().unwrap().backup_path {
            // This is not a decrypted backup
            db_file.lock().unwrap().reset();
        }
        if let Some(chat) = cookies.get("chat") {
            return Ok(Template::render(
                "reader.html",
                actions::chat(&db_file.lock().unwrap(), backup.value(), chat.value()),
            ));
        }
    }
    Err(Redirect::to("/"))
}

// POST requests

#[post("/decrypt", data = "<password>")]
fn post_decrypt(db_file: &State<Mutex<DBFile>>, cookies: &CookieJar<'_>, password: Form<Password>) -> Json<Vec<[String; 2]>> {
    if let Some(backup) = cookies.get("backup") {
        db_file.lock().unwrap().backup_path = backup.value().to_owned();
        return Json(actions::decrypt(&mut db_file.lock().unwrap(), &password.password));
    }
    Json(Vec::new())
}

#[post("/jump", data = "<info>")]
fn post_jump<'a>(db_file: &State<Mutex<DBFile>>, cookies: &CookieJar<'_>, info: Form<JumpDetails>) -> Json<actions::ChatContext<'a>> {
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
fn post_messages(db_file: &State<Mutex<DBFile>>, cookies: &CookieJar<'_>, info: Form<GetMessages>) -> Json<Vec<actions::Message>> {
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
fn post_search(db_file: &State<Mutex<DBFile>>, cookies: &CookieJar<'_>, query: Form<Query>) -> Json<Vec<actions::Message>> {
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

fn customize(tera: &mut Tera) {
    tera.add_raw_templates([
        ("index.html", include_str!("../templates/index.html.tera")),
        ("reader.html", include_str!("../templates/reader.html.tera")),
    ]).unwrap();
}

#[launch]
fn rocket() -> _ {
    // Read environment variables from .env
    dotenv().ok();
    // Configure rocket
    let figment = Config::figment().merge(("port", 4000)).merge(("template_dir", "."));
    // Start the webserver
    rocket::custom(figment)
        .mount(
            "/",
            routes![
                get_index,
                get_reader,
                post_decrypt,
                post_jump,
                post_messages,
                post_search,
                static_file
            ],
        )
        .attach(Template::custom(|engines| customize(&mut engines.tera)))
        .manage(Mutex::new(DBFile {backup_path: String::new(), file: None}))
}
