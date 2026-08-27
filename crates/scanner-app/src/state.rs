use std::path::PathBuf;

use eframe::egui::TextureHandle;
use scanner_core::{
    EditState, ImageId, ImageMetadata, QueueStatus, SessionItem, SourceImage,
    display_name_for_path,
};

use crate::tasks::CancellationToken;

pub struct PreviewInfo {
    pub revision: u64,
    pub width: u32,
    pub height: u32,
}

pub struct QueueItem {
    pub id: ImageId,
    pub path: PathBuf,
    pub display_name: String,
    pub metadata: Option<ImageMetadata>,
    pub edit: EditState,
    pub status: QueueStatus,
    pub error: Option<String>,
    pub source_texture: Option<TextureHandle>,
    pub preview_texture: Option<TextureHandle>,
    pub preview: Option<PreviewInfo>,
    pub preview_cancellation: Option<CancellationToken>,
    history: Vec<EditState>,
    redo: Vec<EditState>,
    next_revision: u64,
}

impl QueueItem {
    pub fn new(path: PathBuf) -> Self {
        Self {
            id: ImageId::new(),
            display_name: display_name_for_path(&path),
            path,
            metadata: None,
            edit: EditState::default(),
            status: QueueStatus::Loading,
            error: None,
            source_texture: None,
            preview_texture: None,
            preview: None,
            preview_cancellation: None,
            history: Vec::new(),
            redo: Vec::new(),
            next_revision: 0,
        }
    }

    pub fn from_session(session: SessionItem) -> Self {
        let SourceImage {
            id,
            path,
            display_name,
            metadata,
        } = session.source;
        let next_revision = session.edit.revision;
        Self {
            id,
            path,
            display_name,
            metadata: Some(metadata),
            edit: session.edit,
            status: QueueStatus::Loading,
            error: None,
            source_texture: None,
            preview_texture: None,
            preview: None,
            preview_cancellation: None,
            history: Vec::new(),
            redo: Vec::new(),
            next_revision,
        }
    }

    pub fn set_loaded(&mut self, source: SourceImage) {
        if self.metadata.is_none() {
            self.edit = EditState::for_dimensions(
                source.metadata.width,
                source.metadata.height,
            );
            self.next_revision = self.edit.revision;
        }
        self.display_name = source.display_name;
        self.metadata = Some(source.metadata);
        self.status = QueueStatus::Ready;
        self.error = None;
    }

    pub fn record_edit(&mut self, before: EditState) -> bool {
        if self.edit == before {
            return false;
        }
        self.history.push(before);
        self.redo.clear();
        self.advance_revision();
        self.status = QueueStatus::Previewing;
        if let Some(token) = self.preview_cancellation.take() {
            token.cancel();
        }
        true
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.history.pop() else {
            return false;
        };
        self.redo.push(self.edit.clone());
        self.edit = previous;
        self.advance_revision();
        self.status = QueueStatus::Previewing;
        if let Some(token) = self.preview_cancellation.take() {
            token.cancel();
        }
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.history.push(self.edit.clone());
        self.edit = next;
        self.advance_revision();
        self.status = QueueStatus::Previewing;
        if let Some(token) = self.preview_cancellation.take() {
            token.cancel();
        }
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn reset_quadrilateral(&mut self) -> bool {
        let before = self.edit.clone();
        self.edit.quadrilateral = scanner_core::Quadrilateral::full_image();
        self.record_edit(before)
    }

    pub fn session_item(&self) -> Option<SessionItem> {
        Some(SessionItem {
            source: SourceImage {
                id: self.id,
                path: self.path.clone(),
                display_name: self.display_name.clone(),
                metadata: self.metadata.clone()?,
            },
            edit: self.edit.clone(),
            status: self.status,
        })
    }

    fn advance_revision(&mut self) {
        self.next_revision = self.next_revision.saturating_add(1);
        self.edit.revision = self.next_revision;
    }
}
