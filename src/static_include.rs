use std::ffi::OsStr;

#[non_exhaustive]
pub struct StaticFiles<const N: usize> {
    pub files: [File; N],
    pub rank: isize,
}

#[non_exhaustive]
pub struct File {
    pub name: &'static OsStr,
    pub body: &'static [u8],
}

#[macro_export]
macro_rules! include_static {
    ($($path:literal),*) => {
        crate::static_include::StaticFiles {
            files: [
                $(
                    crate::static_include::File {
                        name: std::path::Path::new($path).file_name().unwrap(),
                        body: include_bytes!(concat![env!("CARGO_MANIFEST_DIR"), "/", $path])
                    }
                ),*
            ],
            rank: 10
        }
    }
}
