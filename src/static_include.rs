use std::path::Path;

use rocket::{
    handler::Outcome,
    http::{uri::Segments, ContentType, Status},
    outcome::IntoOutcome,
    response,
    Data, Handler, Request,
};

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticFiles<const N: usize> {
    pub files: [File; N],
    pub rank: isize,
    pub root: &'static Path,
}

impl<const N: usize> StaticFiles<N> {
    pub fn rank(self, rank: isize) -> Self {
        StaticFiles { rank, ..self }
    }

    pub fn root(self, root: &'static Path) -> Self {
        StaticFiles { root, ..self }
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct File {
    pub path: &'static Path,
    pub body: &'static [u8],
}

#[macro_export]
macro_rules! include_static {
    ($($path:literal),*) => {
        crate::static_include::StaticFiles {
            files: [
                $(
                    crate::static_include::File {
                        name: std::path::Path::new($path),
                        body: include_bytes!(concat![env!("CARGO_MANIFEST_DIR"), "/", $path])
                    }
                ),*
            ],
            rank: 10,
            root: Path::new("static"),
        }
    }
}

impl<'r> response::Responder<'r> for File {
    fn respond_to(self, req: &Request) -> response::Result<'r> {
        let mut response = self.body.respond_to(req)?;
        if let Some(ext) = self.path.extension() {
            if let Some(ct) = ContentType::from_extension(&ext.to_string_lossy()) {
                response.set_header(ct);
            }
        }
        Ok(response)
    }
}

impl<const N: usize> Handler for StaticFiles<N> {
    fn handle<'r>(&self, request: &'r Request, _data: Data) -> Outcome<'r> {
        let req_path = request
            .get_segments::<Segments>(0)
            .and_then(Result::ok)
            .and_then(|segments| segments.into_path_buf(false).ok())
            .map(|path| self.root.join(path))
            .filter(|path| path.is_file())
            .into_outcome(Status::NotFound)?;

        Outcome::from(
            request,
            self.files
                .iter()
                .copied()
                .find(|File { path, .. }| &req_path == path),
        )
    }
}
