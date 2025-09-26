use super::*;

#[test]
fn x() {
    let input = include_bytes!("../../../../data/iplogo.mve");

    fn dp(x: f64, places: i32) -> f64 {
        let power = 10.0_f64.powi(places);
        let rounded = (x * power).round();

        rounded / power
    }

    let (video, audio) = State::new().mve(input).unwrap();
    assert_eq!(video.frames.len(), 225);
    assert_eq!(dp(video.frame_rate, 2), 14.99);
    assert_eq!(video.width, 432);
    assert_eq!(video.height, 320);

    assert_eq!(audio.samples.len(), 1324800);
    assert_eq!(audio.format, AudioFormat::I16);
    assert_eq!(audio.sample_rate, 22050);
    assert_eq!(audio.channels, 2);
}
