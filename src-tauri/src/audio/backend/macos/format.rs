//! Audio buffer extraction from Core Audio `AudioBufferList`.

use std::mem;

use objc2_core_audio_types::AudioBufferList;

use super::super::pcm::{
    float_buffer_to_mono_f32_into, float_buffer_to_mono_i16_into,
    float_channel_buffers_to_mono_f32_into, float_channel_buffers_to_mono_i16_into,
    write_mono_f32_as_float_interleaved, write_mono_i16_as_float_interleaved,
};

/// Reusable scratch for non-interleaved input reads (avoids per-callback `Vec` alloc).
#[derive(Default)]
pub struct MonoReadScratch {
    channel_views: Vec<(*const f32, usize)>,
    frame_scratch: Vec<f32>,
}

impl MonoReadScratch {
    pub fn with_capacity(channels: usize) -> Self {
        Self {
            channel_views: Vec::with_capacity(channels.max(1)),
            frame_scratch: Vec::with_capacity(channels.max(1)),
        }
    }

    fn collect_non_interleaved(&mut self, buffer_list: &AudioBufferList) {
        self.channel_views.clear();
        let num_buffers = buffer_list.mNumberBuffers as usize;
        for i in 0..num_buffers {
            let buf = &buffer_list.mBuffers[i];
            if buf.mData.is_null() || buf.mDataByteSize == 0 {
                continue;
            }
            let count = buf.mDataByteSize as usize / mem::size_of::<f32>();
            self.channel_views.push((buf.mData as *const f32, count));
        }
    }
}

/// Read interleaved or non-interleaved float32 samples from an input buffer list.
///
/// # Safety
/// `buffer_list` must describe valid Core Audio buffers; each non-null `mData`
/// pointer must remain valid for the duration of this call.
pub unsafe fn read_input_mono_i16(buffer_list: &AudioBufferList) -> Vec<i16> {
    let mut out = Vec::new();
    let mut scratch = MonoReadScratch::default();
    read_input_mono_i16_into(buffer_list, &mut out, &mut scratch);
    out
}

/// Read mono i16 into a reusable buffer (clears `out` first).
///
/// # Safety
/// `buffer_list` must describe valid Core Audio buffers; each non-null `mData`
/// pointer must remain valid for the duration of this call.
pub unsafe fn read_input_mono_i16_into(
    buffer_list: &AudioBufferList,
    out: &mut Vec<i16>,
    scratch: &mut MonoReadScratch,
) {
    out.clear();
    let num_buffers = buffer_list.mNumberBuffers as usize;
    if num_buffers == 0 {
        return;
    }

    if num_buffers == 1 {
        let buf = &buffer_list.mBuffers[0];
        if buf.mData.is_null() || buf.mDataByteSize == 0 {
            return;
        }
        let channels = buf.mNumberChannels.max(1) as usize;
        let float_count = buf.mDataByteSize as usize / mem::size_of::<f32>();
        let data = std::slice::from_raw_parts(buf.mData as *const f32, float_count);
        float_buffer_to_mono_i16_into(data, channels, out);
        return;
    }

    scratch.collect_non_interleaved(buffer_list);
    if scratch.channel_views.is_empty() {
        return;
    }
    float_channel_buffers_to_mono_i16_into(&scratch.channel_views, out, &mut scratch.frame_scratch);
}

/// Read mono f32 into a reusable buffer (clears `out` first).
///
/// # Safety
/// `buffer_list` must describe valid Core Audio buffers; each non-null `mData`
/// pointer must remain valid for the duration of this call.
pub unsafe fn read_input_mono_f32_into(
    buffer_list: &AudioBufferList,
    out: &mut Vec<f32>,
    scratch: &mut MonoReadScratch,
) {
    out.clear();
    let num_buffers = buffer_list.mNumberBuffers as usize;
    if num_buffers == 0 {
        return;
    }

    if num_buffers == 1 {
        let buf = &buffer_list.mBuffers[0];
        if buf.mData.is_null() || buf.mDataByteSize == 0 {
            return;
        }
        let channels = buf.mNumberChannels.max(1) as usize;
        let float_count = buf.mDataByteSize as usize / mem::size_of::<f32>();
        let data = std::slice::from_raw_parts(buf.mData as *const f32, float_count);
        float_buffer_to_mono_f32_into(data, channels, out);
        return;
    }

    scratch.collect_non_interleaved(buffer_list);
    if scratch.channel_views.is_empty() {
        return;
    }
    float_channel_buffers_to_mono_f32_into(&scratch.channel_views, out, &mut scratch.frame_scratch);
}

