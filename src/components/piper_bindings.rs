use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::os::raw::{c_char, c_float, c_int};
use std::ptr;

// Match the C structs
#[repr(C)]
pub struct PiperSynthesizer {
    _private: [u8; 0], // opaque pointer
}

#[repr(C)]
pub struct PiperAudioChunk {
    pub samples: *const c_float,
    pub num_samples: usize,
    pub sample_rate: c_int,
    pub is_last: bool,
    _padding: [u8; 3],
}

#[repr(C)]
pub struct PiperSynthesizeOptions {
    pub speaker_id: c_int,
    pub length_scale: c_float,
    pub noise_scale: c_float,
    pub noise_w_scale: c_float,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PiperStatus {
    Ok = 0,
    Done = 1,
    Error = 2,
}

// Function bindings
unsafe extern "C" {
    pub unsafe fn piper_create(
        model_path: *const c_char,
        config_path: *const c_char,
        espeak_ng_data_path: *const c_char,
    ) -> *mut PiperSynthesizer;

    pub unsafe fn piper_free(synth: *mut PiperSynthesizer);

    pub unsafe fn piper_default_synthesize_options(
        synth: *mut PiperSynthesizer,
    ) -> PiperSynthesizeOptions;

    pub unsafe fn piper_synthesize_start(
        synth: *mut PiperSynthesizer,
        text: *const c_char,
        options: *const PiperSynthesizeOptions,
    );

    pub unsafe fn piper_synthesize_next(
        synth: *mut PiperSynthesizer,
        chunk: *mut PiperAudioChunk,
    ) -> c_int;
}

pub struct Piper {
    synth: *mut PiperSynthesizer,
}

impl Piper {
    pub fn new(model_path: &str, config_path: &str, espeak_data: &str) -> Self {
        unsafe {
            let model_path = CString::new(model_path).unwrap();
            let config_path = CString::new(config_path).unwrap();
            let espeak_data = CString::new(espeak_data).unwrap();

            let synth = piper_create(
                model_path.as_ptr(),
                config_path.as_ptr(),
                espeak_data.as_ptr(),
            );

            if synth.is_null() {
                panic!("Failed to create Piper synthesizer");
            }

            Self { synth }
        }
    }

    pub fn generate(&self, message: &str, output_path: &str) {
        unsafe {
            let mut options = piper_default_synthesize_options(self.synth);
            options.length_scale = 1.0;

            let text = CString::new(message).unwrap();
            piper_synthesize_start(self.synth, text.as_ptr(), &options);

            let mut audio_stream = File::create(output_path).unwrap();
            let mut chunk = PiperAudioChunk {
                samples: ptr::null(),
                num_samples: 0,
                sample_rate: 0,   // default, will be set by the library
                is_last: false,   // default, will be set by the library
                _padding: [0; 3], // to match alignment
            };

            loop {
                let status = piper_synthesize_next(self.synth, &mut chunk);

                if !chunk.samples.is_null() && chunk.num_samples > 0 {
                    let samples = std::slice::from_raw_parts(chunk.samples, chunk.num_samples);
                    let bytes = std::slice::from_raw_parts(
                        samples.as_ptr() as *const u8,
                        std::mem::size_of_val(samples),
                    );
                    audio_stream.write_all(bytes).unwrap();
                }

                if status == PiperStatus::Done as c_int || chunk.is_last {
                    break;
                }
            }
        }
    }
}

impl Drop for Piper {
    fn drop(&mut self) {
        unsafe { piper_free(self.synth) };
    }
}
