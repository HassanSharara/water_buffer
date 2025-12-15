use bytes::Buf;
// use bytes::{Buf, BytesMut};
use water_buffer::WaterBuffer;


fn main(){

    let mut b = bytes::BytesMut::with_capacity(10);
    b.extend_from_slice(b"12");
    b.advance(1);
    println!("called {}",b.capacity());
    return;
    // unsafe {dos();}
    // std::alloc::Global;
    let mut b = WaterBuffer::with_capacity(20);
    b.extend_from_slice(b"Hello, world!");
    b.extend_from_slice(b"2222222222222");
    println!("{:?} {}",String::from_utf8_lossy(&b),b.capacity());
    b.advance(13);
    println!("{:?} {}",String::from_utf8_lossy(&b),b.capacity());
    b.extend_from_slice(b"3333333333333");
    println!("{:?} {}",String::from_utf8_lossy(&b),b.capacity());
    b.extend_from_slice(b"4444444444444");
    println!("{:?} {}",String::from_utf8_lossy(&b),b.capacity());
}
