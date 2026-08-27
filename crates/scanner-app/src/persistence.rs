use std::{fs, path::Path};

use anyhow::{Context, Result, anyhow};
use scanner_core::{
    SessionDocument, SessionItem, decode_session, encode_session,
};

use crate::state::QueueItem;

pub fn document_from_items(items: &[QueueItem]) -> Result<SessionDocument> {
    let mut session_items = Vec::with_capacity(items.len());
    for item in items {
        let session_item = item.session_item().ok_or_else(|| {
            anyhow!(
                "{} is not loaded yet; wait for import to finish before saving",
                item.display_name
            )
        })?;
        session_items.push(session_item);
    }
    Ok(SessionDocument {
        schema_version: scanner_core::model::CURRENT_SESSION_SCHEMA,
        items: session_items,
    })
}

pub fn save(path: &Path, items: &[QueueItem]) -> Result<()> {
    let document = document_from_items(items)?;
    let contents = encode_session(&document).context("encoding session")?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).context("creating session directory")?;
    let temporary = path.with_extension("scanner-session.tmp");
    fs::write(&temporary, contents).context("writing session file")?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error).context("replacing session file");
    }
    Ok(())
}

pub fn load(path: &Path) -> Result<Vec<SessionItem>> {
    let contents = fs::read_to_string(path).context("reading session file")?;
    let document = decode_session(&contents).context("decoding session")?;
    Ok(document.items)
}
