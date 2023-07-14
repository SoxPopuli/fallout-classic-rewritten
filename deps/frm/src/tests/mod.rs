use crate::{ 
    Frame,
    FrmFile,
    PixelShift 
};

#[test]
fn pixel_shift_test() {
    fn shift(index: usize, frame: &Frame) -> Option<usize> {
        FrmFile::apply_pixel_shift(index, frame)
    }

    let mut frame = Frame {
        width: 400,
        height: 600,
        size: 400 * 600,
        shift: PixelShift { x: 1, y: 1 },
        color_index: Vec::default(),
    };
    assert_eq!(shift(500, &frame), Some(901));

    frame.shift.y = 0;
    assert_eq!(shift(1, &frame), Some(2));

    frame.shift = PixelShift { x: -1, y: -1 };
    assert_eq!(shift(500, &frame), Some(99));

    frame.shift = PixelShift { x: -1000, y: 2 };
    assert_eq!(shift(500, &frame), None);
}
