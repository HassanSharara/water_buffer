use water_buffer::WaterBuffer;

fn main(){

    let buf = WaterBuffer::with_capacity(200);
    let b2 = unsafe {buf.unsafe_clone()};
    println!("{:?} {:?}",buf,b2);
}