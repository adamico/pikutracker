use super::exporting::FinishedExport;
use super::files::{FileRequest, FileResponse};
use crate::project::Project;
use std::path::PathBuf;

/// What the App asked a file for, handed back with the answer.
#[derive(Debug)]
pub(super) enum Asked {
    Listing,
    Opening,
    Importing,
    Saving(Box<Project>),
    ExportReading,
    ExportWriting(FinishedExport),
    CartViewReading,
    PickingCart,
    MakingCart,
    SettingAsideState { state_file: PathBuf },
    SettingAsideConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Success {
    Listed,
    Read,
    BytesRead,
    Written,
    Renamed,
}

#[derive(Debug)]
struct Conversation {
    success: Success,
    succeeded_at: PathBuf,
    failed_at: PathBuf,
    asked: Asked,
}

impl Conversation {
    fn answered_by(&self, response: &FileResponse) -> bool {
        let (success, path) = match response {
            FileResponse::Listed { directory, .. } => (Success::Listed, directory),
            FileResponse::Read { path, .. } => (Success::Read, path),
            FileResponse::BytesRead { path, .. } => (Success::BytesRead, path),
            FileResponse::Written { path } => (Success::Written, path),
            FileResponse::Renamed { path } => (Success::Renamed, path),
            FileResponse::Missing { path } | FileResponse::Failed { path, .. } => {
                return *path == self.failed_at;
            }
        };
        success == self.success && *path == self.succeeded_at
    }
}

/// The file requests the App has sent and is still waiting on. Any number
/// can be outstanding; each response goes back with what its request was for.
#[derive(Debug, Default)]
pub(super) struct FileConversations {
    requests: Vec<FileRequest>,
    open: Vec<Conversation>,
}

impl FileConversations {
    pub(super) fn ask(&mut self, request: FileRequest, asked: Asked) {
        let (success, succeeded_at, failed_at) = match &request {
            FileRequest::List { directory } => (Success::Listed, directory, directory),
            FileRequest::Read { path } => (Success::Read, path, path),
            FileRequest::ReadBytes { path } => (Success::BytesRead, path, path),
            FileRequest::Write { path, .. } | FileRequest::WriteBytes { path, .. } => {
                (Success::Written, path, path)
            }
            FileRequest::Rename { from, to } => (Success::Renamed, to, from),
        };
        self.open.push(Conversation {
            success,
            succeeded_at: succeeded_at.clone(),
            failed_at: failed_at.clone(),
            asked,
        });
        self.requests.push(request);
    }

    /// Sends a request whose answer nobody waits for.
    pub(super) fn tell(&mut self, request: FileRequest) {
        self.requests.push(request);
    }

    pub(super) fn take_requests(&mut self) -> Vec<FileRequest> {
        std::mem::take(&mut self.requests)
    }

    pub(super) fn waiting(&self) -> bool {
        !self.open.is_empty()
    }

    /// Ends the oldest conversation this response answers and says what it
    /// was for, or `None` when nothing was waiting on it.
    pub(super) fn answer(&mut self, response: &FileResponse) -> Option<Asked> {
        let index = self
            .open
            .iter()
            .position(|conversation| conversation.answered_by(response))?;
        Some(self.open.remove(index).asked)
    }
}
