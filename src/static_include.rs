use std::{borrow::Cow, ffi::OsStr, path::PathBuf};

use rocket::{
    get,
    http::{ContentType, Status},
};
use rust_embed::RustEmbed;

#[derive(Clone, Copy, Debug, RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/static"]
#[exclude = "*.scss"]
#[exclude = "*.ts"]
struct Assets;

#[get("/<file..>", rank = 10)]
pub fn static_file<'r>(file: PathBuf) -> Result<(ContentType, Cow<'static, [u8]>), Status> {
    let filename = file.display().to_string();
    let d = Assets::get(&filename).ok_or(Status::NotFound)?;
    let ext = file
        .as_path()
        .extension()
        .and_then(OsStr::to_str)
        .ok_or(Status::InternalServerError)?;
    let content_type = ContentType::from_extension(ext).ok_or(Status::InternalServerError)?;
    Ok((content_type, d.data))
}
