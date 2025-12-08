

fn main() {
    
    let mut buf = water_buffer::WaterBuffer::with_capacity(100);
    buf.extend_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

    for i in buf.iter_mut() {
        println!("{}", i);
    }
}