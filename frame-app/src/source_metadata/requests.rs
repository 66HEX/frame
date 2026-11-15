use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use frame_core::error::ConversionError;

use super::SourceMetadata;

#[derive(Default)]
pub struct SourceMetadataProbes {
    active: HashMap<String, SourceMetadataProbe>,
}

#[derive(Clone)]
pub struct SourceMetadataProbe {
    pub(crate) file_id: String,
    pub(crate) file_path: String,
    cancelled: Arc<AtomicBool>,
}

impl SourceMetadataProbes {
    pub(crate) fn begin(&mut self, file_id: String, file_path: String) -> SourceMetadataProbe {
        self.cancel(&file_id);
        let request = SourceMetadataProbe {
            file_id: file_id.clone(),
            file_path,
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        self.active.insert(file_id, request.clone());
        request
    }

    pub(crate) fn cancel(&mut self, file_id: &str) {
        if let Some(request) = self.active.remove(file_id) {
            request.cancelled.store(true, Ordering::Release);
        }
    }

    pub(crate) fn cancel_all(&mut self) {
        for (_, request) in self.active.drain() {
            request.cancelled.store(true, Ordering::Release);
        }
    }

    pub(crate) fn finish(&mut self, request: &SourceMetadataProbe) -> bool {
        if !self.active.get(&request.file_id).is_some_and(|active| {
            Arc::ptr_eq(&active.cancelled, &request.cancelled)
                && !request.cancelled.load(Ordering::Acquire)
        }) {
            return false;
        }
        self.active.remove(&request.file_id);
        true
    }
}

impl Drop for SourceMetadataProbes {
    fn drop(&mut self) {
        self.cancel_all();
    }
}

impl SourceMetadataProbe {
    pub(crate) fn run_with(
        &self,
        probe: impl FnOnce(&str, &dyn Fn() -> bool) -> Result<SourceMetadata, ConversionError>,
    ) -> Result<SourceMetadata, ConversionError> {
        probe(&self.file_path, &|| self.cancelled.load(Ordering::Acquire))
    }
}
