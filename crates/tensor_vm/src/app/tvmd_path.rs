use std::path::Path;

pub(super) fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