/// Frames of mono audio needed to fill this output buffer list.
///
/// Interleaved (`mNumberBuffers == 1`): byte size covers all channels.
/// Non-interleaved (one buffer per channel): each buffer is one channel of N frames.
pub fn output_frames_needed(buffer_list: &AudioBufferList) -> usize {
    let num_buffers = buffer_list.mNumberBuffers as usize;
    if num_buffers == 0 {
        return 0;
    }
    let buf = &buffer_list.mBuffers[0];
    if buf.mDataByteSize == 0 {
        return 0;
    }
    let float_count = buf.mDataByteSize as usize / mem::size_of::<f32>();
    if num_buffers == 1 {
        let channels = buf.mNumberChannels.max(1) as usize;
        float_count / channels.max(1)
    } else {
        float_count
    }
}

/// Write mono i16 samples as float32 into an output buffer list (no intermediate `Vec`).
///
/// # Safety
/// `buffer_list` must describe valid writable Core Audio buffers; each non-null
/// `mData` pointer must remain valid for the duration of this call.
pub unsafe fn write_output_mono_i16(buffer_list: &mut AudioBufferList, samples: &[i16]) {
    let num_buffers = buffer_list.mNumberBuffers as usize;
    if num_buffers == 0 {
        return;
    }

    if num_buffers == 1 {
        let buf = &mut buffer_list.mBuffers[0];
        if buf.mData.is_null() {
            return;
        }
        let channels = buf.mNumberChannels.max(1) as usize;
        let max_floats = buf.mDataByteSize as usize / mem::size_of::<f32>();
        let frames = samples.len().min(max_floats / channels.max(1));
        let dest = std::slice::from_raw_parts_mut(buf.mData as *mut f32, frames * channels.max(1));
        write_mono_i16_as_float_interleaved(&samples[..frames], channels, dest);
        buf.mDataByteSize = (frames * channels * mem::size_of::<f32>()) as u32;
        return;
    }

    let frames = samples.len();
    for ch in 0..num_buffers {
        let buf = &mut buffer_list.mBuffers[ch];
        if buf.mData.is_null() {
            continue;
        }
        let max_frames = buf.mDataByteSize as usize / mem::size_of::<f32>();
        let n = frames.min(max_frames);
        let dst = std::slice::from_raw_parts_mut(buf.mData as *mut f32, n);
        for (frame, slot) in dst.iter_mut().enumerate() {
            *slot = samples[frame] as f32 / i16::MAX as f32;
        }
        buf.mDataByteSize = (n * mem::size_of::<f32>()) as u32;
    }
}

/// Write mono f32 samples into an output buffer list (no format conversion).
///
/// # Safety
/// `buffer_list` must describe valid writable Core Audio buffers; each non-null
/// `mData` pointer must remain valid for the duration of this call.
pub unsafe fn write_output_mono_f32(buffer_list: &mut AudioBufferList, samples: &[f32]) {
    let num_buffers = buffer_list.mNumberBuffers as usize;
    if num_buffers == 0 {
        return;
    }

    if num_buffers == 1 {
        let buf = &mut buffer_list.mBuffers[0];
        if buf.mData.is_null() {
            return;
        }
        let channels = buf.mNumberChannels.max(1) as usize;
        let max_floats = buf.mDataByteSize as usize / mem::size_of::<f32>();
        let frames = samples.len().min(max_floats / channels.max(1));
        let dest = std::slice::from_raw_parts_mut(buf.mData as *mut f32, frames * channels.max(1));
        write_mono_f32_as_float_interleaved(&samples[..frames], channels, dest);
        buf.mDataByteSize = (frames * channels * mem::size_of::<f32>()) as u32;
        return;
    }

    let frames = samples.len();
    for ch in 0..num_buffers {
        let buf = &mut buffer_list.mBuffers[ch];
        if buf.mData.is_null() {
            continue;
        }
        let max_frames = buf.mDataByteSize as usize / mem::size_of::<f32>();
        let n = frames.min(max_frames);
        let dst = std::slice::from_raw_parts_mut(buf.mData as *mut f32, n);
        dst[..n].copy_from_slice(&samples[..n]);
        buf.mDataByteSize = (n * mem::size_of::<f32>()) as u32;
    }
}

/// Zero-fill an output buffer list in place (no allocation).
///
/// # Safety
/// `buffer_list` must describe valid writable Core Audio buffers; each non-null
/// `mData` pointer must remain valid for the duration of this call.
pub unsafe fn write_silence_output(buffer_list: &mut AudioBufferList) {
    let num_buffers = buffer_list.mNumberBuffers as usize;
    for i in 0..num_buffers {
        let buf = &mut buffer_list.mBuffers[i];
        if buf.mData.is_null() || buf.mDataByteSize == 0 {
            continue;
        }
        let byte_len = buf.mDataByteSize as usize;
        std::ptr::write_bytes(buf.mData as *mut u8, 0, byte_len);
    }
}
