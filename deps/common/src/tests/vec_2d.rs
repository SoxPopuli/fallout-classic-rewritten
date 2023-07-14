use crate::Vec2d;

#[test]
fn insert_test() {
    let mut v = Vec2d::new(10, 10);

    v.insert(0, 0, 1).unwrap();
    v.insert(1, 1, 2).unwrap();

    assert_eq!(v.insert(100, 100, 0), None);
}

#[test]
fn get_test() {
    let data = [
        00, 01, 02, 03,
        04, 05, 06, 07,
        08, 09, 10, 11,
        12, 13, 14, 15,
    ];

    let v = Vec2d::from_slice(4, 4, &data);

    assert_eq!(v.get(0, 0).unwrap(), &0);
    assert_eq!(v.get(1, 1).unwrap(), &5);
    assert_eq!(v.get(2, 2).unwrap(), &10);
    assert_eq!(v.get(3, 3).unwrap(), &15);

    assert_eq!(v.get(2, 1).unwrap(), &6);
    assert_eq!(v.get(1, 2).unwrap(), &9);
    assert_eq!(v.get(3, 2).unwrap(), &11);
    assert_eq!(v.get(0, 3).unwrap(), &12);

    assert_eq!(v.get(10, 10), None);
}
