

fn main() {

    let mut buf = water_buffer::WaterBuffer::with_capacity(12);
    buf.extend_from_slice(b"hello world");
    println!("{:?}",String::from_utf8_lossy(&buf));

}