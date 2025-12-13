use bytes::{Buf, BytesMut};
use water_buffer::WaterBuffer;

unsafe fn dos(){

    let layout = std::alloc::Layout::array::<u8>(10).unwrap();
    let p =unsafe{ std::alloc::alloc(layout)};
    for i in 0..10 {
        *p.add(i) = i as u8;
    }
    for i in 0..10 {
        let po = p.add(i);
        println!("{:?}",* po);
    }

}
fn main(){
    // unsafe {dos();}

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
