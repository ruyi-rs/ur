use ruyi_iou::IoUring;

#[test]
fn build_io_uring() {
    let uring = IoUring::entries(1).try_build().unwrap();
    println!("{:?}", uring);
}