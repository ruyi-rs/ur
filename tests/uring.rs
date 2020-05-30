use ruyi_iou::Uring;

#[test]
fn build_io_uring() {
    let uring = Uring::entries(4).try_build().unwrap();
    println!("{:?}", uring);
}
