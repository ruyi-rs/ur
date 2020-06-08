use ruyi_iou::{Op, Uring};

#[test]
fn uring_probe() {
    let uring = Uring::entries(4).try_build().unwrap();
    let probe = uring.probe().unwrap();

    assert!(Op::Nop.is_supported(&probe))
}
