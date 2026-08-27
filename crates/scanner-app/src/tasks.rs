use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

use scanner_core::{
    CollisionPolicy, DEFAULT_PREVIEW_MAX_DIMENSION, EditState, ExportResult,
    ImageId, LoadedSource, ProcessingMode, ProcessingResult, export_image,
    load_image, process_image,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

enum WorkerCommand {
    Load {
        item_id: ImageId,
        path: PathBuf,
    },
    Preview {
        item_id: ImageId,
        revision: u64,
        path: PathBuf,
        edit: EditState,
        cancellation: CancellationToken,
    },
    Export {
        task_id: TaskId,
        item_id: ImageId,
        path: PathBuf,
        destination: PathBuf,
        edit: EditState,
        cancellation: CancellationToken,
    },
}

#[derive(Debug)]
pub enum WorkerEvent {
    Loaded {
        item_id: ImageId,
        result: std::result::Result<LoadedSource, String>,
    },
    PreviewReady {
        item_id: ImageId,
        revision: u64,
        result: std::result::Result<ProcessingResult, String>,
    },
    PreviewCancelled {
        item_id: ImageId,
        revision: u64,
    },
    ExportProgress {
        task_id: TaskId,
        item_id: ImageId,
        progress: f32,
    },
    ExportFinished {
        task_id: TaskId,
        item_id: ImageId,
        result: std::result::Result<ExportResult, String>,
    },
    ExportCancelled {
        task_id: TaskId,
        item_id: ImageId,
    },
}

pub struct TaskRunner {
    commands: Sender<WorkerCommand>,
    events: Receiver<WorkerEvent>,
    next_task_id: u64,
}

impl TaskRunner {
    pub fn new() -> std::io::Result<Self> {
        let (command_sender, command_receiver) = mpsc::channel();
        let (event_sender, event_receiver) = mpsc::channel();
        thread::Builder::new()
            .name("scanner-worker".to_owned())
            .spawn(move || worker_loop(command_receiver, event_sender))?;
        Ok(Self {
            commands: command_sender,
            events: event_receiver,
            next_task_id: 0,
        })
    }

    pub fn load(&self, item_id: ImageId, path: PathBuf) {
        let _ = self.commands.send(WorkerCommand::Load { item_id, path });
    }

    pub fn preview(
        &self,
        item_id: ImageId,
        revision: u64,
        path: PathBuf,
        edit: EditState,
    ) -> CancellationToken {
        let cancellation = CancellationToken::new();
        let command = WorkerCommand::Preview {
            item_id,
            revision,
            path,
            edit,
            cancellation: cancellation.clone(),
        };
        let _ = self.commands.send(command);
        cancellation
    }

    pub fn export(
        &mut self,
        item_id: ImageId,
        path: PathBuf,
        destination: PathBuf,
        edit: EditState,
    ) -> (TaskId, CancellationToken) {
        self.next_task_id = self.next_task_id.saturating_add(1);
        let task_id = TaskId(self.next_task_id);
        let cancellation = CancellationToken::new();
        let command = WorkerCommand::Export {
            task_id,
            item_id,
            path,
            destination,
            edit,
            cancellation: cancellation.clone(),
        };
        let _ = self.commands.send(command);
        (task_id, cancellation)
    }

    pub fn drain_events(&self) -> Vec<WorkerEvent> {
        self.events.try_iter().collect()
    }
}

fn worker_loop(receiver: Receiver<WorkerCommand>, sender: Sender<WorkerEvent>) {
    while let Ok(command) = receiver.recv() {
        let should_continue = match command {
            WorkerCommand::Load { item_id, path } => {
                let result = load_image(&path)
                    .map(|mut loaded| {
                        loaded.source.id = item_id;
                        loaded
                    })
                    .map_err(|error| error.to_string());
                sender.send(WorkerEvent::Loaded { item_id, result }).is_ok()
            }
            WorkerCommand::Preview {
                item_id,
                revision,
                path,
                edit,
                cancellation,
            } => {
                if cancellation.is_cancelled() {
                    sender
                        .send(WorkerEvent::PreviewCancelled {
                            item_id,
                            revision,
                        })
                        .is_ok()
                } else {
                    let result = process_image(
                        &path,
                        &edit,
                        ProcessingMode::Preview {
                            max_dimension: DEFAULT_PREVIEW_MAX_DIMENSION,
                        },
                    )
                    .map_err(|error| error.to_string());
                    if cancellation.is_cancelled() {
                        sender
                            .send(WorkerEvent::PreviewCancelled {
                                item_id,
                                revision,
                            })
                            .is_ok()
                    } else {
                        sender
                            .send(WorkerEvent::PreviewReady {
                                item_id,
                                revision,
                                result,
                            })
                            .is_ok()
                    }
                }
            }
            WorkerCommand::Export {
                task_id,
                item_id,
                path,
                destination,
                edit,
                cancellation,
            } => {
                if cancellation.is_cancelled() {
                    sender
                        .send(WorkerEvent::ExportCancelled { task_id, item_id })
                        .is_ok()
                } else if sender
                    .send(WorkerEvent::ExportProgress {
                        task_id,
                        item_id,
                        progress: 0.0,
                    })
                    .is_err()
                {
                    false
                } else {
                    let result = export_image(
                        &path,
                        &edit,
                        &destination,
                        CollisionPolicy::AutoRename,
                    )
                    .map_err(|error| error.to_string());
                    if cancellation.is_cancelled() {
                        if let Ok(export) = &result {
                            let _ = fs::remove_file(&export.path);
                        }
                        sender
                            .send(WorkerEvent::ExportCancelled {
                                task_id,
                                item_id,
                            })
                            .is_ok()
                    } else {
                        match result {
                            Ok(result) => {
                                if sender
                                    .send(WorkerEvent::ExportProgress {
                                        task_id,
                                        item_id,
                                        progress: 1.0,
                                    })
                                    .is_err()
                                {
                                    false
                                } else {
                                    sender
                                        .send(WorkerEvent::ExportFinished {
                                            task_id,
                                            item_id,
                                            result: Ok(result),
                                        })
                                        .is_ok()
                                }
                            }
                            Err(error) => sender
                                .send(WorkerEvent::ExportFinished {
                                    task_id,
                                    item_id,
                                    result: Err(error),
                                })
                                .is_ok(),
                        }
                    }
                }
            }
        };
        if !should_continue {
            break;
        }
    }
}
