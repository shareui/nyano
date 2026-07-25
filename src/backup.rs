use std::io;
use std::path::Path;

// cp curr file as(s) file+.bak
pub fn create_backup(path: &Path, saved_text: &str) -> io::Result<()> {
    let mut backup_path = path.as_os_str().to_os_string();
    backup_path.push(".bak");
    std::fs::write(backup_path, saved_text)
}
