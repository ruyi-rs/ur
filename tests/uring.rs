use ruyi_ur::{op, Uring};

#[test]
fn uring_probe() {
    let uring = Uring::entries(4).try_build().unwrap();
    let probe = uring.probe().unwrap();

    assert!(probe.support::<op::Nop>());
}
