use std::sync::{atomic::AtomicBool, Arc};

use crate::providers::{gemini, openai, soniox};

/// Shared surface for provider-specific live bridge handles.
pub trait LiveBridge: Send + Sync {
    fn send_audio(&self, pcm: &[i16]);
    fn is_ready(&self) -> bool;
    fn ready_flag(&self) -> Arc<AtomicBool>;
}

#[derive(Clone)]
pub enum LiveBridgeHandle {
    Gemini(gemini::GeminiBridgeHandle),
    OpenAi(openai::OpenAiBridgeHandle),
    Soniox(soniox::SonioxBridgeHandle),
}

impl LiveBridgeHandle {
    pub fn send_audio(&self, pcm: &[i16]) {
        match self {
            Self::Gemini(handle) => handle.send_audio(pcm),
            Self::OpenAi(handle) => handle.send_audio(pcm),
            Self::Soniox(handle) => handle.send_audio(pcm),
        }
    }

    pub fn is_ready(&self) -> bool {
        match self {
            Self::Gemini(handle) => handle.is_ready(),
            Self::OpenAi(handle) => handle.is_ready(),
            Self::Soniox(handle) => handle.is_ready(),
        }
    }

    pub fn ready_flag(&self) -> Arc<AtomicBool> {
        match self {
            Self::Gemini(handle) => handle.ready_flag(),
            Self::OpenAi(handle) => handle.ready_flag(),
            Self::Soniox(handle) => handle.ready_flag(),
        }
    }

    pub async fn stop(self) {
        match self {
            Self::Gemini(handle) => handle.stop().await,
            Self::OpenAi(handle) => handle.stop().await,
            Self::Soniox(handle) => handle.stop().await,
        }
    }

    pub async fn abort(self) {
        match self {
            Self::Gemini(handle) => handle.abort().await,
            Self::OpenAi(handle) => handle.abort().await,
            Self::Soniox(handle) => handle.abort().await,
        }
    }
}

impl LiveBridge for LiveBridgeHandle {
    fn send_audio(&self, pcm: &[i16]) {
        LiveBridgeHandle::send_audio(self, pcm);
    }

    fn is_ready(&self) -> bool {
        LiveBridgeHandle::is_ready(self)
    }

    fn ready_flag(&self) -> Arc<AtomicBool> {
        LiveBridgeHandle::ready_flag(self)
    }
}

impl LiveBridge for gemini::GeminiBridgeHandle {
    fn send_audio(&self, pcm: &[i16]) {
        self.send_audio(pcm);
    }

    fn is_ready(&self) -> bool {
        self.is_ready()
    }

    fn ready_flag(&self) -> Arc<AtomicBool> {
        self.ready_flag()
    }
}

impl LiveBridge for openai::OpenAiBridgeHandle {
    fn send_audio(&self, pcm: &[i16]) {
        self.send_audio(pcm);
    }

    fn is_ready(&self) -> bool {
        self.is_ready()
    }

    fn ready_flag(&self) -> Arc<AtomicBool> {
        self.ready_flag()
    }
}

impl LiveBridge for soniox::SonioxBridgeHandle {
    fn send_audio(&self, pcm: &[i16]) {
        self.send_audio(pcm);
    }

    fn is_ready(&self) -> bool {
        self.is_ready()
    }

    fn ready_flag(&self) -> Arc<AtomicBool> {
        self.ready_flag()
    }
}
