use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

// cp curr file as(s) file+.<timestamp>.bak, unique per save so earlier backups survive
pub fn create_backup(path: &Path, saved_text: &str) -> io::Result<()> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut backup_path = path.as_os_str().to_os_string();
    backup_path.push(format!(".{ts}.bak"));
    std::fs::write(backup_path, saved_text)
}
