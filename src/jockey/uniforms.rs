use lazy_static::lazy_static;
use std::ffi::CString;

lazy_static! {
    // slerpy's golf coding stuff
    pub static ref R_NAME: CString = CString::new("R").unwrap();
    pub static ref K_NAME: CString = CString::new("K").unwrap();

    // miscellaneous
    pub static ref RESOLUTION_NAME: CString = CString::new("resolution").unwrap();
    pub static ref PASS_INDEX_NAME: CString = CString::new("pass_index").unwrap();
    pub static ref OUT_COLOR_NAME: CString = CString::new("out_color").unwrap();
    pub static ref POSITION_NAME: CString = CString::new("position").unwrap();
    pub static ref VERTEX_COUNT_NAME: CString = CString::new("vertex_count").unwrap();
    pub static ref NOISE_NAME: CString = CString::new("noise").unwrap();

    // time tracking
    pub static ref TIME_NAME: CString = CString::new("time").unwrap();
    pub static ref TIME_DELTA_NAME: CString = CString::new("time_delta").unwrap();
    pub static ref FRAME_COUNT_NAME: CString = CString::new("frame_count").unwrap();

    // direct user input
    pub static ref BEAT_NAME: CString = CString::new("beat").unwrap();
    pub static ref SLIDERS_NAME: CString = CString::new("sliders").unwrap();
    pub static ref BUTTONS_NAME: CString = CString::new("buttons").unwrap();

    // volume input
    pub static ref VOLUME_NAME: CString = CString::new("volume").unwrap();
    pub static ref VOLUME_INTEGRATED_NAME: CString = CString::new("volume_integrated").unwrap();

    // audio textures
    pub static ref SAMPLES_NAME: CString = CString::new("samples").unwrap();
    pub static ref SPECTRUM_NAME: CString = CString::new("spectrum").unwrap();
    pub static ref SPECTRUM_RAW_NAME: CString = CString::new("spectrum_raw").unwrap();
    pub static ref SPECTRUM_SMOOTH_NAME: CString = CString::new("spectrum_smooth").unwrap();
    pub static ref SPECTRUM_INTEGRATED_NAME: CString = CString::new("spectrum_integrated").unwrap();
    pub static ref SPECTRUM_SMOOTH_INTEGRATED_NAME: CString = CString::new("spectrum_smooth_integrated").unwrap();

    // bass
    pub static ref BASS_NAME: CString = CString::new("bass").unwrap();
    pub static ref BASS_SMOOTH_NAME: CString = CString::new("bass_smooth").unwrap();
    pub static ref BASS_INTEGRATED_NAME: CString = CString::new("bass_integrated").unwrap();
    pub static ref BASS_SMOOTH_INTEGRATED_NAME: CString = CString::new("bass_smooth_integrated").unwrap();

    // mid
    pub static ref MID_NAME: CString = CString::new("mid").unwrap();
    pub static ref MID_SMOOTH_NAME: CString = CString::new("mid_smooth").unwrap();
    pub static ref MID_INTEGRATED_NAME: CString = CString::new("mid_integrated").unwrap();
    pub static ref MID_SMOOTH_INTEGRATED_NAME: CString = CString::new("mid_smooth_integrated").unwrap();

    // high
    pub static ref HIGH_NAME: CString = CString::new("high").unwrap();
    pub static ref HIGH_SMOOTH_NAME: CString = CString::new("high_smooth").unwrap();
    pub static ref HIGH_INTEGRATED_NAME: CString = CString::new("high_integrated").unwrap();
    pub static ref HIGH_SMOOTH_INTEGRATED_NAME: CString = CString::new("high_smooth_integrated").unwrap();
}
